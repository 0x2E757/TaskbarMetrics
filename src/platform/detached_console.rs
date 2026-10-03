//! The collectors run in the background. Windows 11 24H2 gives them no console
//! when nothing started them from one (`consoleAllocationPolicy` in their manifest);
//! older builds open a console window for them, which they leave at once.
pub struct DetachedConsole;
impl DetachedConsole {
    /// Leaves a console that exists only for this process; a terminal the
    /// process was started from is shared with the shell and stays.
    pub fn release() {
        let mut processes = [0u32; 2];
        // SAFETY: the buffer holds the two ids the call may write.
        unsafe {
            if GetConsoleProcessList(processes.as_mut_ptr(), 2) == 1 {
                FreeConsole();
            }
        }
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn GetConsoleProcessList(processes: *mut u32, count: u32) -> u32;
    fn FreeConsole() -> i32;
}
