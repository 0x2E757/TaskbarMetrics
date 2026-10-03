use super::{locale::Language, totals::IoTotals};
use crate::{
    metrics::GpuEngineUsage,
    platform::{
        devices::{DeviceId, DiskName},
        process_history::{
            memory::MemoryRows,
            store::{Frame, IoBytes, Sample},
        },
        providers::PhysicalMemory,
    },
};

pub type ProcessKey = (u32, u64);

/// Memory is counted in binary megabytes, as Windows shows it.
pub const MIB: f64 = 1_048_576.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resource {
    Cpu,
    Gpu,
    Ram,
    Disk,
    Net,
}

impl Resource {
    pub fn title(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Gpu => "GPU",
            Self::Ram => "RAM",
            Self::Disk => "Disk",
            Self::Net => "Network",
        }
    }

    pub const ALL: [Self; 5] = [Self::Cpu, Self::Gpu, Self::Ram, Self::Disk, Self::Net];

    pub fn id(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
            Self::Ram => "ram",
            Self::Disk => "disk",
            Self::Net => "net",
        }
    }

    /// Two directions drawn above and below the axis, with I/O attributed by ETW.
    pub fn dual(self) -> bool {
        matches!(self, Self::Disk | Self::Net)
    }

    /// Two values per process, shown with their sum: read and write, receive and
    /// send, private and shared memory.
    pub fn paired(self) -> bool {
        matches!(self, Self::Disk | Self::Net | Self::Ram)
    }

    /// Fixed 0–100 % value axis.
    pub fn percent(self) -> bool {
        matches!(self, Self::Cpu | Self::Gpu)
    }

    /// A second, 0–100 °C axis for the temperature line.
    pub fn temperature_axis(self) -> bool {
        matches!(self, Self::Cpu | Self::Gpu)
    }

    pub fn unit(self) -> &'static str {
        match self {
            Self::Ram => "MB",
            Self::Disk | Self::Net => "MB/s",
            _ => "%",
        }
    }

    pub fn floor(self) -> f64 {
        match self {
            Self::Cpu | Self::Gpu => 100.0,
            Self::Disk => 10.0,
            Self::Ram => 4096.0,
            Self::Net => 1.0,
        }
    }

    /// Kind of a device id.
    pub fn of(kind: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.id() == kind)
    }
}

/// One entry of the menu: a resource kind and the device whose readings it shows.
/// Reading ids carry the device tag (`disk_read@C:`); CPU and memory have none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub resource: Resource,
    pub id: DeviceId,
    /// Media and volumes of a disk the machine lists; demo disks have none.
    pub disk: Option<DiskName>,
    /// Link of a network adapter the machine lists: «Wi‑Fi», «Ethernet», «WWAN».
    pub link: Option<&'static str>,
    /// Number within its group (`group`), from 1; none for the only one.
    pub ordinal: Option<u32>,
}

impl Device {
    pub fn new(id: DeviceId) -> Option<Self> {
        Some(Self {
            resource: Resource::of(&id.kind)?,
            id,
            disk: None,
            link: None,
            ordinal: None,
        })
    }

    pub fn with_link(mut self, link: Option<&'static str>) -> Self {
        self.link = link;
        self
    }

    pub fn with_disk(mut self, disk: Option<DiskName>) -> Self {
        self.disk = disk;
        self
    }

    /// The main device of a kind, with untagged reading ids.
    #[cfg(test)]
    pub fn of(resource: Resource) -> Self {
        Self {
            resource,
            id: DeviceId::new(resource.id(), None),
            disk: None,
            link: None,
            ordinal: None,
        }
    }

