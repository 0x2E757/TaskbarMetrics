use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub(crate) struct Identity {
    pub pid: u32,
    pub created: u64,
    pub name: Arc<str>,
}

/// Identities shared by the frames a viewer keeps: a process mentioned by all 600
/// frames is one allocation instead of 600.
#[derive(Default)]
pub(crate) struct Identities(std::collections::HashMap<(u32, u64), Vec<Arc<Identity>>>);

impl Identities {
    pub fn get(&mut self, pid: u32, created: u64, name: &str) -> Arc<Identity> {
        let known = self.0.entry((pid, created)).or_default();
        if let Some(identity) = known.iter().find(|i| &*i.name == name) {
            return identity.clone();
        }
        let identity = Arc::new(Identity {
            pid,
            created,
            name: name.into(),
        });
        known.push(identity.clone());
        identity
    }

    /// Forgets identities no frame refers to any more.
    pub fn prune(&mut self) {
        self.0.retain(|_, known| {
            known.retain(|identity| Arc::strong_count(identity) > 1);
            !known.is_empty()
        });
    }
}

/// Bytes of one process in one bucket: disk read, disk write, net receive and net
/// send in total, and per device (`disk@C:` read/write, `net@Wi‑Fi` receive/send).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct IoBytes {
    pub total: [u64; 4],
    pub devices: Vec<(Arc<str>, [u64; 2])>,
}

impl IoBytes {
    /// `column` indexes `total`; I/O without a known device counts in the total only.
    pub fn add(&mut self, column: usize, count: u64, device: Option<&Arc<str>>) {
        self.total[column] = self.total[column].saturating_add(count);
        if let Some(device) = device {
            let bytes = self.device(device);
            bytes[column % 2] = bytes[column % 2].saturating_add(count);
        }
    }

    pub fn merge(&mut self, other: &Self) {
        for (total, value) in self.total.iter_mut().zip(other.total) {
            *total = total.saturating_add(value);
        }
        for (device, values) in &other.devices {
            let bytes = self.device(device);
            for (total, value) in bytes.iter_mut().zip(values) {
                *total = total.saturating_add(*value);
            }
        }
    }

    fn device(&mut self, id: &Arc<str>) -> &mut [u64; 2] {
        let index = match self.devices.iter().position(|(device, _)| device == id) {
            Some(index) => index,
            None => {
                self.devices.push((id.clone(), [0; 2]));
                self.devices.len() - 1
            }
        };
        &mut self.devices[index].1
    }
}

/// One process in one frame. Values not measured are kept as the sentinels the wire
/// format uses (NaN, `u64::MAX`) rather than as `Option`s, which would double their
/// size in each of the 600 frames; the accessors return `Option`s again.
#[derive(Clone)]
pub(crate) struct Sample {
    pub identity: Arc<Identity>,
    /// CPU and GPU percent.
    usage: [f64; 2],
    /// Private working set, working set, private bytes and the share of shared
    /// pages, see `shared`.
    memory: [u64; 4],
    /// Disk and network bytes, see `io` and `device_io`. Few processes do I/O in a
    /// bucket; the others keep none instead of 56 bytes in each of the 600 frames.
    pub io_bytes: Option<Box<IoBytes>>,
}

impl Sample {
    pub fn new(
        identity: Arc<Identity>,
        cpu: Option<f64>,
        gpu: Option<f64>,
        memory: [Option<u64>; 3],
        io_bytes: Option<Box<IoBytes>>,
    ) -> Self {
        Self {
            identity,
            usage: [cpu, gpu].map(|value| value.unwrap_or(f64::NAN)),
            memory: [memory[0], memory[1], memory[2], None].map(|v| v.unwrap_or(u64::MAX)),
            io_bytes,
        }
    }

    fn measured(value: f64) -> Option<f64> {
        (!value.is_nan()).then_some(value)
    }

    fn bytes(value: u64) -> Option<u64> {
        (value != u64::MAX).then_some(value)
    }

    pub fn cpu(&self) -> Option<f64> {
        Self::measured(self.usage[0])
    }

