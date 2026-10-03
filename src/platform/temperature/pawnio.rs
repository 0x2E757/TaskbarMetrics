use crate::platform::abi::*;
use std::{ffi::CStr, path::Path, ptr};

type Open = unsafe extern "system" fn(*mut Raw) -> Hr;
type Load = unsafe extern "system" fn(Raw, *const u8, usize) -> Hr;
type Execute =
    unsafe extern "system" fn(Raw, *const u8, *const u64, usize, *mut u64, usize, *mut usize) -> Hr;
type Close = unsafe extern "system" fn(Raw) -> Hr;

/// Owns the installed library and executor. Used exclusively by the helper process.
pub(super) struct PawnModule {
    library: Library,
    handle: Raw,
    execute: Execute,
    close: Close,
}
struct Library(Raw);
impl Library {
    fn open() -> Result<Self> {
        // The official installer has a fixed location. Never search PATH/current directory.
        let program_files = std::env::var_os("ProgramW6432").ok_or(E_FAIL)?;
        let path = Path::new(&program_files).join("PawnIO/PawnIOLib.dll");
        use std::os::windows::ffi::OsStrExt;
        let path: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let raw = unsafe { LoadLibraryExW(path.as_ptr(), ptr::null_mut(), 0x1100) };
        if raw.is_null() {
            Err(last_error())
        } else {
            Ok(Self(raw))
        }
    }
    fn symbol(&self, name: &CStr) -> Result<Raw> {
        let raw = unsafe { GetProcAddress(self.0, name.as_ptr().cast()) };
        if raw.is_null() {
            Err(last_error())
        } else {
            Ok(raw)
        }
    }
}
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.0);
        }
    }
}
impl PawnModule {
    pub fn load(path: &Path) -> Result<Self> {
        let blob = std::fs::read(path).map_err(|_| E_FAIL)?;
        let library = Library::open()?;
        unsafe {
            let open: Open = std::mem::transmute(library.symbol(c"pawnio_open")?);
            let load: Load = std::mem::transmute(library.symbol(c"pawnio_load")?);
            let execute: Execute = std::mem::transmute(library.symbol(c"pawnio_execute")?);
            let close: Close = std::mem::transmute(library.symbol(c"pawnio_close")?);
            let mut handle = ptr::null_mut();
            check(open(&mut handle))?;
            let module = Self {
                library,
                handle,
                execute,
                close,
            };
            check(load(handle, blob.as_ptr(), blob.len()))?;
            Ok(module)
        }
    }
    pub fn read(&self, command: &CStr, input: &[u64]) -> Result<u64> {
        let mut output = 0;
        let mut count = 0;
        unsafe {
            check((self.execute)(
                self.handle,
                command.as_ptr().cast(),
                input.as_ptr(),
                input.len(),
                &mut output,
                1,
                &mut count,
            ))?;
        }
        if count == 1 {
            Ok(output)
        } else {
            Err(E_FAIL)
        }
    }
}
impl Drop for PawnModule {
    fn drop(&mut self) {
        // Executor must close before the DLL is released by Library::drop.
        let _keep_alive = &self.library;
        unsafe {
            (self.close)(self.handle);
        }
    }
}
