//! `--help` of the programs `TaskbarMetrics.exe` starts. People and agents who come
//! across one of them are sent to `TaskbarMetrics.exe`, which runs the program and
//! reads what it recorded.

/// The arguments that ask any of the programs for help.
pub const FLAGS: [&str; 3] = ["--help", "-h", "/?"];

/// The help of one of the programs `TaskbarMetrics.exe` starts.
pub struct SubprogramHelp {
    /// What the program is, and its arguments meant for people.
    pub about: &'static str,
}

impl SubprogramHelp {
    const POINTER: &'static str =
        "Scripts and agents: use TaskbarMetrics.exe in this folder instead.\n\
        TaskbarMetrics.exe --dump prints the history of the last 5 minutes the program recorded:\n\
        the load of the computer and of its processes. TaskbarMetrics.exe --help explains it.";

    pub fn requested() -> bool {
        std::env::args()
            .skip(1)
            .any(|arg| FLAGS.contains(&arg.as_str()))
    }

    /// Prints the help; a program without a console prints into the terminal it was
    /// started from.
    pub fn print(&self) {
        // SAFETY: plain calls without pointers.
        unsafe {
            if GetStdHandle(STD_OUTPUT_HANDLE).is_null() {
                AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
        println!("{}\n{}", self.about, Self::POINTER);
    }
}

const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
const ATTACH_PARENT_PROCESS: u32 = u32::MAX;

#[link(name = "kernel32")]
extern "system" {
    fn GetStdHandle(kind: u32) -> *mut std::ffi::c_void;
    fn AttachConsole(process: u32) -> i32;
}
