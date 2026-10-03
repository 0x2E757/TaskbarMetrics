//! `TaskbarMetrics.exe --watch`: stays in the tray and keeps the program running.
//! A new Explorer gets the tiles again, a collector that ended is started again,
//! and the icon's menu closes everything.
mod restart;
mod supervision;
mod tray;
use super::{
    abi::*, executables::Executables, launcher::Explorer, process_history::ProcessHistory,
    shutdown::Shutdown, temperature::SensorCollector,
};
use std::{
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Instant,
};
use supervision::{Action, Observation, Supervision};
pub(crate) use tray::CLASS;

/// DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP: no console, and Ctrl+C in the
/// terminal that started the launcher does not reach it.
const DETACHED: u32 = 0x8 | 0x200;
/// CREATE_NO_WINDOW, for the launcher the watcher starts.
const NO_WINDOW: u32 = 0x0800_0000;

pub(crate) struct Watcher {
    executable: PathBuf,
    supervision: Supervision,
    /// The launcher started for an Explorer, while it runs.
    attaching: Option<(u32, Child)>,
    /// "Restart all services", while it runs.
    restarting: Option<std::thread::JoinHandle<()>>,
}
impl Watcher {
    /// Starts the watcher of the copy at `executable`, unless one already runs.
    pub fn spawn(executable: &Path) {
        if Self::running() {
            return;
        }
        let mut command = wide(&format!(
            "\"{}\" {}",
            executable.display(),
            super::launcher::WATCH
        ));
        let startup = StartupInfo {
            size: std::mem::size_of::<StartupInfo>() as u32,
            ..StartupInfo::default()
        };
        let mut started = ProcessInformation::default();
        // No inherited handles: std's spawn would hand down the pipe a script or the
        // portable copy reads the launcher's output from, and they would wait for
        // the watcher to end.
        let created = unsafe {
            CreateProcessW(
                std::ptr::null(),
                command.as_mut_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                DETACHED,
                std::ptr::null_mut(),
                std::ptr::null(),
                &startup,
                &mut started,
            )
        };
        if created == 0 {
            log(&format!("Watcher not started: 0x{:08X}", last_error()));
            return;
        }
        drop((Handle::new(started.process), Handle::new(started.thread)));
    }
    fn running() -> bool {
        Handle::new(unsafe { OpenMutexW(0x100000, 0, Self::mutex().as_ptr()) }).is_ok()
    }
    fn mutex() -> Vec<u16> {
        wide("Local\\TaskbarMetrics.Watch")
    }
    /// Runs until the icon's menu or `--stop` closes it; the launcher started it
    /// right after attaching to the current Explorer.
    pub fn run(executable: &Path) -> Result<i32> {
        let _single =
            Handle::new(unsafe { CreateMutexW(std::ptr::null_mut(), 0, Self::mutex().as_ptr()) })?;
        if unsafe { GetLastError() } == 183 {
            return Ok(2);
        }
        // Per-monitor DPI awareness for a sharp icon.
        unsafe { SetProcessDpiAwarenessContext(-4isize as Raw) };
        let watcher = Self {
            executable: executable.to_owned(),
            supervision: Supervision::new(Explorer::find().ok().map(|explorer| explorer.pid)),
            attaching: None,
            restarting: None,
        };
        let language = super::metrics_window::Language::load();
        tray::TrayIcon::run(
            Box::new(watcher),
            "Taskbar Metrics",
            tray::MenuLabels {
                restart: language.text("Restart all services").into(),
                close: language.text("Close Taskbar Metrics").into(),
            },
            2000,
        )?;
        Ok(0)
    }
    fn observe(&mut self) -> Observation {
        if let Some((pid, child)) = &mut self.attaching {
            if let Ok(Some(status)) = child.try_wait() {
                // 2: already attached; 3: attached, nothing visible yet (no room).
                let success = matches!(status.code(), Some(0 | 2 | 3));
                self.supervision.attached(*pid, success);
                self.attaching = None;
            }
        }
        if self
            .restarting
            .as_ref()
            .is_some_and(|thread| thread.is_finished())
        {
            self.restarting = None;
        }
        let explorer = Explorer::find().ok().map(|explorer| explorer.pid);
        let Some(pid) = explorer else {
            return Observation::default();
        };
        Observation {
            explorer,
            stopped: signaled("stop", pid),
            recorder: ProcessHistory::running(pid),
            sensor: SensorCollector::running(pid),
            busy: self.attaching.is_some() || self.restarting.is_some(),
        }
    }
    fn act(&mut self, action: Action) {
        let started = match action {
            Action::Attach(pid) => {
                log(&format!("Watcher: attaching to Explorer {pid}"));
                Command::new(&self.executable)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .creation_flags(NO_WINDOW)
                    .spawn()
                    .map(|child| self.attaching = Some((pid, child)))
                    .map_err(|_| E_FAIL)
            }
            Action::Recorder(pid) => {
                log("Watcher: starting the history recorder again");
                start_recorder(&self.executable, pid)
            }
            Action::Sensor(pid) => {
                log("Watcher: starting the CPU temperature collector again");
                SensorCollector::launch(&self.executable.with_file_name(Executables::SENSORS), pid)
            }
        };
        if let Err(hr) = started {
            log(&format!("Watcher: start failed: 0x{hr:08X}"));
            self.supervision.failed(action);
        }
    }
}
impl tray::TrayEvents for Watcher {
    fn tick(&mut self) {
        let seen = self.observe();
        for action in self.supervision.next(seen, Instant::now()) {
            self.act(action);
        }
    }
    /// Runs on a thread of its own: waiting for the collectors to exit takes
    /// seconds, and the icon keeps answering meanwhile.
    fn restart(&mut self) {
        if self.restarting.is_some() {
            return;
        }
        let Ok(explorer) = Explorer::find() else {
            return;
        };
        let restart = restart::ServicesRestart {
            executable: self.executable.clone(),
            pid: explorer.pid,
        };
        self.restarting = Some(std::thread::spawn(move || restart.run()));
    }
    fn close_all(&mut self) {
        Shutdown::stop_tiles();
        Shutdown::close(Shutdown::WINDOW);
    }
}

