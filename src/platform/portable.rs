//! The portable program: one executable that carries the others. The DLL has to
//! be a file for Explorer to load it, so the files go to the `bin` folder of the
//! data directory, where they stay put for the Run value; a newer portable
//! executable replaces them, freeing those in use first.
use super::{data_directory::DataDirectory, executables::Executables, unload::Unload};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

/// A carried file: its path under `bin` and its contents.
pub type Payload = [(&'static str, &'static [u8])];

pub struct PortableCopy {
    directory: PathBuf,
}
impl PortableCopy {
    /// Puts the files in place, then runs the launcher with the same arguments
    /// and returns its exit code.
    pub fn run(payload: &Payload) -> i32 {
        let copy = match DataDirectory::path() {
            Ok(data) => Self {
                directory: data.join("bin"),
            },
            Err(error) => {
                eprintln!("Data folder: {error}");
                return 1;
            }
        };
        let restarted = match copy.update(payload) {
            Ok(restarted) => restarted,
            Err(error) => {
                eprintln!("Unpacking failed: {error}");
                return 1;
            }
        };
        copy.launch(restarted)
    }
    /// Writes the files that differ; true when Explorer had to restart to free them.
    fn update(&self, payload: &Payload) -> std::io::Result<bool> {
        let stale: Vec<_> = payload
            .iter()
            .filter(|(name, bytes)| {
                std::fs::read(self.directory.join(name)).ok().as_deref() != Some(*bytes)
            })
            .collect();
        if stale.is_empty() {
            return Ok(false);
        }
        if stale.iter().all(|file| self.write(file).is_ok()) {
            return Ok(false);
        }
        Unload::run(&self.directory)
            .map_err(|hr| std::io::Error::other(format!("files in use: 0x{hr:08X}")))?;
        for file in stale {
            self.write(file)?;
        }
        Ok(true)
    }
    fn write(&self, (name, bytes): &(&str, &[u8])) -> std::io::Result<()> {
        let path = self.directory.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)
    }
    /// Right after a restart the taskbar may not be ready: a failed attach is
    /// retried for half a minute.
    fn launch(&self, restarted: bool) -> i32 {
        let launcher = self.directory.join(Executables::LAUNCHER);
        let arguments: Vec<_> = std::env::args_os().skip(1).collect();
        let attempts = if restarted && arguments.is_empty() {
            15
        } else {
            1
        };
        let mut code = 1;
        for attempt in 1..=attempts {
            code = Self::status(&launcher, &arguments);
            if code != 1 || attempt == attempts {
                break;
            }
            std::thread::sleep(Duration::from_secs(2));
        }
        code
    }
    fn status(launcher: &Path, arguments: &[std::ffi::OsString]) -> i32 {
        match Command::new(launcher).args(arguments).status() {
            Ok(status) => status.code().unwrap_or(1),
            Err(error) => {
                eprintln!("{}: {error}", launcher.display());
                1
            }
        }
    }
}
