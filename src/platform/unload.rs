//! Frees the program's files so an installer or a portable update can replace or
//! remove them: Explorer keeps the DLL loaded until it exits, the window and the
//! collectors hold their executables.
use super::{
    abi::*, executables::Executables, launcher::Explorer, process_history::Elevation,
    shutdown::Shutdown,
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(crate) struct Unload;
impl Unload {
    /// Closes the tray icon and the window, ends Explorer, which Windows starts again by itself
    /// (`AutoRestartShell`), and waits until the files in `directory` are free; the
    /// collectors exit with the Explorer they were started for.
    pub fn run(directory: &Path) -> Result<()> {
        // The tray icon first, or it would attach to the new Explorer again.
        Shutdown::close(super::watch::CLASS);
        Shutdown::close(Shutdown::WINDOW);
        if let Ok(explorer) = Explorer::find() {
            Self::restart_explorer(explorer.pid)?;
        }
        Self::wait_free(directory)
    }
    /// Ends the Explorer `pid` and waits for the one Windows starts in its place.
    pub fn restart_explorer(pid: u32) -> Result<()> {
        unsafe {
            // PROCESS_TERMINATE | SYNCHRONIZE, for this one process.
            let process = Handle::new(OpenProcess(0x100001, 0, pid))?;
            // Windows restarts a shell that ended this way; exit code 1 counts as
            // "Exit Explorer" from the taskbar menu, which it does not restart.
            if TerminateProcess(process.0, u32::MAX) == 0 {
                return Err(last_error());
            }
            WaitForSingleObject(process.0, 10000);
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if Explorer::find().is_ok_and(|explorer| explorer.pid != pid) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        // Started from an elevated process, Explorer would run elevated too.
        if Elevation::current() {
            eprintln!("Explorer did not restart; start it from the Task Manager.");
            return Err(E_FAIL);
        }
        std::process::Command::new(
            Path::new(&std::env::var_os("WINDIR").ok_or(E_FAIL)?).join("explorer.exe"),
        )
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|_| E_FAIL)
    }
    /// Every file but the running launcher opens for writing within 10 s.
    fn wait_free(directory: &Path) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        let files: Vec<_> = Executables::PACKAGED
            .iter()
            .map(|(_, name)| directory.join(name))
            .filter(|path| path.file_name() != Some(Executables::LAUNCHER.as_ref()))
            .filter(|path| path.is_file())
            .collect();
        loop {
            let busy = files
                .iter()
                .any(|path| std::fs::OpenOptions::new().write(true).open(path).is_err());
            if !busy {
                return Ok(());
            }
            if Instant::now() >= deadline {
                eprintln!("The program's files are still in use.");
                return Err(E_FAIL);
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn TerminateProcess(process: Raw, code: u32) -> i32;
}
