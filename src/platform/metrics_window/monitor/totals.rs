//! Bytes each process moved since the recorder started, as of a moment.
use super::model::{Device, ProcessKey, Timeline};
use crate::platform::process_history::store::{Frame, IoBytes};
use std::{collections::HashMap, sync::Arc};

/// Read and written (received and sent) bytes of each process on one device.
pub struct IoTotals(HashMap<ProcessKey, [u64; 2]>);
impl IoTotals {
    /// Totals at `bucket`: the recorder's totals now, less the I/O of the frames after
    /// it. `frames` are the live ones, in order, from the same packet as `now`.
    pub fn at(
        device: &Device,
        now: &HashMap<ProcessKey, IoBytes>,
        frames: &[Arc<Frame>],
        bucket: u64,
    ) -> Self {
        let mut totals: HashMap<_, _> = now
            .iter()
            .map(|(key, io)| (*key, device.bytes(io)))
            .collect();
        for frame in frames.iter().rev().take_while(|f| f.bucket > bucket) {
            for process in &frame.processes {
                let (Some(io), Some(total)) = (
                    process.io_bytes.as_deref(),
                    totals.get_mut(&Timeline::key(process)),
                ) else {
                    continue;
                };
                for (total, later) in total.iter_mut().zip(device.bytes(io)) {
                    *total = total.saturating_sub(later);
                }
            }
        }
        Self(totals)
    }
    /// None for a process without I/O since the recorder started.
    pub fn of(&self, key: ProcessKey) -> Option<[u64; 2]> {
        self.0.get(&key).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{
        devices::DeviceId,
        process_history::store::{Identity, Sample},
    };
    fn frame(bucket: u64, read: u64) -> Arc<Frame> {
        Arc::new(Frame {
            gpu_engines: Vec::new(),
            bucket,
            elapsed_ms: 500.0,
            processes: vec![Sample::new(
                Arc::new(Identity {
                    pid: 7,
                    created: 1,
                    name: "p.exe".into(),
                }),
                None,
                None,
                [None; 3],
                Sample::boxed_io(IoBytes {
                    total: [read, 1, 0, 0],
                    devices: vec![("disk@D:".into(), [read, 0])],
                }),
            )],
            totals: Vec::new(),
            etw_active: true,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 1.0,
        })
    }
    #[test]
    fn totals_at_a_moment_leave_out_later_frames() {
        let frames = [frame(1, 100), frame(2, 200), frame(3, 400)];
        let now = HashMap::from([(
            (7, 1),
            IoBytes {
                total: [10_700, 50, 0, 0],
                devices: vec![("disk@D:".into(), [700, 0])],
            },
        )]);
        let disk = Device::new(DeviceId::parse("disk").unwrap()).unwrap();
        assert_eq!(
            IoTotals::at(&disk, &now, &frames, 3).of((7, 1)),
            Some([10_700, 50])
        );
        assert_eq!(
            IoTotals::at(&disk, &now, &frames, 1).of((7, 1)),
            Some([10_100, 48])
        );
        let d = Device::new(DeviceId::parse("disk@D:").unwrap()).unwrap();
        assert_eq!(
            IoTotals::at(&d, &now, &frames, 1).of((7, 1)),
            Some([100, 0])
        );
        assert_eq!(IoTotals::at(&disk, &now, &frames, 1).of((8, 1)), None);
    }
}
