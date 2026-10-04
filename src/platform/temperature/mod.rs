mod amd;
mod channel;
mod intel;
mod pawnio;
mod provider;
mod sensor;
mod zhaoxin;
use crate::platform::{abi::*, help::SubprogramHelp};
pub(crate) use provider::CpuTemperatureProvider;

/// The only entry point that opens a driver. Runs elevated, outside Explorer.
pub struct SensorCollector;

impl SensorCollector {
    /// The collector for this Explorer is publishing.
    pub(crate) fn running(pid: u32) -> bool {
        channel::TemperatureChannel::open(pid).is_ok_and(|channel| channel.fresh())
    }

    /// The collector's shared memory is gone, so a new collector can create it.
    pub(crate) fn released(pid: u32) -> bool {
        channel::TemperatureChannel::open(pid).is_err()
    }

    pub(crate) fn launch(executable: &std::path::Path, pid: u32) -> Result<()> {
        if Self::running(pid) {
            return Ok(());
        }
        if !executable.is_file() {
            return Err(E_FAIL);
        }
        // The installed copy elevates through its task, without a prompt.
        if crate::platform::scheduled_task::CollectorTask::SENSORS
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
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                wide("runas").as_ptr(),
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

    pub fn run() -> std::result::Result<(), String> {
        if SubprogramHelp::requested() {
            SubprogramHelp {
                about: "TaskbarMetrics.Sensors.exe reads the CPU temperature through the PawnIO driver for\n\
                    the tiles and the history; TaskbarMetrics.exe --sensors starts it.\n\
                    --sample: print three readings (administrator rights).",
            }
            .print();
            return Ok(());
        }
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let args: Vec<_> = std::env::args().skip(1).collect();
        let sample = args == ["--sample"];
        let pid = if args.len() == 2 && args[0] == "--serve" {
            args[1].parse::<u32>().ok()
        } else {
            None
        };
        if !sample && pid.is_none() {
            return Err("Usage: TaskbarMetrics.Sensors.exe --sample | --serve <Explorer PID> (administrator required)".into());
        }
        let sensor = sensor::SensorCatalog::open(executable.parent().ok_or("No executable directory")?)
            .map_err(|hr| format!("CPU sensor initialization failed: 0x{hr:08X}. Administrator rights and official PawnIO are required; check CPU support and the pawnio folder beside the EXE."))?;
        if sample {
            let mut report = String::new();
            for _ in 0..3 {
                let value = sensor
                    .sample()
                    .map_err(|hr| format!("Read failed: 0x{hr:08X}"))?;
                let line = format!("{}: {value:.3} C\n", sensor.name());
                print!("{line}");
                report.push_str(&line);
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            std::fs::write(executable.with_extension("sample.log"), report)
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
        let pid = pid.unwrap();
        let parent = Handle::new(unsafe { OpenProcess(0x100000, 0, pid) })
            .map_err(|hr| format!("Explorer handle: 0x{hr:08X}"))?;
        let mut channel = channel::TemperatureChannel::create(pid).map_err(|hr| {
            format!("Collector channel: 0x{hr:08X}; another collector may already be running")
        })?;
        loop {
            // Respect the application's stop and restart signals without exposing any
            // privileged command interface. The events may appear after startup.
            if signaled("stop", pid) || signaled("restart", pid) {
                return Ok(());
            }
            channel.publish(sensor.sample().ok());
            match unsafe { WaitForSingleObject(parent.0, 500) } {
                WAIT_TIMEOUT => {}
                0 => return Ok(()),
                _ => return Err("Explorer lifetime wait failed".into()),
            }
        }
    }
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        window: Raw,
        verb: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
    ) -> Raw;
}
