use super::*;
use std::path::PathBuf;

/// `process_monitoring` and the device keys in the configuration.
pub(super) struct MonitoringConfig {
    file: crate::config::ConfigFile,
}

impl MonitoringConfig {
    pub fn locate() -> Result<Self> {
        let directory =
            crate::platform::data_directory::DataDirectory::path().map_err(|_| E_FAIL)?;
        Ok(Self {
            file: crate::config::ConfigFile::locate(&directory),
        })
    }

    pub fn appearance_path(&self) -> PathBuf {
        self.file
            .path()
            .with_file_name("taskbar-metrics.appearance")
    }

    /// Devices an open window shows, read by the recorder.
    pub fn watch_path(&self) -> PathBuf {
        self.file.path().with_file_name("taskbar-metrics.watch")
    }

    pub fn enabled(&self) -> Result<bool> {
        Ok(crate::config::Settings::load(self.file.path())
            .map_err(|_| E_FAIL)?
            .process_monitoring)
    }

    pub fn settings(&self) -> Result<crate::config::Settings> {
        crate::config::Settings::load(self.file.path()).map_err(|_| E_FAIL)
    }

    /// Taskbar tiles and background history; the taskbar and the recorder pick
    /// them up within a second.
    pub fn store_devices(&self, metrics: &[String], history: &[String]) -> std::io::Result<()> {
        self.file.write(&[
            ("metrics", metrics.join(",")),
            ("history", history.join(",")),
        ])
    }

    /// Rewrites the key and restarts the collector when enabling.
    pub fn store(&self, enabled: bool) -> std::io::Result<()> {
        self.file
            .write(&[("process_monitoring", enabled.to_string())])?;
        if enabled {
            use std::os::windows::process::CommandExt;
            let launcher = std::env::current_exe()?
                .with_file_name(crate::platform::executables::Executables::LAUNCHER);
            if launcher.is_file() {
                let _ = std::process::Command::new(launcher)
                    .creation_flags(0x08000000)
                    .spawn();
            }
        }
        Ok(())
    }
}

pub(super) struct MonitoringSetting {
    toggle: Com,
    status: Com,
    config: MonitoringConfig,
    enabled: bool,
    verifying: bool,
}

impl MonitoringSetting {
    pub fn appearance_path(&self) -> PathBuf {
        self.config.appearance_path()
    }

    pub fn new(root: &Com) -> Result<Self> {
        let config = MonitoringConfig::locate()?;
        let enabled = config.enabled()?;
        let toggle = playground::Playground::find(root, "ProcessMonitoring")?
            .query(&Guid::from_u128(0x331d8f00_c5f9_46a5_b6c8_ede539304567))?;
        unsafe {
            let set: unsafe extern "system" fn(Raw, u8) -> Hr = toggle.slot(7);
            check(set(toggle.raw(), enabled as u8))?;
        }
        Ok(Self {
            toggle,
            status: playground::Playground::find(root, "MonitoringStatus")?
                .query(&Guid::from_u128(0xae2d9271_3b4a_45fc_8468_f7949548f4d5))?,
            config,
            enabled,
            verifying: std::env::args().any(|a| {
                matches!(
                    a.as_str(),
                    "--verify-settings" | "--verify-monitor" | "--demo"
                )
            }),
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        let mut enabled = 0u8;
        unsafe {
            let get: unsafe extern "system" fn(Raw, *mut u8) -> Hr = self.toggle.slot(6);
            check(get(self.toggle.raw(), &mut enabled))?;
        }
        let enabled = enabled != 0;
        if enabled == self.enabled {
            return Ok(());
        }
        if !self.verifying {
            if let Err(error) = self.config.store(enabled) {
                self.status.set_string(
                    27,
                    &format!(
                        "{}: {error}",
                        monitor::settings_language().text("Could not save setting")
                    ),
                )?;
                unsafe {
                    let set: unsafe extern "system" fn(Raw, u8) -> Hr = self.toggle.slot(7);
                    check(set(self.toggle.raw(), self.enabled as u8))?;
                }
                return Ok(());
            }
        }
        self.enabled = enabled;
        self.status.set_string(
            27,
            monitor::settings_language().text(if enabled {
                "Background monitoring enabled. History: 5 minutes at 500 ms intervals."
            } else {
                "Background process monitoring disabled. Taskbar metrics continue to work."
            }),
        )
    }
}
