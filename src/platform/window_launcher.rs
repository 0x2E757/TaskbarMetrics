use super::abi::*;
pub(crate) struct WindowLauncher;
impl WindowLauncher {
    pub fn open(resource: &str) -> Result<()> {
        std::process::Command::new(
            Self::directory()?.join(super::executables::Executables::WINDOW),
        )
        .args(["--monitor", resource])
        .spawn()
        .map_err(|_| E_FAIL)?;
        Ok(())
    }
    pub fn directory() -> Result<std::path::PathBuf> {
        // Resolve beside our DLL, not beside Explorer.exe.
        let mut module = std::ptr::null_mut();
        unsafe {
            if GetModuleHandleExW(4, Self::open as *const () as *const u16, &mut module) == 0 {
                return Err(last_error());
            }
        }
        let mut path = vec![0u16; 32768];
        let length = unsafe { GetModuleFileNameW(module, path.as_mut_ptr(), path.len() as u32) };
        unsafe {
            FreeLibrary(module);
        }
        if length == 0 || length as usize >= path.len() {
            return Err(E_FAIL);
        }
        use std::os::windows::ffi::OsStringExt;
        let executable =
            std::path::PathBuf::from(std::ffi::OsString::from_wide(&path[..length as usize]));
        executable.parent().map(|p| p.to_owned()).ok_or(E_FAIL)
    }
}
#[link(name = "kernel32")]
extern "system" {
    fn FreeLibrary(module: Raw) -> i32;
}
