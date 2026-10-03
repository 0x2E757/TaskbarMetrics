//! SeDebugPrivilege in the recorder's own token.

use crate::platform::abi::*;

/// Lets the elevated recorder open the processes of services and other accounts,
/// such as a virtual machine's, to list their working sets. Their memory is not read.
pub(super) struct DebugPrivilege;

impl DebugPrivilege {
    /// True when the privilege is now enabled; a token without it, as unelevated
    /// ones are, cannot enable it.
    pub fn enable() -> bool {
        // SAFETY: the token handle is owned by `Handle`, the structures are sized and
        // laid out as the API expects, and the name is NUL terminated.
        unsafe {
            let mut token = std::ptr::null_mut();
            // TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY of this process (pseudo-handle -1).
            if OpenProcessToken(-1isize as Raw, 0x0028, &mut token) == 0 {
                return false;
            }
            let token = Handle(token);
            let mut privileges = TokenPrivileges {
                count: 1,
                luid: Luid::default(),
                // SE_PRIVILEGE_ENABLED.
                attributes: 2,
            };
            if LookupPrivilegeValueW(
                std::ptr::null(),
                wide("SeDebugPrivilege").as_ptr(),
                &mut privileges.luid,
            ) == 0
            {
                return false;
            }
            // Succeeds without assigning when the token lacks the privilege; only
            // ERROR_SUCCESS means it was assigned.
            AdjustTokenPrivileges(
                token.0,
                0,
                &privileges,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            ) != 0
                && GetLastError() == 0
        }
    }
}

#[repr(C)]
#[derive(Default)]
struct Luid {
    low: u32,
    high: i32,
}

/// TOKEN_PRIVILEGES with one LUID_AND_ATTRIBUTES.
#[repr(C)]
struct TokenPrivileges {
    count: u32,
    luid: Luid,
    attributes: u32,
}

#[link(name = "advapi32")]
extern "system" {
    fn OpenProcessToken(process: Raw, access: u32, token: *mut Raw) -> i32;
    fn LookupPrivilegeValueW(system: *const u16, name: *const u16, luid: *mut Luid) -> i32;
    fn AdjustTokenPrivileges(
        token: Raw,
        disable_all: i32,
        state: *const TokenPrivileges,
        length: u32,
        previous: Raw,
        returned: *mut u32,
    ) -> i32;
}
