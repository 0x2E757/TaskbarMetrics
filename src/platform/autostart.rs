//! Start at sign-in: a value in the current user's Run key. No elevation is needed;
//! the value starts `TaskbarMetrics.exe --autostart`, which attaches
//! to the taskbar the same way as a manual start; its manifest keeps a console
//! from opening.

use super::abi::Raw;
use std::path::Path;

pub struct Autostart {
    command: String,
}

impl Autostart {
    const KEY: &'static str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE: &'static str = "Taskbar Metrics";
    /// Argument of `TaskbarMetrics.exe` that runs the sign-in start.
    pub const ARGUMENT: &'static str = "--autostart";

    /// Starts the copy in `directory`, the folder of the running executables.
    pub fn new(directory: &Path) -> Self {
        Self {
            command: format!(
                "\"{}\" {}",
                directory
                    .join(super::executables::Executables::LAUNCHER)
                    .display(),
                Self::ARGUMENT
            ),
        }
    }

    /// On only while the Run value starts this copy; one left by a copy elsewhere
    /// shows as off and is replaced when switched on.
    pub fn enabled(&self) -> bool {
        Self::read().as_deref() == Some(self.command.as_str())
    }

    pub fn set(&self, enabled: bool) -> std::io::Result<()> {
        let (key, value) = (wide(Self::KEY), wide(Self::VALUE));
        let status = if enabled {
            let data = wide(&self.command);
            // SAFETY: the strings are NUL-terminated and outlive the call; the size
            // counts the terminator.
            unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    key.as_ptr(),
                    value.as_ptr(),
                    REG_SZ,
                    data.as_ptr().cast(),
                    (data.len() * 2) as u32,
                )
            }
        } else {
            // SAFETY: NUL-terminated strings; a missing value is not an error.
            match unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr()) } {
                ERROR_FILE_NOT_FOUND => 0,
                status => status,
            }
        };
        match status {
            0 => Ok(()),
            status => Err(std::io::Error::from_raw_os_error(status)),
        }
    }

    fn read() -> Option<String> {
        let (key, value) = (wide(Self::KEY), wide(Self::VALUE));
        let mut data = vec![0u16; 1024];
        let mut size = (data.len() * 2) as u32;
        // SAFETY: the buffer holds `size` bytes; RRF_RT_REG_SZ adds the terminator.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                data.as_mut_ptr().cast(),
                &mut size,
            )
        };
        (status == 0).then(|| {
            let length = (size as usize / 2).saturating_sub(1);
            String::from_utf16_lossy(&data[..length.min(data.len())])
        })
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

const HKEY_CURRENT_USER: Raw = 0x8000_0001_u32 as i32 as isize as Raw;
const REG_SZ: u32 = 1;
const RRF_RT_REG_SZ: u32 = 2;
const ERROR_FILE_NOT_FOUND: i32 = 2;

#[link(name = "advapi32")]
extern "system" {
    fn RegSetKeyValueW(
        key: Raw,
        subkey: *const u16,
        value: *const u16,
        kind: u32,
        data: *const core::ffi::c_void,
        size: u32,
    ) -> i32;
    fn RegDeleteKeyValueW(key: Raw, subkey: *const u16, value: *const u16) -> i32;
    fn RegGetValueW(
        key: Raw,
        subkey: *const u16,
        value: *const u16,
        flags: u32,
        kind: *mut u32,
        data: *mut core::ffi::c_void,
        size: *mut u32,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_starts_the_quoted_copy_beside_the_window() {
        let autostart = Autostart::new(Path::new(r"C:\Program Files\Taskbar Metrics"));
        assert_eq!(
            autostart.command,
            r#""C:\Program Files\Taskbar Metrics\TaskbarMetrics.exe" --autostart"#
        );
    }
}
