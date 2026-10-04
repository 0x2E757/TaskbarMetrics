use super::{
    super::abi::*,
    store::{Frame, History},
};

use std::{
    io::BufWriter,
    sync::{Arc, Mutex, Weak},
};

pub(super) struct HistoryServer;
#[repr(C)]
struct SecurityAttributes {
    length: u32,
    descriptor: Raw,
    inherit: i32,
}

struct PipeSecurity(Raw);

impl PipeSecurity {
    fn new() -> Result<Self> {
        unsafe {
            let mut token = std::ptr::null_mut();
            if OpenProcessToken(-1isize as Raw, 8, &mut token) == 0 {
                return Err(last_error());
            }
            let token = Handle::new(token)?;
            let mut size = 0;
            GetTokenInformation(token.0, 1, std::ptr::null_mut(), 0, &mut size);
            let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
            if GetTokenInformation(token.0, 1, buffer.as_mut_ptr().cast(), size, &mut size) == 0 {
                return Err(last_error());
            }
            let sid = *(buffer.as_ptr() as *const Raw);
            let mut text = std::ptr::null_mut();
            if ConvertSidToStringSidW(sid, &mut text) == 0 {
                return Err(last_error());
            }
            let mut len = 0;
            while *text.add(len) != 0 {
                len += 1;
            }
            let sid = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
            LocalFree(text.cast());
            let sddl = wide(&format!(
                "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GR;;;{sid})S:(ML;;NW;;;ME)"
            ));
            let mut descriptor = std::ptr::null_mut();
            if ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                1,
                &mut descriptor,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err(last_error());
            }
            Ok(Self(descriptor))
        }
    }
}

impl Drop for PipeSecurity {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}