    pub fn gpu(&self) -> Option<f64> {
        Self::measured(self.usage[1])
    }

    pub fn private_working_set(&self) -> Option<u64> {
        Self::bytes(self.memory[0])
    }

    pub fn working_set(&self) -> Option<u64> {
        Self::bytes(self.memory[1])
    }

    pub fn private_bytes(&self) -> Option<u64> {
        Self::bytes(self.memory[2])
    }

    /// This process's part of the shared pages in its working set: a page mapped by
    /// n working sets counts 1/n. None while no window shows the history.
    pub fn shared(&self) -> Option<u64> {
        Self::bytes(self.memory[3])
    }

    pub fn set_shared(&mut self, bytes: Option<u64>) {
        self.memory[3] = bytes.unwrap_or(u64::MAX);
    }

    /// Disk read, disk write, net receive and net send bytes.
    pub fn io(&self) -> [u64; 4] {
        self.io_bytes.as_ref().map_or([0; 4], |io| io.total)
    }

    /// `io` of each disk and network adapter the I/O went through.
    pub fn device_io(&self) -> &[(Arc<str>, [u64; 2])] {
        self.io_bytes.as_ref().map_or(&[], |io| &io.devices)
    }

    /// `io_bytes` holding `io`: none when there was no I/O.
    pub fn boxed_io(io: IoBytes) -> Option<Box<IoBytes>> {
        (io != IoBytes::default()).then(|| Box::new(io))
    }
}

#[derive(Clone)]
pub(crate) struct Frame {
    /// Busiest engine of each recorded GPU, by device id (`gpu@0`).
    pub gpu_engines: Vec<(String, crate::metrics::GpuEngineUsage)>,
    pub bucket: u64,
    pub elapsed_ms: f64,
    pub processes: Vec<Sample>,
    pub totals: Vec<(String, Option<f64>)>,
    pub etw_active: bool,
    pub lost_events: u32,
    pub undecoded: u64,
    pub sample_ms: f64,
}

/// Buckets of 0.5 s kept by the recorder: the last 5 minutes.
pub const RETENTION: u64 = 600;

#[derive(Default)]
pub(crate) struct History {
    pub frames: VecDeque<Arc<Frame>>,
    pub enabled: bool,
    pub status: String,
    /// I/O of each process since the recorder started, late events included: the
    /// frames hold only the last 5 minutes of it.
    pub io_totals: HashMap<(u32, u64), IoBytes>,
}

impl History {
    pub fn prune(&mut self, now: u64) {
        while self
            .frames
            .front()
            .is_some_and(|f| f.bucket + RETENTION <= now)
        {
            self.frames.pop_front();
        }
    }

    pub fn push(&mut self, frame: Frame) {
        self.prune(frame.bucket);
        if self.frames.back().is_some_and(|f| f.bucket == frame.bucket) {
            self.frames.pop_back();
        }
        let bucket = frame.bucket;
        self.frames.push_back(Arc::new(frame));
        while self.frames.len() > RETENTION as usize {
            self.frames.pop_front();
        }
        // Once a minute: the totals of processes no frame mentions any more go.
        if bucket.is_multiple_of(120) {
            let present: HashSet<_> = self
                .frames
                .iter()
                .flat_map(|f| &f.processes)
                .map(|p| (p.identity.pid, p.identity.created))
                .collect();
            self.io_totals.retain(|key, _| present.contains(key));
        }
    }

    /// Identity that was alive in `bucket` under `pid`, not the process currently
    /// using the PID. Late ETW buffers update old frames.
    fn identity(&self, bucket: u64, pid: u32) -> Arc<Identity> {
        self.frames
            .iter()
            .rev()
            .filter(|f| f.bucket <= bucket)
            .find_map(|f| {
                f.processes
                    .iter()
                    .find(|p| p.identity.pid == pid)
                    .map(|p| p.identity.clone())
            })
            .unwrap_or_else(|| {
                Arc::new(Identity {
                    pid,
                    created: 0,
                    name: if pid == u32::MAX {
                        "Unattributed disk I/O".into()
                    } else {
                        format!("PID {pid} (not sampled)").into()
                    },
                })
            })
    }

