//! "Restart all services": the history recorder and the CPU temperature collector
//! start again. The DLL in Explorer stays as it is; only when it is missing or
//! stopped does the launcher attach it, which starts the recorder too.

use crate::platform::{
    abi::*, executables::Executables, process_history::ProcessHistory, temperature::SensorCollector,
};

use std::{
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub(super) struct ServicesRestart {
    pub executable: PathBuf,
    /// The Explorer whose services start again.
    pub pid: u32,
}

impl ServicesRestart {
    /// The collectors may take this long to see the signal and exit.
    const RECORDER_EXIT: Duration = Duration::from_secs(5);
    /// Explorer lets go of the collector's shared memory once its reading is 5 s old.
    const SENSOR_EXIT: Duration = Duration::from_secs(10);

    pub fn run(self) {
        log(&format!(
            "Watcher: restarting the services of Explorer {}",
            self.pid
        ));
        let sensor = SensorCollector::running(self.pid);
        let Ok(event) = Handle::new(unsafe {
            CreateEventW(
                std::ptr::null_mut(),
                1,
                0,
                event_name("restart", self.pid).as_ptr(),
            )
        }) else {
            return;
        };
        unsafe { SetEvent(event.0) };
        Self::wait(Self::RECORDER_EXIT, || !ProcessHistory::running(self.pid));
        if sensor {
            Self::wait(Self::SENSOR_EXIT, || SensorCollector::released(self.pid));
        }
        unsafe { ResetEvent(event.0) };
        drop(event);
        if self.tiles_running() {
            if let Err(hr) = super::start_recorder(&self.executable, self.pid) {
                log(&format!("Watcher: recorder not started: 0x{hr:08X}"));
            }
        } else {
            self.attach();
        }
        if sensor {
            if let Err(hr) = SensorCollector::launch(
                &self.executable.with_file_name(Executables::SENSORS),
                self.pid,
            ) {
                log(&format!("Watcher: CPU collector not started: 0x{hr:08X}"));
            }
        }
    }

    /// The DLL is loaded in this Explorer and its tiles are not stopped.
    fn tiles_running(&self) -> bool {
        Handle::new(unsafe { OpenEventW(0x100000, 0, event_name("stop", self.pid).as_ptr()) })
            .is_ok()
            && !signaled("stop", self.pid)
    }

    /// The launcher attaches the DLL, or wakes a stopped one, and starts the recorder.
    fn attach(&self) {
        let status = Command::new(&self.executable)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(super::NO_WINDOW)
            .status();
        if let Err(error) = status {
            log(&format!("Watcher: launcher not started: {error}"));
        }
    }

    fn wait(limit: Duration, done: impl Fn() -> bool) {
        let deadline = Instant::now() + limit;
        while !done() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
