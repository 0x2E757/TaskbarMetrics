//! Where the program keeps what it writes: the configuration, the window's choices,
//! pins and the diagnostic log. The binaries may sit in a folder the user cannot
//! write, such as Program Files, so the data lives in the user's profile, the same
//! for an installed and a portable copy.

use std::path::PathBuf;

pub struct DataDirectory;

impl DataDirectory {
    /// `%LOCALAPPDATA%\Taskbar Metrics`, created when missing.
    pub fn path() -> std::io::Result<PathBuf> {
        let base = std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| std::io::Error::other("LOCALAPPDATA is not set"))?;
        let path = PathBuf::from(base).join("Taskbar Metrics");
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// The data file `name`, such as `taskbar-metrics.conf`.
    pub fn file(name: &str) -> std::io::Result<PathBuf> {
        Ok(Self::path()?.join(name))
    }
}