    /// Devices numbered together: GPUs, disks of one media and adapters of one link.
    /// A demo disk tagged by its letter is named by it instead.
    pub fn group(&self) -> Option<&'static str> {
        match (self.resource, &self.disk) {
            (Resource::Gpu, _) => Some("GPU"),
            (Resource::Disk, Some(name)) => Some(name.media.unwrap_or("Disk")),
            (Resource::Disk, None) if !self.lettered() => Some("Disk"),
            (Resource::Net, _) => self.link,
            _ => None,
        }
    }

    /// A drive letter mapped to a network share.
    pub fn network_drive(&self) -> bool {
        self.disk.as_ref().is_some_and(|disk| disk.remote.is_some())
    }

    fn lettered(&self) -> bool {
        self.id.tag.as_deref().is_some_and(|tag| tag.ends_with(':'))
    }

    /// Menu and page title: "GPU 2", "SSD", "HDD 1", "Wi‑Fi", "Ethernet 2" (by link, as
    /// Task Manager names adapters; the Windows name is in the subtitle); CPU and
    /// memory keep theirs. An adapter of unknown link keeps its Windows name. A disk of unknown media is "Disk"; a demo disk, "Disk D:/".
    pub fn title(&self, language: Language) -> String {
        let disk = language.text("Disk");
        let numbered = |name: &str| match self.ordinal {
            Some(ordinal) => format!("{name} {ordinal}"),
            None => name.to_owned(),
        };
        match (self.resource, &self.disk, self.id.tag.as_deref()) {
            (Resource::Gpu, _, _) => numbered("GPU"),
            (Resource::Disk, Some(name), _) => {
                numbered(name.media.map_or(disk, |m| language.text(m)))
            }
            (Resource::Disk, None, Some(tag)) if self.lettered() => format!("{disk} {tag}/"),
            (Resource::Disk, None, _) => numbered(disk),
            (Resource::Net, _, _) if self.link.is_some() => numbered(self.link.unwrap_or_default()),
            (Resource::Net, _, Some(tag)) => tag.to_owned(),
            (resource, _, _) => language.text(resource.title()).to_owned(),
        }
    }

    fn total_of(&self, frame: &Frame, base: &str) -> Option<f64> {
        frame
            .totals
            .iter()
            .find(|(id, _)| self.id.is_reading(id, base))
            .and_then(|(_, value)| *value)
    }

    pub fn temperature(&self, frame: &Frame) -> Option<f64> {
        match self.resource {
            Resource::Cpu => self.total_of(frame, "cpu_temperature"),
            Resource::Gpu => self.total_of(frame, "gpu_temperature"),
            _ => None,
        }
    }

    /// Whether the recorder kept this device's values in `frame`: a device without
    /// a tile or «Always monitor» is recorded only while the window is open.
    pub fn recorded(&self, frame: &Frame) -> bool {
        let base = match self.resource {
            Resource::Cpu => "cpu",
            Resource::Gpu => "gpu",
            Resource::Ram => "ram",
            Resource::Disk => "disk_read",
            Resource::Net => "net_down",
        };
        frame
            .totals
            .iter()
            .any(|(id, _)| self.id.is_reading(id, base))
    }

    /// Busiest engine of this GPU.
    pub fn engine<'a>(&self, frame: &'a Frame) -> Option<&'a GpuEngineUsage> {
        frame
            .gpu_engines
            .iter()
            .find(|(device, _)| self.id.is(device))
            .map(|(_, engine)| engine)
    }

    /// All of the device in its chart unit, which its chart spans: the physical
    /// memory in MiB. Other devices are drawn against their peak.
    pub fn capacity(&self) -> Option<f64> {
        (self.resource == Resource::Ram)
            .then(PhysicalMemory::total_bytes)
            .flatten()
            .map(|bytes| bytes as f64 / MIB)
    }

    pub fn total(&self, frame: &Frame) -> [Option<f64>; 2] {
        match self.resource {
            Resource::Cpu => [self.total_of(frame, "cpu"), None],
            Resource::Gpu => [self.total_of(frame, "gpu"), None],
            // As on the taskbar: physical memory in use, which the rows add up to.
            Resource::Ram => [
                self.total_of(frame, "ram_used")
                    .map(|bytes| bytes / MIB)
                    .or_else(|| {
                        Some(
                            self.total_of(frame, "ram")? / 100.0
                                * PhysicalMemory::total_bytes()? as f64
                                / MIB,
                        )
                    }),
                None,
            ],
            Resource::Disk => [
                self.total_of(frame, "disk_read"),
                self.total_of(frame, "disk_write"),
            ],
            Resource::Net => [
                self.total_of(frame, "net_down"),
                self.total_of(frame, "net_up"),
            ],
        }
    }

    /// A process's values; disk and network I/O through this device only, or through
    /// all of them for an untagged device.
    pub fn process(&self, frame: &Frame, process: &Sample) -> [Option<f64>; 2] {
        let io = || {
            let bytes = process
                .io_bytes
                .as_deref()
                .map_or([0; 2], |io| self.bytes(io));
            bytes.map(|bytes| frame.etw_active.then_some(bytes as f64 / 500_000.0))
        };
        match self.resource {
            Resource::Cpu => [process.cpu(), None],
            Resource::Gpu => [
                self.engine(frame).map(|engine| {
                    engine
                        .processes
                        .get(&process.identity.pid)
                        .copied()
                        .unwrap_or(0.0)
                }),
                None,
            ],
            Resource::Ram => [process.private_working_set(), process.shared()]
                .map(|bytes| bytes.map(|n| n as f64 / MIB)),
            Resource::Disk | Resource::Net => io(),
        }
    }

    /// Read and write (receive and send) bytes of this disk (adapter) in `io`, or of
    /// all of them for an untagged device.
    pub fn bytes(&self, io: &IoBytes) -> [u64; 2] {
        let first = if self.resource == Resource::Net { 2 } else { 0 };
        match &self.id.tag {
            None => [io.total[first], io.total[first + 1]],
            Some(_) => io
                .devices
                .iter()
                .find(|(device, _)| self.id.is(device))
                .map_or([0; 2], |(_, bytes)| *bytes),
        }
    }

    /// A process's values as the chart draws them: private and shared memory as one.
    pub fn plotted(&self, frame: &Frame, process: &Sample) -> [Option<f64>; 2] {
        match (self.resource, self.process(frame, process)) {
            (Resource::Ram, [private, shared]) => [
                private
                    .zip(shared)
                    .map(|(a, b)| a + b)
                    .or(private)
                    .or(shared),
                None,
            ],
            (_, values) => values,
        }
    }

    /// Memory rows stand for no process and are listed on the RAM page only.
    pub fn lists(&self, process: &Sample) -> bool {
        self.resource == Resource::Ram || process.identity.pid != MemoryRows::PID
    }

    /// Processes matching `search`, busiest first by value `column`: either value of a
    /// pair (read, write; private, shared) or their sum, the only value elsewhere.
    /// Column 3 is the bytes since the recorder started, from `totals`; without them
    /// it sorts as the sum.
    pub fn ranked(
        &self,
        frame: &Frame,
        search: &str,
        column: usize,
        totals: Option<&IoTotals>,
    ) -> Vec<usize> {
        let query = search.to_lowercase();
        let mut rows: Vec<_> = frame
            .processes
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                self.lists(p)
                    && (query.is_empty()
                        || p.identity.name.to_lowercase().contains(&query)
                        || p.identity.pid.to_string().contains(&query))
            })
            .map(|(i, _)| i)
            .collect();
        // Each weight once, not twice per comparison of the sort.
        let weights: Vec<f64> = frame
            .processes
            .iter()
            .map(|p| self.weight(frame, p, column, totals))
            .collect();
        rows.sort_by(|a, b| {
            weights[*b].total_cmp(&weights[*a]).then_with(|| {
                frame.processes[*a]
                    .identity
                    .pid
                    .cmp(&frame.processes[*b].identity.pid)
            })
        });
        rows
    }

    /// Value of `sample` in the sort `column`: either value of a pair, their sum
    /// or the total since the window opened.
    pub fn weight(
        &self,
        frame: &Frame,
        sample: &Sample,
        column: usize,
        totals: Option<&IoTotals>,
    ) -> f64 {
        let column = if self.resource.paired() {
            column.min(3)
        } else {
            0
        };
        match (column, totals) {
            (3, Some(totals)) => totals
                .of(Timeline::key(sample))
                .map_or(0.0, |[first, second]| (first + second) as f64),
            (2 | 3, _) => self.process(frame, sample).into_iter().flatten().sum(),
            (column, _) => self.process(frame, sample)[column].unwrap_or(0.0),
        }
    }
}

