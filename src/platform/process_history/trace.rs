//! Narrow kernel ETW subscription: disk completions, TCP/UDP bytes, thread IDs.
//! No stacks, filenames, packet contents, context switches or trace files.

use super::{
    super::{abi::*, devices::IoDevices},
    store::IoBytes,
};

use std::{
    collections::{BTreeMap, HashMap},
    net::IpAddr,
    sync::Mutex,
    thread::JoinHandle,
};

pub(super) type IoBuckets = BTreeMap<(u64, u32), IoBytes>;

#[derive(Default)]
pub(super) struct Events {
    pub bytes: IoBuckets,
    pub threads: HashMap<u32, u32>,
    pub undecoded: u64,
    /// Disks and adapters the events are attributed to.
    pub devices: IoDevices,
}

#[repr(C)]
struct Record {
    header: [u8; 80],
    context: u32,
    extended_count: u16,
    length: u16,
    extended: Raw,
    data: Raw,
    user: Raw,
}

struct Properties {
    words: [u64; 80],
}

impl Properties {
    fn new(name: &[u16]) -> Self {
        let mut value = Self { words: [0; 80] };
        value.set(0, 640); // WNODE_HEADER.BufferSize including logger name.
        value.set(40, 2); // SystemTime (100ns FILETIME), not delivery time.
        value.set(44, 0x20000); // WNODE_FLAG_TRACED_GUID
        value.set(48, 64);
        value.set(52, 8);
        value.set(56, 64);
        value.set(64, 0x02000100); // system logger, real time
        value.set(68, 1);
        value.set(72, 0x10010102);
        value.set(116, 120); // LoggerNameOffset: the name follows the 120-byte structure.
        unsafe {
            std::ptr::copy_nonoverlapping(
                name.as_ptr(),
                value.words.as_mut_ptr().cast::<u8>().add(120).cast(),
                name.len().min(260),
            );
        }
        value
    }

    fn set(&mut self, offset: usize, value: u32) {
        unsafe {
            self.words
                .as_mut_ptr()
                .cast::<u8>()
                .add(offset)
                .cast::<u32>()
                .write(value);
        }
    }

    fn get(&self, offset: usize) -> u32 {
        unsafe {
            self.words
                .as_ptr()
                .cast::<u8>()
                .add(offset)
                .cast::<u32>()
                .read()
        }
    }

    fn raw(&mut self) -> Raw {
        self.words.as_mut_ptr().cast()
    }
}

pub(super) struct IoTrace {
    handle: u64,
    consumer: u64,
    name: Vec<u16>,
    events: Box<Mutex<Events>>,
    worker: Option<JoinHandle<u32>>,
}

impl IoTrace {
    pub fn is_running(&self) -> bool {
        self.worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
    }

    pub fn start(threads: HashMap<u32, u32>) -> Result<Self> {
        let name = wide(&format!("TaskbarMetrics.ProcessHistory.{}", unsafe {
            GetCurrentProcessId()
        }));
        let mut properties = Properties::new(&name);
        let mut handle = 0;
        let status = unsafe { StartTraceW(&mut handle, name.as_ptr(), properties.raw()) };
        if status != 0 {
            return Err((0x80070000 | status) as i32);
        }
        let mut events = Box::new(Mutex::new(Events {
            threads,
            ..Events::default()
        }));
        // EVENT_TRACE_LOGFILEW, 448 bytes on x64. SDK offsets: LoggerName 8,
        // ProcessTraceMode 28, EventRecordCallback 424, Context 440.
        let mut logfile = [0u64; 56];
        logfile[1] = name.as_ptr() as u64;
        logfile[3] = (0x10000100u64) << 32; // REAL_TIME | EVENT_RECORD
        logfile[53] = Self::callback as *const () as u64;
        logfile[55] = (&mut *events as *mut Mutex<Events>) as u64;
        let consumer = unsafe { OpenTraceW(logfile.as_mut_ptr().cast()) };
        if consumer == u64::MAX {
            unsafe {
                ControlTraceW(handle, name.as_ptr(), properties.raw(), 1);
            }
            return Err(last_error());
        }
        let worker = std::thread::spawn(move || unsafe {
            ProcessTrace(&consumer, 1, std::ptr::null(), std::ptr::null())
        });
        Ok(Self {
            handle,
            consumer,
            name,
            events,
            worker: Some(worker),
        })
    }

    /// Replaces the disks and adapters new events are attributed to.
    pub fn set_devices(&self, devices: IoDevices) {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .devices = devices;
    }

    pub fn drain(&self, threads: HashMap<u32, u32>) -> (IoBuckets, u32, u64) {
        let mut properties = Properties::new(&self.name);
        unsafe {
            ControlTraceW(self.handle, self.name.as_ptr(), properties.raw(), 3);
        } // flush, bounded buffers
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        events.threads = threads;
        // Refresh the bounded map from the snapshot. Events arriving after this
        // point add/remove threads; unresolved completions remain unattributed.
        (
            std::mem::take(&mut events.bytes),
            // EventsLost + RealTimeBuffersLost.
            properties.get(88).saturating_add(properties.get(100)),
            events.undecoded,
        )
    }

