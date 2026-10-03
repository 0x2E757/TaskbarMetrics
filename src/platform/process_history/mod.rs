//! Optional out-of-process recorder. UI polling never waits for this collector.
mod collector;
mod debug_privilege;
mod ipc;
pub(crate) mod memory;
mod shared_pages;
mod snapshot;
pub(crate) mod store;
mod trace;
pub(crate) mod wire;
use super::abi::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub struct ProcessHistory;
impl ProcessHistory {
    /// Starts the recorder for this Explorer. Process monitoring needs elevation for
    /// ETW (`elevated`, one UAC prompt); totals alone run without it. An elevated
    /// start replaces a running unelevated recorder.
    pub(crate) fn launch(executable: &std::path::Path, pid: u32, elevated: bool) -> Result<()> {
        if Handle::new(unsafe { OpenMutexW(0x100000, 0, event_name("history", pid).as_ptr()) })
            .is_ok()
        {
            let basic = Handle::new(unsafe {
                OpenEventW(0x100000, 0, event_name("history-upgrade", pid).as_ptr())
            })
            .is_ok();
            if !(elevated && basic) {
                return Ok(());
            }
        }
        if !executable.is_file() {
            return Err(E_FAIL);
        }
        // The installed copy elevates through its task, without a prompt.
        if elevated
            && super::scheduled_task::CollectorTask::HISTORY
                .run(executable, pid)
                .is_ok()
        {
            return Ok(());
        }
        use std::os::windows::ffi::OsStrExt;
        let path: Vec<_> = executable
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // Kernel disk/network tracing needs elevation. This executable exposes
        // only a read-only history pipe; it cannot execute client commands.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                wide(if elevated { "runas" } else { "open" }).as_ptr(),
                path.as_ptr(),
                wide(&format!("--serve {pid}")).as_ptr(),
                std::ptr::null(),
                0,
            )
        };
        if result as isize <= 32 {
            Err(E_FAIL)
        } else {
            Ok(())
        }
    }
    /// The recorder for this Explorer holds its mutex.
    pub(crate) fn running(pid: u32) -> bool {
        Handle::new(unsafe { OpenMutexW(0x100000, 0, event_name("history", pid).as_ptr()) }).is_ok()
    }
    pub fn run() -> std::result::Result<(), String> {
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.len() == 3 && args[0] == "--dump" {
            return ipc::HistoryServer::dump(
                args[1].parse().map_err(|_| "Invalid PID")?,
                std::path::Path::new(&args[2]),
            )
            .map_err(|e| e.to_string());
        }
        let probe = args.len() == 2 && args[0] == "--probe";
        let pid: u32 = if args.len() == 2 && args[0] == "--serve" {
            args[1].parse().map_err(|_| "Invalid PID")?
        } else if probe {
            unsafe { GetCurrentProcessId() }
        } else {
            return Err("Usage: TaskbarMetrics.History.exe --serve <Explorer PID> | --probe <seconds> | --dump <Explorer PID> <file.csv>".into());
        };
        let limit = if probe {
            Some(Duration::from_secs(
                args[1]
                    .parse::<u64>()
                    .map_err(|_| "Invalid duration")?
                    .clamp(1, 600),
            ))
        } else {
            None
        };
        let elevated = Elevation::current();
        let Some(_singleton) = Self::singleton(pid, elevated)? else {
            return Ok(());
        };
        // An unelevated recorder yields to an elevated one started for process monitoring.
        let upgrade = if elevated {
            None
        } else {
            Some(
                Handle::new(unsafe {
                    CreateEventW(
                        std::ptr::null_mut(),
                        1,
                        0,
                        event_name("history-upgrade", pid).as_ptr(),
                    )
                })
                .map_err(|e| format!("Event {e:X}"))?,
            )
        };
        let parent = Handle::new(unsafe { OpenProcess(0x100000, 0, pid) })
            .map_err(|e| format!("Parent {e:X}"))?;
        let config = crate::platform::data_directory::DataDirectory::file("taskbar-metrics.conf")
            .map_err(|e| e.to_string())?;
        let watch = config.with_file_name("taskbar-metrics.watch");
        // For the whole run: an elevated recorder then lists the working sets of
        // services and virtual machines too; an unelevated token cannot enable it.
        debug_privilege::DebugPrivilege::enable();
        let history = Arc::new(Mutex::new(store::History::default()));
        ipc::HistoryServer::spawn(pid, history.clone());
        ipc::HistoryServer::stream(pid, history.clone());
        let start = Instant::now();
        let cpu_start = Self::cpu_ticks();
        let mut next = start;
        let mut collector: Option<collector::Collector> = None;
        let mut devices = RecordedDevices::default();
        let mut pending = BTreeMap::new();
        let mut retry = start;
        let mut durations = Vec::new();
        let mut settings =
            crate::config::Settings::load(&config).map_err(|e| format!("Configuration: {e}"))?;
        loop {
            if limit.is_some_and(|l| start.elapsed() >= l) {
                break;
            }
            if !probe && unsafe { WaitForSingleObject(parent.0, 0) } == 0 {
                break;
            }
            if upgrade
                .as_ref()
                .is_some_and(|event| unsafe { WaitForSingleObject(event.0, 0) } == 0)
            {
                break;
            }
            // Stopped tiles, or "Restart all services", which starts a new recorder.
            if !probe && (signaled("stop", pid) || signaled("restart", pid)) {
                break;
            }
            match crate::config::Settings::load(&config) {
                Ok(updated) => settings = updated,
                Err(error) => {
                    history.lock().unwrap().status =
                        format!("Configuration: {error}; keeping previous settings")
                }
            }
            let bucket = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_millis() as u64
                / 500;
            {
                let mut h = history.lock().unwrap_or_else(|e| e.into_inner());
                h.prune(bucket);
                h.enabled = settings.process_monitoring;
            }
            let watched = crate::platform::devices::WatchList::read(&watch);
            let wanted = devices.update(&settings, &watched);
            if collector.as_ref().is_none_or(|c| c.devices != wanted) && Instant::now() >= retry {
                match collector::Collector::new(wanted.to_vec()) {
                    Ok(c) => collector = Some(c),
                    Err(e) => {
                        retry = Instant::now() + Duration::from_secs(30);
                        history.lock().unwrap().status = format!("Sampling unavailable 0x{e:08X}")
                    }
                }
            }
            if let Some(c) = &mut collector {
                c.monitor_processes(settings.process_monitoring);
                // An open window refreshes its watch list every 2 s, also while minimized.
                c.shared = !watched.is_empty();
                match c.sample(bucket) {
                    Ok((frame, events)) => {
                        if probe {
                            durations.push(frame.sample_ms);
                        }
                        let mut h = history.lock().unwrap_or_else(|e| e.into_inner());
                        h.status = c.status(settings.process_monitoring);
                        h.push(frame);
                        for (key, bytes) in events {
                            pending
                                .entry(key)
                                .or_insert_with(store::IoBytes::default)
                                .merge(&bytes);
                        }
                        let ready: Vec<_> = pending
                            .keys()
                            .copied()
                            .filter(|(b, _)| *b <= bucket)
                            .collect();
                        for (b, p) in ready {
                            if let Some(bytes) = pending.remove(&(b, p)) {
                                h.add_io(b, p, &bytes);
                            }
                        }
                    }
                    Err(e) => {
                        history.lock().unwrap().status = format!("Snapshot failed 0x{e:08X}");
                        collector = None;
                    }
                }
            }
            next += Duration::from_millis(500);
            let now = Instant::now();
            if next <= now {
                next = now + Duration::from_millis(500);
            }
            std::thread::sleep(next.saturating_duration_since(now));
        }
        if probe {
            durations.sort_by(f64::total_cmp);
            let h = history.lock().unwrap();
            let cpu_ms_per_second = Self::cpu_ticks().saturating_sub(cpu_start) as f64
                / 10000.0
                / start.elapsed().as_secs_f64();
            let text = format!(
                "frames={} processes={} status={} sample_wall_ms_p50={:.3} p95={:.3} collector_cpu_ms_per_second={cpu_ms_per_second:.3}\n",
                h.frames.len(),
                h.frames.back().map_or(0, |f| f.processes.len()),
                h.status,
                durations.get(durations.len() / 2).copied().unwrap_or(0.0),
                durations
                    .get(durations.len() * 95 / 100)
                    .copied()
                    .unwrap_or(0.0)
            );
            print!("{text}");
            std::fs::write(
                std::env::current_exe().unwrap().with_extension("probe.txt"),
                text,
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    /// Holds the per-Explorer recorder mutex. An elevated recorder asks a running
    /// unelevated one to yield and waits up to 3 s for it; otherwise `None`.
    fn singleton(pid: u32, elevated: bool) -> std::result::Result<Option<Handle>, String> {
        for _ in 0..30 {
            let handle = Handle::new(unsafe {
                CreateMutexW(std::ptr::null_mut(), 0, event_name("history", pid).as_ptr())
            })
            .map_err(|e| format!("Mutex {e:X}"))?;
            if unsafe { GetLastError() } != 183 {
                return Ok(Some(handle));
            }
            drop(handle);
            let Ok(upgrade) = Handle::new(unsafe {
                OpenEventW(2, 0, event_name("history-upgrade", pid).as_ptr())
            }) else {
                return Ok(None);
            };
            if !elevated {
                return Ok(None);
            }
            unsafe { SetEvent(upgrade.0) };
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(None)
    }
    fn cpu_ticks() -> u64 {
        let (mut created, mut exited, mut kernel, mut user) = (0u64, 0u64, 0u64, 0u64);
        unsafe {
            GetProcessTimes(
                -1isize as Raw,
                &mut created,
                &mut exited,
                &mut kernel,
                &mut user,
            );
        }
        kernel + user
    }
}
/// Whether this process runs elevated (`TokenElevation`).
pub(crate) struct Elevation;
impl Elevation {
    pub(crate) fn current() -> bool {
        let mut token = std::ptr::null_mut();
        // SAFETY: the pseudo handle needs no closing; the token handle is owned below.
        if unsafe { OpenProcessToken(-1isize as Raw, 8, &mut token) } == 0 {
            return false;
        }
        let Ok(token) = Handle::new(token) else {
            return false;
        };
        let (mut elevated, mut size) = (0u32, 0u32);
        // SAFETY: TokenElevation (20) writes one DWORD.
        (unsafe {
            GetTokenInformation(
                token.0,
                20,
                (&mut elevated as *mut u32).cast(),
                4,
                &mut size,
            )
        } != 0)
            && elevated != 0
    }
}

/// Devices the recorder keeps history for: taskbar tiles, `history=` and, while
/// fresh, the devices an open window shows. Bare kinds resolve to their main
/// device; resolution is repeated at most every 30 s or when the inputs change.
#[derive(Default)]
struct RecordedDevices {
    inputs: Vec<crate::platform::devices::DeviceId>,
    resolved: Vec<crate::platform::devices::DeviceId>,
    next: Option<Instant>,
}
impl RecordedDevices {
    fn update(
        &mut self,
        settings: &crate::config::Settings,
        watched: &[crate::platform::devices::DeviceId],
    ) -> &[crate::platform::devices::DeviceId] {
        use crate::platform::devices::{DeviceCatalog, DeviceId};
        let mut inputs: Vec<DeviceId> = settings
            .metrics
            .iter()
            .chain(&settings.history)
            .filter_map(|id| DeviceId::parse(id))
            .chain(watched.iter().cloned())
            .collect();
        inputs.sort();
        inputs.dedup();
        if inputs != self.inputs || self.next.is_none_or(|next| Instant::now() >= next) {
            self.next = Some(Instant::now() + Duration::from_secs(30));
            let mut resolved: Vec<DeviceId> = inputs.iter().map(DeviceCatalog::canonical).collect();
            resolved.sort();
            resolved.dedup();
            self.inputs = inputs;
            self.resolved = resolved;
        }
        &self.resolved
    }
}
#[link(name = "advapi32")]
extern "system" {
    fn OpenProcessToken(process: Raw, access: u32, token: *mut Raw) -> i32;
    fn GetTokenInformation(
        token: Raw,
        class: u32,
        information: *mut core::ffi::c_void,
        size: u32,
        returned: *mut u32,
    ) -> i32;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetProcessTimes(
        process: Raw,
        created: *mut u64,
        exited: *mut u64,
        kernel: *mut u64,
        user: *mut u64,
    ) -> i32;
    fn OpenMutexW(access: u32, inherit: i32, name: *const u16) -> Raw;
}
#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        window: Raw,
        verb: *const u16,
        file: *const u16,
        args: *const u16,
        directory: *const u16,
        show: i32,
    ) -> Raw;
}