    pub fn add_io(&mut self, bucket: u64, pid: u32, bytes: &IoBytes) {
        let Some(position) = self.frames.iter().position(|f| f.bucket == bucket) else {
            return;
        };
        // The sample of `pid`, or the identity of a new row: searching older
        // frames is needed only for a process the frame lacks.
        let row = match self.frames[position]
            .processes
            .iter()
            .position(|p| p.identity.pid == pid)
        {
            Some(index) => Ok(index),
            None => Err(self.identity(bucket, pid)),
        };
        let frame = Arc::make_mut(&mut self.frames[position]);
        let index = match row {
            Ok(index) => index,
            Err(identity) => {
                frame
                    .processes
                    .push(Sample::new(identity, None, None, [None, None, None], None));
                frame.processes.len() - 1
            }
        };
        let sample = &mut frame.processes[index];
        self.io_totals
            .entry((sample.identity.pid, sample.identity.created))
            .or_default()
            .merge(bytes);
        let mut merged = sample
            .io_bytes
            .take()
            .map_or_else(IoBytes::default, |io| *io);
        merged.merge(bytes);
        sample.io_bytes = Sample::boxed_io(merged);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(bucket: u64, created: u64) -> Frame {
        Frame {
            gpu_engines: Vec::new(),
            bucket,
            elapsed_ms: 500.0,
            processes: vec![Sample::new(
                Arc::new(Identity {
                    pid: 42,
                    created,
                    name: "test".into(),
                }),
                None,
                None,
                [None, None, None],
                None,
            )],
            totals: vec![],
            etw_active: true,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 0.0,
        }
    }

    #[test]
    fn samples_keep_missing_values_apart_from_zeroes_in_64_bytes() {
        let identity = frame(1, 1).processes[0].identity.clone();
        let measured = Sample::new(
            identity.clone(),
            Some(0.0),
            Some(2.5),
            [Some(0), Some(7), Some(9)],
            None,
        );
        assert_eq!((measured.cpu(), measured.gpu()), (Some(0.0), Some(2.5)));
        assert_eq!(
            [
                measured.private_working_set(),
                measured.working_set(),
                measured.private_bytes()
            ],
            [Some(0), Some(7), Some(9)]
        );
        let mut missing = Sample::new(identity, None, None, [None; 3], None);
        assert_eq!(
            (missing.cpu(), missing.working_set(), missing.shared()),
            (None, None, None)
        );
        missing.set_shared(Some(0));
        assert_eq!(missing.shared(), Some(0));
        assert_eq!(std::mem::size_of::<Sample>(), 64);
    }

    #[test]
    fn bounded_five_minutes_and_delayed_io_survives_pid_reuse() {
        let mut h = History::default();
        h.push(frame(1, 100));
        h.push(frame(2, 200));
        let mut bytes = IoBytes::default();
        bytes.add(0, 1024, Some(&"disk@C:".into()));
        bytes.add(3, 10, None);
        h.add_io(1, 42, &bytes);
        h.add_io(1, 42, &bytes);
        assert_eq!(h.frames[0].processes[0].identity.created, 100);
        assert_eq!(h.frames[0].processes[0].io(), [2048, 0, 0, 20]);
        assert_eq!(
            h.frames[0].processes[0].device_io(),
            [("disk@C:".into(), [2048, 0])]
        );
        assert_eq!(h.frames[1].processes[0].io()[0], 0);
        assert!(h.frames[1].processes[0].io_bytes.is_none());
        // The totals outlive the frames while the process is in one of them.
        h.add_io(2, 42, &bytes);
        assert_eq!(h.io_totals[&(42, 100)].total, [2048, 0, 0, 20]);
        assert_eq!(h.io_totals[&(42, 200)].total, [1024, 0, 0, 10]);
        for i in 3..=720 {
            h.push(frame(i, 200));
        }
        assert_eq!(h.frames.len(), 600);
        assert!(!h.io_totals.contains_key(&(42, 100)));
        assert_eq!(h.io_totals[&(42, 200)].total[0], 1024);
        h.prune(1400);
        assert!(h.frames.is_empty());
    }
}