/// Starts the history recorder of the copy at `executable` for the Explorer `pid`,
/// elevated when the configuration monitors processes.
fn start_recorder(executable: &Path, pid: u32) -> Result<()> {
    let settings = super::data_directory::DataDirectory::file("taskbar-metrics.conf")
        .ok()
        .and_then(|path| crate::config::Settings::load(&path).ok())
        .unwrap_or_default();
    ProcessHistory::launch(
        &executable.with_file_name(Executables::HISTORY),
        pid,
        settings.process_monitoring,
    )
}

/// STARTUPINFOW with nothing but its size set.
#[repr(C)]
#[derive(Default)]
struct StartupInfo {
    size: u32,
    reserved: u32,
    rest: [u64; 12],
}
/// PROCESS_INFORMATION.
#[repr(C)]
struct ProcessInformation {
    process: Raw,
    thread: Raw,
    pid: u32,
    tid: u32,
}
impl Default for ProcessInformation {
    fn default() -> Self {
        Self {
            process: std::ptr::null_mut(),
            thread: std::ptr::null_mut(),
            pid: 0,
            tid: 0,
        }
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn OpenMutexW(access: u32, inherit: i32, name: *const u16) -> Raw;
    #[allow(clippy::too_many_arguments)]
    fn CreateProcessW(
        application: *const u16,
        command: *mut u16,
        process_attributes: Raw,
        thread_attributes: Raw,
        inherit: i32,
        flags: u32,
        environment: Raw,
        directory: *const u16,
        startup: *const StartupInfo,
        information: *mut ProcessInformation,
    ) -> i32;
}
#[link(name = "user32")]
extern "system" {
    fn SetProcessDpiAwarenessContext(context: Raw) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layouts_match_the_windows_sdk() {
        assert_eq!(std::mem::size_of::<StartupInfo>(), 104);
        assert_eq!(std::mem::size_of::<ProcessInformation>(), 24);
    }
}
