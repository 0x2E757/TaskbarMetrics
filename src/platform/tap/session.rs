//! Sampling session and taskbar target lifecycle, composed independently of COM entry points.
use super::*;
pub(super) struct Session {
    pub(super) site: Agile,
    pub(super) diagnostics: Agile,
    pub(super) stop: Handle,
    pub(super) resume: Handle,
    pub(super) idle: Handle,
    pub(super) stopping: AtomicBool,
    pub(super) targets: Mutex<HashMap<u64, Arc<Target>>>,
    pub(super) settings: Mutex<Settings>,
    pub(super) config: std::path::PathBuf,
}
impl Session {
    pub(super) fn settings(&self) -> Result<Settings> {
        Ok(self.settings.lock().map_err(|_| E_FAIL)?.clone())
    }

    pub(super) fn reload(&self) -> Result<()> {
        let settings = Settings::load(&self.config).map_err(|error| {
            log(&error);
            E_FAIL
        })?;
        let _ = Composition::monitor(&settings).map_err(|error| {
            log(&error);
            E_FAIL
        })?;
        *self.settings.lock().map_err(|_| E_FAIL)? = settings;
        Ok(())
    }

    pub(super) fn list(&self) -> Result<Vec<Arc<Target>>> {
        Ok(self
            .targets
            .lock()
            .map_err(|_| E_FAIL)?
            .values()
            .cloned()
            .collect())
    }
    pub(super) fn object(&self, key: u64) -> Result<Com> {
        let diagnostics = self.diagnostics.resolve(&DIAGNOSTICS)?;
        unsafe {
            let call: unsafe extern "system" fn(Raw, u64, *mut Raw) -> Hr = diagnostics.slot(6);
            let mut result = ptr::null_mut();
            check(call(diagnostics.raw(), key, &mut result))?;
            Com::owned(result)
        }
    }
    pub(super) fn change(&self, relation: Relation, element: Element, mutation: i32) -> Result<()> {
        if self.stopping.load(Ordering::Acquire) {
            return Ok(());
        }
        if mutation == 1 {
            xaml::ClockCatalog::remove(element.handle);
            let removed = self
                .targets
                .lock()
                .map_err(|_| E_FAIL)?
                .remove(&element.handle);
            if let Some(target) = removed {
                target.remove();
            }
            return Ok(());
        }
        if mutation != 0 {
            return Ok(());
        }
        let object = self.object(element.handle)?;
        if let Ok(framework) = object.query(&xaml::FRAMEWORK) {
            if framework.string(33)? == "NotificationCenterButton" {
                xaml::ClockCatalog::register(element.handle, &object)?;
            }
        }
        let Ok(panel) = object.query(&xaml::PANEL) else {
            return Ok(());
        };
        if panel.query(&xaml::FRAMEWORK)?.string(33)? != "RootGrid" {
            return Ok(());
        }
        if relation.parent == 0
            || self.object(relation.parent)?.string(4)? != "Taskbar.TaskbarFrame"
        {
            return Ok(());
        }
        // Diagnostic handles can be reused after removal. Queue jobs use a distinct generation.
        let target = Target::current(NEXT_TARGET.fetch_add(1, Ordering::Relaxed))?;
        {
            let mut targets = self.targets.lock().map_err(|_| E_FAIL)?;
            if self.stopping.load(Ordering::Acquire) {
                return Ok(());
            }
            if targets.contains_key(&element.handle) {
                return Ok(());
            }
            targets.insert(element.handle, target.clone());
        }
        if let Err(hr) = target.insert(panel, self.settings()?) {
            self.targets
                .lock()
                .map_err(|_| E_FAIL)?
                .remove(&element.handle);
            return Err(hr);
        }
        Ok(())
    }
    pub(super) fn run(&self, callback: &Com) -> Result<()> {
        self.reload()?;
        let settings = self.settings()?;
        self.stopping.store(false, Ordering::Release);
        let diagnostics = self.diagnostics.resolve(&DIAGNOSTICS)?;
        let service = diagnostics.query(&TREE_SERVICE)?;
        unsafe {
            let advise: unsafe extern "system" fn(Raw, Raw) -> Hr = service.slot(3);
            check(advise(service.raw(), callback.raw()))?;
        }
        log("Visual tree subscription active");
        // A guard guarantees unsubscription even if a Rust panic is caught outside run().
        struct Subscription<'a> {
            service: &'a Com,
            callback: &'a Com,
            session: &'a Session,
        }
        impl Drop for Subscription<'_> {
            fn drop(&mut self) {
                self.session.stopping.store(true, Ordering::Release);
                unsafe {
                    let unadvise: unsafe extern "system" fn(Raw, Raw) -> Hr = self.service.slot(4);
                    let hr = unadvise(self.service.raw(), self.callback.raw());
                    if hr < 0 {
                        log(&format!("Unadvise failed: 0x{:08X}", hr as u32));
                    }
                }
            }
        }
        let _subscription = Subscription {
            service: &service,
            callback,
            session: self,
        };
        let mut monitor = Composition::monitor(&settings).map_err(|error| {
            log(&error);
            E_FAIL
        })?;
        monitor.tick(self);
        let mut settings = settings;
        let mut checked = std::time::Instant::now();
        let mut next = std::time::Instant::now() + settings.interval;
        loop {
            let wait = next.saturating_duration_since(std::time::Instant::now());
            match unsafe {
                WaitForSingleObject(self.stop.0, wait.as_millis().min(u32::MAX as u128) as u32)
            } {
                WAIT_TIMEOUT => {}
                0 => break,
                _ => return Err(last_error()),
            }
            // Tiles added or removed in the main window: sample the new set.
            if checked.elapsed().as_secs() >= 1 {
                checked = std::time::Instant::now();
                if let Ok(updated) = Settings::load(&self.config) {
                    if updated.metrics != settings.metrics {
                        // A drag only reorders the tiles; the samplers stay warm.
                        let replacement = if updated.same_tiles(&settings) {
                            Ok(None)
                        } else {
                            Composition::monitor(&updated).map(Some)
                        };
                        if let Ok(replacement) = replacement {
                            if let Some(replacement) = replacement {
                                monitor = replacement;
                            }
                            *self.settings.lock().map_err(|_| E_FAIL)? = updated.clone();
                            settings = updated;
                        }
                    }
                }
            }
            monitor.tick(self);
            next += settings.interval;
            if next <= std::time::Instant::now() {
                next = std::time::Instant::now() + settings.interval;
            }
        }
        Ok(())
    }
    pub(super) fn cleanup(&self) {
        self.stopping.store(true, Ordering::Release);
        let targets = match self.targets.lock() {
            Ok(mut map) => std::mem::take(&mut *map),
            Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
        };
        for target in targets.values() {
            target.remove();
        }
        log("Stopped; DLL stays pinned until Explorer exits");
    }
}
impl MetricSink for Session {
    fn publish(&self, _text: &str) {}
    fn publish_readings(&self, readings: &[crate::metrics::MetricReading], _text: &str) {
        if let Ok(targets) = self.list() {
            for target in targets {
                target.update(readings.to_vec());
            }
        }
    }
}
