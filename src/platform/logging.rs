use std::{
    fs::File,
    io::Write,
    path::Path,
    sync::{Mutex, OnceLock},
};

/// Best-effort local startup diagnostics; failures never escape into Explorer.
pub(crate) struct DiagnosticLog;
static FILE: OnceLock<Mutex<File>> = OnceLock::new();
impl DiagnosticLog {
    pub fn initialize(path: &Path) {
        if let Ok(file) = File::create(path) {
            let _ = FILE.set(Mutex::new(file));
        }
    }
    pub fn write(message: &str) {
        if let Some(file) = FILE.get() {
            if let Ok(mut file) = file.lock() {
                let _ = writeln!(file, "{message}");
            }
        }
    }
}