impl HistoryServer {
    pub fn stream(pid: u32, history: Arc<Mutex<History>>) {
        std::thread::spawn(move || {
            use std::os::windows::io::FromRawHandle;
            let Ok(security) = PipeSecurity::new() else {
                return;
            };
            let mut attributes = SecurityAttributes {
                length: std::mem::size_of::<SecurityAttributes>() as u32,
                descriptor: security.0,
                inherit: 0,
            };
            loop {
                let raw = unsafe {
                    CreateNamedPipeW(
                        wide(&format!("{}.Live", Self::path(pid))).as_ptr(),
                        2 | 0x80000,
                        8,
                        1,
                        65536,
                        0,
                        1000,
                        &mut attributes as *mut _ as Raw,
                    )
                };
                if raw as isize == -1 {
                    return;
                }
                let mut file = unsafe { std::fs::File::from_raw_handle(raw) };
                if unsafe { ConnectNamedPipe(raw, std::ptr::null_mut()) } == 0
                    && unsafe { GetLastError() } != 535
                {
                    return;
                }
                // Weak: a strong reference here would make every late I/O update
                // (`Arc::make_mut`) deep-copy its frame. The weak reference keeps the
                // allocation, so an unchanged address still means an unchanged frame.
                let mut previous = std::collections::BTreeMap::<u64, Weak<Frame>>::new();
                loop {
                    let (frames, enabled, status, io_totals) = {
                        let h = history.lock().unwrap_or_else(|e| e.into_inner());
                        (
                            h.frames.clone(),
                            h.enabled,
                            h.status.clone(),
                            h.io_totals
                                .iter()
                                .map(|(key, io)| (*key, io.clone()))
                                .collect(),
                        )
                    };
                    let changed = frames
                        .iter()
                        .filter(|f| {
                            !previous
                                .get(&f.bucket)
                                .is_some_and(|old| std::ptr::eq(old.as_ptr(), Arc::as_ptr(f)))
                        })
                        .cloned()
                        .collect();
                    previous = frames
                        .iter()
                        .map(|f| (f.bucket, Arc::downgrade(f)))
                        .collect();
                    drop(frames);
                    let packet = super::wire::Packet {
                        enabled,
                        status,
                        frames: changed,
                        io_totals,
                    };
                    // One write per pipe buffer (64 KB) instead of one per 8 KB.
                    if packet
                        .write(&mut BufWriter::with_capacity(65536, &mut file))
                        .is_err()
                    {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                unsafe {
                    DisconnectNamedPipe(raw);
                }
            }
        });
    }

    pub fn path(pid: u32) -> String {
        format!(r"\\.\pipe\TaskbarMetrics.History.{pid}")
    }

    pub fn spawn(pid: u32, history: Arc<Mutex<History>>) {
        std::thread::spawn(move || {
            use std::os::windows::io::FromRawHandle;
            let Ok(security) = PipeSecurity::new() else {
                return;
            };
            let mut attributes = SecurityAttributes {
                length: std::mem::size_of::<SecurityAttributes>() as u32,
                descriptor: security.0,
                inherit: 0,
            };
            loop {
                let raw = unsafe {
                    CreateNamedPipeW(
                        wide(&Self::path(pid)).as_ptr(),
                        2 | 0x80000, // PIPE_ACCESS_OUTBOUND | FIRST_PIPE_INSTANCE
                        8,
                        1,
                        65536,
                        0,
                        1000,
                        &mut attributes as *mut _ as Raw,
                    )
                };
                if raw as isize == -1 {
                    return;
                }
                let mut file = unsafe { std::fs::File::from_raw_handle(raw) };
                if unsafe { ConnectNamedPipe(raw, std::ptr::null_mut()) } == 0
                    && unsafe { GetLastError() } != 535
                {
                    return;
                }
                let packet = {
                    let history = history.lock().unwrap_or_else(|e| e.into_inner());
                    super::wire::Packet {
                        enabled: history.enabled,
                        status: history.status.clone(),
                        frames: history.frames.iter().cloned().collect(),
                        io_totals: Vec::new(),
                    }
                };
                let _ = packet.write(&mut BufWriter::with_capacity(65536, &mut file));
                // Only the export worker waits for the reader; sampling never does.
                unsafe {
                    FlushFileBuffers(raw);
                }
                unsafe {
                    DisconnectNamedPipe(raw);
                }
            }
        });
    }

    /// The whole history of the recorder for the Explorer `pid`, as `spawn` serves
    /// it. The pipe serves one reader at a time, so a busy one is waited for.
    pub fn read(pid: u32) -> std::io::Result<super::wire::Packet> {
        let path = Self::path(pid);
        let mut attempts = 0;
        let pipe = loop {
            match std::fs::File::open(&path) {
                // ERROR_PIPE_BUSY: another reader is being served.
                Err(error) if error.raw_os_error() == Some(231) && attempts < 50 => {
                    attempts += 1;
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                result => break result?,
            }
        };
        super::wire::Packet::read(&mut std::io::BufReader::with_capacity(65536, pipe))
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn LocalFree(memory: Raw) -> Raw;
    fn CreateNamedPipeW(
        name: *const u16,
        open: u32,
        mode: u32,
        instances: u32,
        out_size: u32,
        in_size: u32,
        timeout: u32,
        security: Raw,
    ) -> Raw;
    fn ConnectNamedPipe(pipe: Raw, overlapped: Raw) -> i32;
    fn DisconnectNamedPipe(pipe: Raw) -> i32;
    fn FlushFileBuffers(pipe: Raw) -> i32;
}

#[link(name = "advapi32")]
extern "system" {
    fn OpenProcessToken(process: Raw, access: u32, token: *mut Raw) -> i32;
    fn GetTokenInformation(token: Raw, class: u32, buffer: Raw, size: u32, needed: *mut u32)
        -> i32;
    fn ConvertSidToStringSidW(sid: Raw, text: *mut *mut u16) -> i32;
    fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
        text: *const u16,
        revision: u32,
        descriptor: *mut Raw,
        size: *mut u32,
    ) -> i32;
}
