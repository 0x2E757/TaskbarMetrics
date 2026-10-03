//! Closing the program: the tiles and the collectors follow the stop event of their
//! Explorer, the window and the tray icon close as windows do.

use super::{abi::*, launcher::Explorer};
use std::ptr;

pub(crate) struct Shutdown;

impl Shutdown {
    /// Window class of the metrics window.
    pub const WINDOW: &'static str = "TaskbarMetrics.Monitor";

    /// Everything: the tiles, the collectors, the window and the tray icon.
    pub fn all() {
        Self::stop_tiles();
        Self::close(Self::WINDOW);
        Self::close(super::watch::CLASS);
    }

    /// Removes the tiles of the current Explorer; the history recorder and the CPU
    /// temperature collector exit on the same event. False when none are attached.
    pub fn stop_tiles() -> bool {
        let Ok(explorer) = Explorer::find() else {
            return false;
        };
        unsafe {
            Handle::new(OpenEventW(2, 0, event_name("stop", explorer.pid).as_ptr()))
                .is_ok_and(|event| SetEvent(event.0) != 0)
        }
    }

    /// Asks the window of `class` to close and waits up to 5 s for its process.
    pub fn close(class: &str) {
        unsafe {
            let window = FindWindowW(wide(class).as_ptr(), ptr::null());
            if window.is_null() {
                return;
            }
            let mut pid = 0;
            GetWindowThreadProcessId(window, &mut pid);
            PostMessageW(window, 0x0010, 0, 0);
            if let Ok(process) = Handle::new(OpenProcess(0x100000, 0, pid)) {
                WaitForSingleObject(process.0, 5000);
            }
        }
    }
}

#[link(name = "user32")]
extern "system" {
    fn PostMessageW(window: Raw, message: u32, wparam: usize, lparam: isize) -> i32;
}
