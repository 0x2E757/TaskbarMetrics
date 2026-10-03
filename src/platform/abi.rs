use std::ffi::c_void;
pub type Raw = *mut c_void;
pub type Hr = i32;
pub type Result<T> = std::result::Result<T, Hr>;
pub const E_FAIL: Hr = 0x80004005u32 as i32;
pub const E_POINTER: Hr = 0x80004003u32 as i32;
pub const E_NOINTERFACE: Hr = 0x80004002u32 as i32;
pub const E_UNEXPECTED: Hr = 0x8000ffffu32 as i32;
pub const CLASS_E_NOAGGREGATION: Hr = 0x80040110u32 as i32;
pub const CLASS_E_CLASSNOTAVAILABLE: Hr = 0x80040111u32 as i32;
pub const ERROR_NOT_FOUND_HR: Hr = 0x80070490u32 as i32;
pub const WAIT_TIMEOUT: u32 = 258;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Guid {
    pub a: u32,
    pub b: u16,
    pub c: u16,
    pub d: [u8; 8],
}
impl Guid {
    pub const fn from_u128(value: u128) -> Self {
        Self {
            a: (value >> 96) as u32,
            b: (value >> 80) as u16,
            c: (value >> 64) as u16,
            d: (value as u64).to_be_bytes(),
        }
    }
}
pub const CLSID: Guid = Guid::from_u128(0x64b850da_9a93_41a7_a614_8fba1ca8b348);
pub fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
pub fn event_name(kind: &str, pid: u32) -> Vec<u16> {
    wide(&format!("Local\\TaskbarMetrics.{pid}.{kind}"))
}
/// The event `kind` of the Explorer `pid` exists and is set: `stop` while the tiles
/// are stopped, `restart` while the collectors are asked to start again.
pub fn signaled(kind: &str, pid: u32) -> bool {
    // SYNCHRONIZE only: the event is read, never set, here.
    Handle::new(unsafe { OpenEventW(0x100000, 0, event_name(kind, pid).as_ptr()) })
        .is_ok_and(|event| unsafe { WaitForSingleObject(event.0, 0) } == 0)
}
pub fn check(hr: Hr) -> Result<()> {
    if hr < 0 {
        Err(hr)
    } else {
        Ok(())
    }
}
pub fn last_error() -> Hr {
    unsafe { (0x80070000 | (GetLastError() & 0xffff)) as Hr }
}
pub fn log(message: &str) {
    super::logging::DiagnosticLog::write(message);
    unsafe {
        OutputDebugStringW(wide(&format!("[TaskbarMetrics] {message}\n")).as_ptr());
    }
}
pub fn boundary(action: impl FnOnce() -> Result<()>) -> Hr {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)) {
        Ok(Ok(())) => 0,
        Ok(Err(hr)) => hr,
        Err(_) => E_FAIL,
    }
}
pub struct Handle(pub Raw);
impl Handle {
    pub fn new(raw: Raw) -> Result<Self> {
        if raw.is_null() {
            Err(last_error())
        } else {
            Ok(Self(raw))
        }
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
// Kernel handles are process-wide, not COM apartment objects. Lifetime is owned by Handle.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}
/// A DLL from System32, loaded on demand and released on drop.
pub struct SystemLibrary(Raw);
impl SystemLibrary {
    pub fn load(name: &str) -> Result<Self> {
        // LOAD_LIBRARY_SEARCH_SYSTEM32: never a same-named DLL from elsewhere.
        let module = unsafe { LoadLibraryExW(wide(name).as_ptr(), std::ptr::null_mut(), 0x800) };
        if module.is_null() {
            Err(last_error())
        } else {
            Ok(Self(module))
        }
    }
    /// An exported function; valid while the library is held.
    pub fn symbol(&self, name: &std::ffi::CStr) -> Result<Raw> {
        let raw = unsafe { GetProcAddress(self.0, name.as_ptr().cast()) };
        if raw.is_null() {
            Err(last_error())
        } else {
            Ok(raw)
        }
    }
}
impl Drop for SystemLibrary {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.0);
        }
    }
}
// Apartment initialization and uninitialization must happen on the same thread.
pub struct Apartment(std::marker::PhantomData<std::rc::Rc<()>>);
impl Apartment {
    pub fn mta() -> Result<Self> {
        unsafe {
            check(RoInitialize(1))?;
        }
        Ok(Self(std::marker::PhantomData))
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetLastError() -> u32;
    pub fn GetCurrentProcessId() -> u32;
    pub fn CloseHandle(handle: Raw) -> i32;
    pub fn CreateEventW(attributes: Raw, manual: i32, initial: i32, name: *const u16) -> Raw;
    pub fn CreateMutexW(attributes: Raw, owner: i32, name: *const u16) -> Raw;
    pub fn FreeLibrary(module: Raw) -> i32;
    pub fn GetModuleFileNameW(module: Raw, path: *mut u16, size: u32) -> u32;
    pub fn OpenEventW(access: u32, inherit: i32, name: *const u16) -> Raw;
    pub fn SetEvent(handle: Raw) -> i32;
    pub fn ResetEvent(handle: Raw) -> i32;
    pub fn WaitForSingleObject(handle: Raw, milliseconds: u32) -> u32;
    pub fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Raw;
    pub fn QueryFullProcessImageNameW(
        process: Raw,
        flags: u32,
        image: *mut u16,
        size: *mut u32,
    ) -> i32;
    pub fn IsWow64Process2(process: Raw, machine: *mut u16, native: *mut u16) -> i32;
    pub fn LoadLibraryExW(path: *const u16, file: Raw, flags: u32) -> Raw;
    pub fn GetProcAddress(module: Raw, name: *const u8) -> Raw;
    pub fn GetModuleHandleExW(flags: u32, address: *const u16, module: *mut Raw) -> i32;
    pub fn OutputDebugStringW(text: *const u16);
}
#[link(name = "user32")]
extern "system" {
    pub fn FindWindowW(class: *const u16, name: *const u16) -> Raw;
    pub fn GetWindowThreadProcessId(window: Raw, pid: *mut u32) -> u32;
}
#[link(name = "runtimeobject")]
extern "system" {
    pub fn RoInitialize(kind: u32) -> Hr;
    pub fn RoUninitialize();
    pub fn RoActivateInstance(name: Raw, result: *mut Raw) -> Hr;
    pub fn RoGetActivationFactory(name: Raw, iid: *const Guid, result: *mut Raw) -> Hr;
    pub fn WindowsCreateString(text: *const u16, length: u32, result: *mut Raw) -> Hr;
    pub fn WindowsDeleteString(text: Raw) -> Hr;
    pub fn WindowsGetStringRawBuffer(text: Raw, length: *mut u32) -> *const u16;
}
#[link(name = "ole32")]
extern "system" {
    pub fn RoGetAgileReference(options: u32, iid: *const Guid, object: Raw, result: *mut Raw)
        -> Hr;
}
