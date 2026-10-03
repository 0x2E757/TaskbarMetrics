//! Signal strength of a connected Wi‑Fi adapter, from the WLAN service. Windows 11
//! keeps the connection details (SSID, quality) behind location access; the RSSI
//! is open to every app.
use std::ptr;

/// A client of the WLAN service, open while the window lives.
pub struct Wlan {
    handle: isize,
}
impl Wlan {
    /// None on machines without the WLAN service.
    pub fn open() -> Option<Self> {
        let (mut version, mut handle) = (0, 0);
        // SAFETY: out pointers reference local storage; version 2 is Vista and later.
        let status = unsafe { WlanOpenHandle(2, ptr::null_mut(), &mut version, &mut handle) };
        (status == 0).then_some(Self { handle })
    }
    /// Received signal strength, dBm, of the interface whose description is
    /// `description`, as `GetAdaptersAddresses` names the adapter.
    pub fn rssi(&self, description: &str) -> Option<i32> {
        const RSSI: u32 = 0x1000_0102;
        let guid = self.interface(description)?;
        let (mut size, mut data, mut kind) = (0u32, ptr::null_mut(), 0u32);
        // SAFETY: the service writes a buffer it allocates; it is freed below.
        let status = unsafe {
            WlanQueryInterface(
                self.handle,
                &guid,
                RSSI,
                ptr::null_mut(),
                &mut size,
                &mut data,
                &mut kind,
            )
        };
        if status != 0 || data.is_null() {
            return None;
        }
        // SAFETY: the buffer holds a LONG.
        let rssi = (size >= 4).then(|| unsafe { data.cast::<i32>().read_unaligned() });
        // SAFETY: allocated by WlanQueryInterface.
        unsafe { WlanFreeMemory(data) };
        rssi
    }
    /// GUID of the interface with `description`. WLAN_INTERFACE_INFO_LIST: count
    /// +0, items from +8, each a GUID, a 256-character description and a state.
    fn interface(&self, description: &str) -> Option<[u8; 16]> {
        let mut list = ptr::null_mut();
        // SAFETY: the service writes a list it allocates; it is freed below.
        if unsafe { WlanEnumInterfaces(self.handle, ptr::null_mut(), &mut list) } != 0 {
            return None;
        }
        // SAFETY: the list holds `count` items of 532 bytes after its 8-byte header.
        let found = unsafe {
            let count = list.cast::<u32>().read_unaligned() as usize;
            (0..count).find_map(|index| {
                let item = list.add(8 + index * 532);
                let name = std::slice::from_raw_parts(item.add(16).cast::<u16>(), 256);
                let end = name.iter().position(|c| *c == 0).unwrap_or(name.len());
                (String::from_utf16_lossy(&name[..end]) == description)
                    .then(|| item.cast::<[u8; 16]>().read_unaligned())
            })
        };
        // SAFETY: allocated by WlanEnumInterfaces.
        unsafe { WlanFreeMemory(list) };
        found
    }
}
impl Drop for Wlan {
    fn drop(&mut self) {
        // SAFETY: the handle came from WlanOpenHandle.
        unsafe { WlanCloseHandle(self.handle, ptr::null_mut()) };
    }
}

#[link(name = "wlanapi")]
extern "system" {
    fn WlanOpenHandle(
        version: u32,
        reserved: *mut u8,
        negotiated: *mut u32,
        handle: *mut isize,
    ) -> u32;
    fn WlanCloseHandle(handle: isize, reserved: *mut u8) -> u32;
    fn WlanEnumInterfaces(handle: isize, reserved: *mut u8, list: *mut *mut u8) -> u32;
    fn WlanQueryInterface(
        handle: isize,
        guid: *const [u8; 16],
        opcode: u32,
        reserved: *mut u8,
        size: *mut u32,
        data: *mut *mut u8,
        kind: *mut u32,
    ) -> u32;
    fn WlanFreeMemory(memory: *mut u8);
}