pub struct Timeline;

impl Timeline {
    pub fn nearest(frames: &[std::sync::Arc<Frame>], fraction: f64) -> Option<usize> {
        let first = frames.first()?.bucket;
        let last = frames.last()?.bucket;
        let time = first as f64 + fraction.clamp(0.0, 1.0) * (last - first) as f64;
        frames
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                (a.bucket as f64 - time)
                    .abs()
                    .total_cmp(&(b.bucket as f64 - time).abs())
            })
            .map(|(i, _)| i)
    }

    pub fn key(sample: &Sample) -> ProcessKey {
        (sample.identity.pid, sample.identity.created)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::process_history::store::Identity;
    use std::sync::Arc;

    fn frame(bucket: u64) -> Arc<Frame> {
        Arc::new(Frame {
            gpu_engines: Vec::new(),
            bucket,
            elapsed_ms: 750.0,
            processes: vec![Sample::new(
                Arc::new(Identity {
                    pid: 7,
                    created: 42,
                    name: "test".into(),
                }),
                None,
                Some(2.0),
                [Some(5_000_000), None, None],
                Sample::boxed_io(IoBytes {
                    total: [500_000, 250_000, 0, 0],
                    devices: vec![("disk@D:".into(), [250_000, 0])],
                }),
            )],
            totals: vec![],
            etw_active: true,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 1.0,
        })
    }

    #[test]
    fn selection_uses_time_and_io_uses_fixed_etw_bucket() {
        let frames = vec![frame(100), frame(101), frame(110)];
        assert_eq!(Timeline::nearest(&frames, 0.2), Some(1));
        assert_eq!(
            Device::of(Resource::Disk).process(&frames[0], &frames[0].processes[0]),
            [Some(1.0), Some(0.5)]
        );
        let ram = Device::of(Resource::Ram);
        let mut memory = (*frames[0]).clone();
        memory.totals = vec![("ram_used".into(), Some(5.0 * MIB))];
        memory.processes[0].set_shared(Some(1 << 20));
        memory.processes.push(Sample::new(
            Arc::new(Identity {
                pid: MemoryRows::PID,
                created: 1,
                name: "Kernel: nonpaged pool".into(),
            }),
            None,
            None,
            [Some(1 << 20), None, None],
            None,
        ));
        assert_eq!(ram.total(&memory), [Some(5.0), None]);
        let process = &memory.processes[0];
        assert_eq!(ram.process(&memory, process), [Some(5e6 / MIB), Some(1.0)]);
        assert_eq!(ram.plotted(&memory, process), [Some(5e6 / MIB + 1.0), None]);
        // Memory rows are listed on the RAM page only.
        assert_eq!(ram.ranked(&memory, "", 2, None), [0, 1]);
        assert_eq!(Device::of(Resource::Cpu).ranked(&memory, "", 0, None), [0]);
        let d = Device::new(DeviceId::parse("disk@D:").unwrap()).unwrap();
        assert_eq!(
            d.process(&frames[0], &frames[0].processes[0]),
            [Some(0.5), Some(0.0)]
        );
        assert_eq!(
            Device::of(Resource::Cpu).process(&frames[0], &frames[0].processes[0]),
            [None, None]
        );
    }

    #[test]
    fn devices_are_named_by_group_and_number() {
        let title = |id: &str, ordinal| {
            let mut device = Device::new(DeviceId::parse(id).unwrap()).unwrap();
            device.ordinal = ordinal;
            device.title(Language::English)
        };
        assert_eq!(title("disk@C:", None), "Disk C:/");
        assert_eq!(title("disk@1", Some(2)), "Disk 2");
        assert_eq!(title("gpu@1", Some(2)), "GPU 2");
        // The only one of its group has no number.
        assert_eq!(title("gpu@0", None), "GPU");
        // Adapters are named by link, like in Task Manager; without one, by name.
        let adapter = |link, ordinal| {
            let mut device = Device::new(DeviceId::parse("net@Ethernet0 2").unwrap())
                .unwrap()
                .with_link(link);
            device.ordinal = ordinal;
            (device.group(), device.title(Language::English))
        };
        assert_eq!(
            adapter(Some("Ethernet"), None),
            (Some("Ethernet"), "Ethernet".into())
        );
        assert_eq!(
            adapter(Some("Ethernet"), Some(2)),
            (Some("Ethernet"), "Ethernet 2".into())
        );
        assert_eq!(adapter(None, None), (None, "Ethernet0 2".into()));
        // A disk the machine lists is named like a GPU: media and number.
        let named = |media, ordinal| {
            let mut device = Device::new(DeviceId::parse("disk@D:").unwrap())
                .unwrap()
                .with_disk(Some(DiskName {
                    media,
                    volumes: vec!["D:".into(), "E:".into()],
                    remote: None,
                }));
            device.ordinal = ordinal;
            (device.group(), device.title(Language::English))
        };
        assert_eq!(named(Some("HDD"), Some(1)), (Some("HDD"), "HDD 1".into()));
        assert_eq!(named(Some("SSD"), None), (Some("SSD"), "SSD".into()));
        assert_eq!(named(None, None), (Some("Disk"), "Disk".into()));
    }

    #[test]
    fn rows_rank_by_the_chosen_column() {
        let sample = |pid, io| {
            Sample::new(
                Arc::new(Identity {
                    pid,
                    created: 1,
                    name: "p".into(),
                }),
                Some(pid as f64),
                None,
                [None, None, None],
                Sample::boxed_io(IoBytes {
                    total: io,
                    devices: Vec::new(),
                }),
            )
        };
        let mut frame = (*frame(1)).clone();
        // pid 1 reads the most, pid 2 writes the most.
        frame.processes = vec![sample(1, [900, 100, 0, 0]), sample(2, [100, 800, 0, 0])];
        let disk = Device::of(Resource::Disk);
        assert_eq!(disk.ranked(&frame, "", 0, None), [0, 1]);
        assert_eq!(disk.ranked(&frame, "", 1, None), [1, 0]);
        // Read plus write: 1000 against 900.
        assert_eq!(disk.ranked(&frame, "", 2, None), [0, 1]);
        // Since the recorder started pid 2 moved more; without totals column 3 is the sum.
        let now = std::collections::HashMap::from([
            ((1, 1), IoBytes::default()),
            (
                (2, 1),
                IoBytes {
                    total: [5000, 5000, 0, 0],
                    devices: Vec::new(),
                },
            ),
        ]);
        let totals = IoTotals::at(&disk, &now, &[], frame.bucket);
        assert_eq!(disk.ranked(&frame, "", 3, Some(&totals)), [1, 0]);
        assert_eq!(disk.ranked(&frame, "", 3, None), [0, 1]);
        // A choice made on the disk page leaves single-value pages alone.
        assert_eq!(
            Device::of(Resource::Cpu).ranked(&frame, "", 1, None),
            [1, 0]
        );
    }
}