    unsafe extern "system" fn callback(record: *mut Record) {
        let _ = std::panic::catch_unwind(|| {
            if record.is_null() {
                return;
            }
            let record = &*record;
            if record.user.is_null() || record.data.is_null() {
                return;
            }
            let provider = std::ptr::read_unaligned(record.header.as_ptr().add(24).cast::<Guid>());
            let opcode = record.header[45];
            let version = record.header[42];
            let data = std::slice::from_raw_parts(record.data.cast::<u8>(), record.length as usize);
            let read32 = |i: usize| {
                data.get(i..i + 4)
                    .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            };
            let state = &*(record.user as *const Mutex<Events>);
            let thread_guid = Guid::from_u128(0x3d6fa8d1_fe05_11d0_9dda_00c04fd7ba7c);
            if provider == thread_guid && matches!(opcode, 1..=4) {
                if let (Some(pid), Some(tid)) = (read32(0), read32(4)) {
                    let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                    if opcode == 2 || opcode == 4 {
                        state.threads.remove(&tid);
                    } else {
                        state.threads.insert(tid, pid);
                    }
                }
                return;
            }
            let tcp = Guid::from_u128(0x9a280ac0_c8e0_11d1_84e2_00c04fb998a2);
            let udp = Guid::from_u128(0xbf3a50c5_a9c9_4988_a005_2df0b7c80f80);
            let disk = Guid::from_u128(0x3d6fa8d4_fe05_11d0_9dda_00c04fd7ba7c);
            let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
            let address = |offset: usize, v6: bool| -> Option<IpAddr> {
                if v6 {
                    let bytes: [u8; 16] = data.get(offset..offset + 16)?.try_into().ok()?;
                    Some(IpAddr::from(bytes))
                } else {
                    let bytes: [u8; 4] = data.get(offset..offset + 4)?.try_into().ok()?;
                    Some(IpAddr::from(bytes))
                }
            };
            let (pid, count, column, device) = if (provider == tcp || provider == udp)
                && matches!(opcode, 10 | 11 | 26 | 27)
            {
                if version < 1 {
                    state.undecoded += 1;
                    return;
                }
                let (Some(pid), Some(count)) = (read32(0), read32(4)) else {
                    state.undecoded += 1;
                    return;
                };
                // TcpIp/UdpIp: daddr +8, then saddr (+12 for IPv4, +24 for IPv6).
                let v6 = opcode >= 26;
                let device = match (address(8, v6), address(if v6 { 24 } else { 12 }, v6)) {
                    (Some(remote), Some(local)) => state.devices.adapter(&local, &remote).cloned(),
                    _ => None,
                };
                (
                    pid,
                    count,
                    if opcode == 10 || opcode == 26 { 3 } else { 2 },
                    device,
                )
            } else if provider == disk && matches!(opcode, 10 | 11) {
                let flags = u16::from_le_bytes(record.header[4..6].try_into().unwrap());
                let tid_offset = if flags & 0x20 != 0 { 40 } else { 48 };
                if version != 3 {
                    state.undecoded += 1;
                    return;
                }
                let (Some(tid), Some(count)) = (read32(tid_offset), read32(8)) else {
                    state.undecoded += 1;
                    return;
                };
                // DiskIo_TypeGroup1 starts with DiskNumber.
                let device = read32(0).and_then(|number| state.devices.disk(number).cloned());
                (
                    state.threads.get(&tid).copied().unwrap_or(u32::MAX),
                    count,
                    if opcode == 10 { 0 } else { 1 },
                    device,
                )
            } else {
                return;
            };
            let timestamp = u64::from_le_bytes(record.header[16..24].try_into().unwrap());
            let bucket = timestamp.saturating_sub(116444736000000000) / 5_000_000;
            state.bytes.entry((bucket, pid)).or_default().add(
                column,
                count as u64,
                device.as_ref(),
            );
        });
    }
}

impl Drop for IoTrace {
    fn drop(&mut self) {
        let mut properties = Properties::new(&self.name);
        unsafe {
            ControlTraceW(self.handle, self.name.as_ptr(), properties.raw(), 1);
            CloseTrace(self.consumer);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[link(name = "advapi32")]
extern "system" {
    fn StartTraceW(handle: *mut u64, name: *const u16, properties: Raw) -> u32;
    fn ControlTraceW(handle: u64, name: *const u16, properties: Raw, control: u32) -> u32;
    fn OpenTraceW(logfile: Raw) -> u64;
    fn ProcessTrace(handles: *const u64, count: u32, start: *const u64, end: *const u64) -> u32;
    fn CloseTrace(handle: u64) -> u32;
}
