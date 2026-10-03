use super::{
    devices::{Binding, DeviceId, DiskInstance, NetworkAdapter, NetworkDrive},
    gpu_temperature::GpuAdapters,
    pdh::PdhCounter,
};
use crate::metrics::{MetricDescriptor, MetricProvider, MetricValue};
use std::{cell::RefCell, rc::Rc};

/// Read or write throughput of one physical disk, in MB/s.
pub struct DiskThroughputProvider {
    descriptor: MetricDescriptor,
    counter: PdhCounter,
    tag: String,
}
impl DiskThroughputProvider {
    pub fn new(id: &DeviceId, write: bool) -> Self {
        Self {
            descriptor: MetricDescriptor::new(
                &id.reading(if write { "disk_write" } else { "disk_read" }),
                if write { "WRITE" } else { "READ" },
                " MB/s",
            ),
            counter: PdhCounter::new(if write {
                r"\PhysicalDisk(*)\Disk Write Bytes/sec"
            } else {
                r"\PhysicalDisk(*)\Disk Read Bytes/sec"
            }),
            tag: id.tag.clone().unwrap_or_else(DiskInstance::main_tag),
        }
    }
}
impl MetricProvider for DiskThroughputProvider {
    fn descriptor(&self) -> &MetricDescriptor {
        &self.descriptor
    }
    fn sample(&mut self) -> MetricValue {
        let tag = &self.tag;
        match self
            .counter
            .instance(|name| DiskInstance::matches(name, tag))
        {
            MetricValue::Available(bytes) => MetricValue::Available(bytes / 1_000_000.0),
            other => other,
        }
    }
}

/// Read or write throughput of a network drive, in MB/s: the SMB client's traffic to
/// the share mapped to the drive letter.
pub struct ShareThroughputProvider {
    descriptor: MetricDescriptor,
    counter: PdhCounter,
    drive: Binding<NetworkDrive>,
}
impl ShareThroughputProvider {
    pub fn new(id: &DeviceId, write: bool) -> Self {
        Self {
            descriptor: MetricDescriptor::new(
                &id.reading(if write { "disk_write" } else { "disk_read" }),
                if write { "WRITE" } else { "READ" },
                " MB/s",
            ),
            counter: PdhCounter::new(if write {
                r"\SMB Client Shares(*)\Write Bytes/sec"
            } else {
                r"\SMB Client Shares(*)\Read Bytes/sec"
            }),
            drive: Binding::new(id.clone()),
        }
    }
}
impl MetricProvider for ShareThroughputProvider {
    fn descriptor(&self) -> &MetricDescriptor {
        &self.descriptor
    }
    fn sample(&mut self) -> MetricValue {
        let Some(drive) = self.drive.get(|id| NetworkDrive::of(id.tag.as_deref()?)) else {
            return MetricValue::Unavailable;
        };
        // The share has no instance until Windows connects to it: no traffic yet.
        match self.counter.entries() {
            Ok(values) => MetricValue::Available(
                values
                    .into_iter()
                    .find(|(name, _)| drive.matches(name))
                    .map_or(0.0, |(_, bytes)| bytes / 1_000_000.0),
            ),
            Err(value) => value,
        }
    }
}

/// Received and sent bytes of every adapter from one PDH query. Collecting
/// `Network Interface` asks each adapter for its statistics and costs about a
/// millisecond, so both directions read one collection per tick.
pub struct NetworkTraffic {
    counter: PdhCounter,
    collected: Result<(), MetricValue>,
    collected_at: Option<std::time::Instant>,
    /// Directions that already read the last collection; the next read by one of
    /// them starts a new tick.
    read: [bool; 2],
}
impl NetworkTraffic {
    pub fn shared() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            counter: PdhCounter::several(&[
                r"\Network Interface(*)\Bytes Received/sec",
                r"\Network Interface(*)\Bytes Sent/sec",
            ]),
            collected: Err(MetricValue::Unavailable),
            collected_at: None,
            read: [true; 2],
        }))
    }
    /// Bytes per second of `instance`: received for direction 0, sent for 1.
    fn sample(&mut self, direction: usize, instance: &str) -> MetricValue {
        // Both directions of a tick read within microseconds, ticks are at least
        // 250 ms apart: an older collection belongs to an earlier tick, even when
        // the other direction skipped that tick.
        let stale = self
            .collected_at
            .is_none_or(|at| at.elapsed() >= std::time::Duration::from_millis(100));
        if self.read[direction] || stale {
            self.collected = self.counter.collect();
            self.collected_at = Some(std::time::Instant::now());
            self.read = [false; 2];
        }
        self.read[direction] = true;
        if let Err(value) = &self.collected {
            return value.clone();
        }
        match self.counter.collected(direction) {
            Ok(values) => values
                .into_iter()
                .find(|(name, _)| name == instance)
                .map_or(MetricValue::Unavailable, |(_, value)| {
                    MetricValue::Available(value)
                }),
            Err(value) => value,
        }
    }
}

