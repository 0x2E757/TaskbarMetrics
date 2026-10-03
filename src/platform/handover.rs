//! Copies of the program in different folders, such as the installed one, the
//! portable one and a build, share the named objects of their Explorer, so one runs
//! at a time. The copy started last takes over: Explorer cannot unload the DLL of
//! another copy, so it restarts without it.
use super::{
    abi::*, executables::Executables, launcher::Explorer, shutdown::Shutdown,
    temperature::SensorCollector, unload::Unload,
};
use std::path::Path;

/// What the copy that gave way ran, for the one that takes over.
pub(crate) struct Handover {
    /// The CPU temperature collector ran.
    pub sensor: bool,
}
impl Handover {
    /// Closes another copy whose DLL the current Explorer holds and restarts
    /// Explorer; `Some` when it did, and the new taskbar may still be starting.
    pub fn take(dll: &Path) -> Option<Self> {
        let explorer = Explorer::find().ok()?;
        let loaded = explorer.module(Executables::HOST)?;
        if Self::same(&loaded, dll) {
            return None;
        }
        log(&format!(
            "Explorer {} holds the copy in {}; taking over",
            explorer.pid,
            loaded.display()
        ));
        let sensor = SensorCollector::running(explorer.pid);
        Shutdown::all();
        match Unload::restart_explorer(explorer.pid) {
            Ok(()) => Some(Self { sensor }),
            Err(hr) => {
                log(&format!("Explorer not restarted: 0x{hr:08X}"));
                None
            }
        }
    }
    /// The same file; a missing one, such as a removed copy, is another.
    fn same(left: &Path, right: &Path) -> bool {
        match (left.canonicalize(), right.canonicalize()) {
            (Ok(left), Ok(right)) => {
                left.to_string_lossy().to_lowercase() == right.to_string_lossy().to_lowercase()
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_compare_as_the_files_they_name() {
        let executable = std::env::current_exe().unwrap();
        let upper = executable.to_string_lossy().to_uppercase();
        assert!(Handover::same(&executable, Path::new(&upper)));
        assert!(!Handover::same(
            &executable,
            &executable.with_file_name("missing.dll")
        ));
    }
}
