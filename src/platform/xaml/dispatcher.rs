use super::*;
use std::{
    ptr,
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Mutex,
    },
};
#[repr(C)]
struct HandlerVtbl {
    base: UnknownVtbl,
    invoke: unsafe extern "system" fn(Raw) -> Hr,
}
#[repr(C)]
struct Handler {
    vtable: &'static HandlerVtbl,
    refs: AtomicU32,
    action: Mutex<Option<UiAction>>,
}
type UiAction = Box<dyn FnOnce() -> Result<()> + Send>;
unsafe extern "system" fn handler_query(raw: Raw, iid: *const Guid, out: *mut Raw) -> Hr {
    if iid.is_null() || out.is_null() {
        return E_POINTER;
    }
    *out = ptr::null_mut();
    if *iid != UNKNOWN && *iid != HANDLER && *iid != AGILE_OBJECT {
        return E_NOINTERFACE;
    }
    *out = raw;
    handler_add(raw);
    0
}
unsafe extern "system" fn handler_add(raw: Raw) -> u32 {
    (*(raw as *const Handler))
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "system" fn handler_release(raw: Raw) -> u32 {
    let count = (*(raw as *const Handler))
        .refs
        .fetch_sub(1, Ordering::AcqRel)
        - 1;
    if count == 0 {
        drop(Box::from_raw(raw as *mut Handler));
    }
    count
}
unsafe extern "system" fn handler_invoke(raw: Raw) -> Hr {
    let hr = boundary(|| {
        let action = (*(raw as *const Handler))
            .action
            .lock()
            .map_err(|_| E_FAIL)?
            .take();
        if let Some(action) = action {
            action()?;
        }
        Ok(())
    });
    if hr < 0 {
        log(&format!("UI operation failed: 0x{:08X}", hr as u32));
    }
    0 // Do not propagate failed UI updates into Explorer's dispatcher.
}
static HANDLER_VTABLE: HandlerVtbl = HandlerVtbl {
    base: UnknownVtbl {
        query: handler_query,
        add_ref: handler_add,
        release: handler_release,
    },
    invoke: handler_invoke,
};
impl Handler {
    fn delegate(action: impl FnOnce() -> Result<()> + Send + 'static) -> Com {
        let object = Box::new(Handler {
            vtable: &HANDLER_VTABLE,
            refs: AtomicU32::new(1),
            action: Mutex::new(Some(Box::new(action))),
        });
        unsafe { Com::owned(Box::into_raw(object).cast()).expect("Box is non-null") }
    }
}
pub struct Target {
    pub key: u64,
    queue: Agile,
    pub active: AtomicBool,
    pending: AtomicBool,
}
impl Target {
    pub fn current(key: u64) -> Result<Arc<Self>> {
        let queue = factory("Windows.System.DispatcherQueue", &QUEUE_STATICS)?.object(6)?;
        Ok(Arc::new(Self {
            key,
            queue: Agile::new(&queue, &QUEUE)?,
            active: AtomicBool::new(true),
            pending: AtomicBool::new(false),
        }))
    }
    pub fn enqueue(&self, action: impl FnOnce() -> Result<()> + Send + 'static) -> Result<()> {
        let queue = self.queue.resolve(&QUEUE)?;
        let handler = Handler::delegate(action);
        let mut accepted = 0u8;
        unsafe {
            let call: unsafe extern "system" fn(Raw, Raw, *mut u8) -> Hr = queue.slot(7);
            check(call(queue.raw(), handler.raw(), &mut accepted))?;
        }
        if accepted == 0 {
            Err(E_FAIL)
        } else {
            Ok(())
        }
    }
    pub fn update(self: &Arc<Self>, readings: Vec<crate::metrics::MetricReading>) {
        if !self.active.load(Ordering::Acquire) || self.pending.swap(true, Ordering::AcqRel) {
            return;
        }
        let target = self.clone();
        if let Err(hr) = self.enqueue(move || {
            target.pending.store(false, Ordering::Release);
            if !target.active.load(Ordering::Acquire) {
                return Ok(());
            }
            // Clone out of TLS before COM calls; XAML can reenter diagnostics callbacks.
            UiTarget::update(&target, &readings)?;
            Ok(())
        }) {
            self.pending.store(false, Ordering::Release);
            log(&format!("Enqueue failed: 0x{:08X}", hr as u32));
        }
    }
    pub fn remove(self: &Arc<Self>) {
        self.active.store(false, Ordering::Release);
        let key = self.key;
        if self.enqueue(move || UiTarget::remove(key)).is_err() {
            log("Dispatcher closed before removal");
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_dispatcher_marshals_manual_delegate_to_its_own_thread() {
        #[repr(C)]
        struct Options {
            size: u32,
            thread: i32,
            apartment: i32,
        }
        #[link(name = "CoreMessaging")]
        extern "system" {
            fn CreateDispatcherQueueController(options: Options, out: *mut Raw) -> Hr;
        }
        let _apartment = Apartment::mta().unwrap();
        let mut raw = ptr::null_mut();
        unsafe {
            check(CreateDispatcherQueueController(
                Options {
                    size: 12,
                    thread: 1,
                    apartment: 2,
                },
                &mut raw,
            ))
            .unwrap();
        }
        let controller = unsafe { Com::owned(raw).unwrap() };
        let queue = controller.object(6).unwrap();
        let bootstrap = Target {
            key: 0,
            queue: Agile::new(&queue, &QUEUE).unwrap(),
            active: AtomicBool::new(true),
            pending: AtomicBool::new(false),
        };
        let (send, receive) = std::sync::mpsc::channel();
        bootstrap
            .enqueue(move || {
                // Same operation as in a XAML tree callback: acquire this thread's queue.
                let target = Target::current(1)?;
                send.send((target, std::thread::current().id()))
                    .map_err(|_| E_FAIL)
            })
            .unwrap();
        let (target, ui_thread) = receive
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert_ne!(ui_thread, std::thread::current().id());
        let (send, receive) = std::sync::mpsc::channel();
        target
            .enqueue(move || send.send(std::thread::current().id()).map_err(|_| E_FAIL))
            .unwrap();
        assert_eq!(
            receive
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap(),
            ui_thread
        );
        let shutdown = controller.object(7).unwrap();
        let info = shutdown
            .query(&Guid::from_u128(0x00000036_0000_0000_c000_000000000046))
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let mut status = 0;
            unsafe {
                let get: unsafe extern "system" fn(Raw, *mut i32) -> Hr = info.slot(7);
                check(get(info.raw(), &mut status)).unwrap();
            }
            if status != 0 {
                assert_eq!(status, 1);
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Dispatcher shutdown timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    #[test]
    fn delegate_identity_lifetime_and_single_invocation() {
        let count = Arc::new(AtomicU32::new(0));
        let captured = count.clone();
        let object = Handler::delegate(move || {
            captured.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
        let queried = object.query(&HANDLER).unwrap();
        assert_eq!(object.raw(), queried.raw());
        assert_eq!(object.query(&UNKNOWN).unwrap().raw(), object.raw());
        assert!(object.query(&PANEL).is_err());
        unsafe {
            let invoke: unsafe extern "system" fn(Raw) -> Hr = queried.slot(3);
            assert_eq!(invoke(queried.raw()), 0);
            assert_eq!(invoke(queried.raw()), 0);
        }
        assert_eq!(count.load(Ordering::Relaxed), 1);
        drop(object);
        drop(queried);
        assert_eq!(Arc::strong_count(&count), 1);
    }
}