/// Traffic of one network adapter; a bare `net` follows the adapter with the gateway.
pub struct NetworkProvider {
    descriptor: MetricDescriptor,
    traffic: Rc<RefCell<NetworkTraffic>>,
    upload: bool,
    adapter: Binding<String>,
}
impl NetworkProvider {
    /// Both directions of one device share `traffic`.
    pub fn new(id: &DeviceId, upload: bool, traffic: Rc<RefCell<NetworkTraffic>>) -> Self {
        Self {
            descriptor: MetricDescriptor::new(
                &id.reading(if upload { "net_up" } else { "net_down" }),
                if upload { "UP" } else { "DOWN" },
                " MB/s",
            ),
            traffic,
            upload,
            adapter: Binding::new(id.clone()),
        }
    }
}
impl MetricProvider for NetworkProvider {
    fn descriptor(&self) -> &MetricDescriptor {
        &self.descriptor
    }
    fn sample(&mut self) -> MetricValue {
        let Some(instance) = self.adapter.get(|id| {
            NetworkAdapter::find(&NetworkAdapter::list(), id).map(NetworkAdapter::instance)
        }) else {
            return MetricValue::Unavailable;
        };
        match self
            .traffic
            .borrow_mut()
            .sample(usize::from(self.upload), &instance)
        {
            MetricValue::Available(bytes) => MetricValue::Available(bytes / 1_000_000.0),
            other => other,
        }
    }
}

pub struct CpuProvider {
    descriptor: MetricDescriptor,
    counter: PdhCounter,
}
impl CpuProvider {
    pub fn new() -> Self {
        Self {
            descriptor: MetricDescriptor::new("cpu", "CPU", "%"),
            counter: PdhCounter::new(r"\Processor Information(_Total)\% Processor Utility"),
        }
    }
}
impl MetricProvider for CpuProvider {
    fn descriptor(&self) -> &MetricDescriptor {
        &self.descriptor
    }
    fn sample(&mut self) -> MetricValue {
        self.counter.scalar()
    }
}

/// Busiest engine of one GPU; a bare `gpu` is the primary adapter.
pub struct GpuProvider {
    descriptor: MetricDescriptor,
    counter: PdhCounter,
    adapter: Binding<(i32, u32)>,
}
impl GpuProvider {
    pub fn new(id: &DeviceId) -> Self {
        Self {
            descriptor: MetricDescriptor::new(&id.reading("gpu"), "GPU", "%"),
            counter: PdhCounter::new(r"\GPU Engine(*)\Utilization Percentage"),
            adapter: Binding::new(id.clone()),
        }
    }
}
impl MetricProvider for GpuProvider {
    fn descriptor(&self) -> &MetricDescriptor {
        &self.descriptor
    }
    fn sample(&mut self) -> MetricValue {
        match self
            .adapter
            .get(|id| GpuAdapters::list().get(GpuAdapters::index(id)?).copied())
        {
            Some(luid) => self.counter.gpu(luid),
            None => MetricValue::Unavailable,
        }
    }
}

#[repr(C)]
#[derive(Default)]
struct MemoryStatus {
    size: u32,
    load: u32,
    total: u64,
    available: u64,
    total_page: u64,
    available_page: u64,
    total_virtual: u64,
    available_virtual: u64,
    extended: u64,
}
#[link(name = "kernel32")]
extern "system" {
    fn GlobalMemoryStatusEx(status: *mut MemoryStatus) -> i32;
}

/// Physical memory usable by Windows, and the part in use, for the history.
pub struct PhysicalMemory;
impl PhysicalMemory {
    pub fn total_bytes() -> Option<u64> {
        Self::status().map(|status| status.total)
    }
    /// Bytes in use as the RAM tile counts them: total minus available.
    pub fn used_bytes() -> Option<u64> {
        Self::status().map(|status| status.total.saturating_sub(status.available))
    }
    fn status() -> Option<MemoryStatus> {
        let mut status = MemoryStatus {
            size: std::mem::size_of::<MemoryStatus>() as u32,
            ..Default::default()
        };
        (unsafe { GlobalMemoryStatusEx(&mut status) } != 0 && status.total > 0).then_some(status)
    }
}

pub struct RamProvider {
    descriptor: MetricDescriptor,
}
impl RamProvider {
    pub fn new() -> Self {
        Self {
            descriptor: MetricDescriptor::new("ram", "RAM", "%"),
        }
    }
}
impl MetricProvider for RamProvider {
    fn descriptor(&self) -> &MetricDescriptor {
        &self.descriptor
    }
    fn sample(&mut self) -> MetricValue {
        let mut status = MemoryStatus {
            size: std::mem::size_of::<MemoryStatus>() as u32,
            ..Default::default()
        };
        if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 || status.total == 0 {
            return MetricValue::Unavailable;
        }
        MetricValue::percent(
            100.0 * status.total.saturating_sub(status.available) as f64 / status.total as f64,
        )
    }
}
