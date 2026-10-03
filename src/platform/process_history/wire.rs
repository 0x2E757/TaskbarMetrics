//! Versioned, bounded binary history packets shared by recorder and viewer.
use super::store::{Frame, Identities, IoBytes, Sample};
use std::{
    io::{self, Read, Write},
    sync::Arc,
};

pub(crate) struct Packet {
    pub enabled: bool,
    pub status: String,
    pub frames: Vec<Arc<Frame>>,
    /// I/O of each process since the recorder started, by PID and creation time.
    pub io_totals: Vec<((u32, u64), IoBytes)>,
}
impl Packet {
    /// Version 5: GPU engines and totals per device, shared memory of processes,
    /// I/O totals since the recorder started.
    const MAGIC: &'static [u8; 4] = b"TMH5";
    pub fn write(&self, output: &mut impl Write) -> io::Result<()> {
        output.write_all(Self::MAGIC)?;
        output.write_all(&[self.enabled as u8])?;
        Self::text(output, &self.status)?;
        Self::number(output, self.frames.len() as u64)?;
        for f in &self.frames {
            Self::number(output, f.bucket)?;
            Self::number(output, f.elapsed_ms.to_bits())?;
            Self::number(output, f.etw_active as u64)?;
            Self::number(output, f.lost_events as u64)?;
            Self::number(output, f.undecoded)?;
            Self::number(output, f.sample_ms.to_bits())?;
            Self::number(output, f.gpu_engines.len() as u64)?;
            for (device, engine) in &f.gpu_engines {
                Self::text(output, device)?;
                Self::text(output, &engine.name)?;
                Self::text(output, &engine.label)?;
                Self::number(output, engine.total.to_bits())?;
                Self::number(output, engine.raw_total.to_bits())?;
                Self::number(output, engine.processes.len() as u64)?;
                for (pid, value) in &engine.processes {
                    Self::number(output, *pid as u64)?;
                    Self::number(output, value.to_bits())?;
                }
            }
            Self::number(output, f.totals.len() as u64)?;
            for (id, value) in &f.totals {
                Self::text(output, id)?;
                Self::number(output, value.unwrap_or(f64::NAN).to_bits())?;
            }
            Self::number(output, f.processes.len() as u64)?;
            for p in &f.processes {
                Self::number(output, p.identity.pid as u64)?;
                Self::number(output, p.identity.created)?;
                Self::text(output, &p.identity.name)?;
                for v in [p.cpu(), p.gpu()] {
                    Self::number(output, v.unwrap_or(f64::NAN).to_bits())?;
                }
                for v in [
                    p.private_working_set(),
                    p.working_set(),
                    p.private_bytes(),
                    p.shared(),
                ] {
                    Self::number(output, v.unwrap_or(u64::MAX))?;
                }
                Self::write_io(output, p.io(), p.device_io())?;
            }
        }
        Self::number(output, self.io_totals.len() as u64)?;
        for ((pid, created), io) in &self.io_totals {
            Self::number(output, *pid as u64)?;
            Self::number(output, *created)?;
            Self::write_io(output, io.total, &io.devices)?;
        }
        output.flush()
    }
    fn write_io(
        output: &mut impl Write,
        total: [u64; 4],
        devices: &[(Arc<str>, [u64; 2])],
    ) -> io::Result<()> {
        for v in total {
            Self::number(output, v)?;
        }
        Self::number(output, devices.len() as u64)?;
        for (device, bytes) in devices {
            Self::text(output, device)?;
            for v in bytes {
                Self::number(output, *v)?;
            }
        }
        Ok(())
    }
    fn read_io(input: &mut impl Read) -> io::Result<IoBytes> {
        let mut total = [0; 4];
        for v in &mut total {
            *v = Self::read_number(input)?;
        }
        let mut devices = Vec::new();
        for _ in 0..Self::count(input, 64)? {
            let device = Self::read_text(input)?.into();
            devices.push((
                device,
                [Self::read_number(input)?, Self::read_number(input)?],
            ));
        }
        Ok(IoBytes { total, devices })
    }
    #[cfg(test)]
    pub fn read(input: &mut impl Read) -> io::Result<Self> {
        Self::read_shared(input, &mut Identities::default())
    }
    /// Reads a packet whose processes reuse the identities a viewer already holds.
    pub fn read_shared(input: &mut impl Read, identities: &mut Identities) -> io::Result<Self> {
        let mut magic = [0; 5];
        input.read_exact(&mut magic)?;
        if &magic[..4] != Self::MAGIC {
            return Err(Self::invalid());
        }
        let status = Self::read_text(input)?;
        // Recorders built before the 5-minute window still send up to 10 minutes.
        let count = Self::count(input, 1200)?;
        let mut frames = Vec::with_capacity(count);
        let mut budget = 2_000_000usize;
        for _ in 0..count {
            let bucket = Self::read_number(input)?;
            let elapsed_ms = f64::from_bits(Self::read_number(input)?);
            if !elapsed_ms.is_finite() || elapsed_ms <= 0.0 {
                return Err(Self::invalid());
            }
            let etw_active = Self::read_number(input)? != 0;
            let lost_events = Self::read_number(input)? as u32;
            let undecoded = Self::read_number(input)?;
            let sample_ms = f64::from_bits(Self::read_number(input)?);
            let mut gpu_engines = Vec::new();
            for _ in 0..Self::count(input, 64)? {
                let device = Self::read_text(input)?;
                let name = Self::read_text(input)?;
                let label = Self::read_text(input)?;
                let total = Self::optional(input)?.ok_or_else(Self::invalid)?;
                let raw_total = Self::optional(input)?.ok_or_else(Self::invalid)?;
                let mut processes = std::collections::BTreeMap::new();
                for _ in 0..Self::count(input, 65536)? {
                    processes.insert(
                        Self::read_number(input)? as u32,
                        Self::optional(input)?.ok_or_else(Self::invalid)?,
                    );
                }
                gpu_engines.push((
                    device,
                    crate::metrics::GpuEngineUsage {
                        name,
                        label,
                        total,
                        raw_total,
                        processes,
                    },
                ));
            }
            let mut totals = Vec::new();
            for _ in 0..Self::count(input, 256)? {
                totals.push((Self::read_text(input)?, Self::optional(input)?));
            }
            let count = Self::count(input, 65536)?;
            budget = budget.checked_sub(count).ok_or_else(Self::invalid)?;
            let mut processes = Vec::with_capacity(count);
            for _ in 0..count {
                let pid = Self::read_number(input)? as u32;
                let created = Self::read_number(input)?;
                let name = Self::read_text(input)?;
                let cpu = Self::optional(input)?;
                let gpu = Self::optional(input)?;
                let mut memory = [None; 4];
                for v in &mut memory {
                    let n = Self::read_number(input)?;
                    *v = (n != u64::MAX).then_some(n);
                }
                let mut sample = Sample::new(
                    identities.get(pid, created, &name),
                    cpu,
                    gpu,
                    [memory[0], memory[1], memory[2]],
                    Sample::boxed_io(Self::read_io(input)?),
                );
                sample.set_shared(memory[3]);
                processes.push(sample);
            }
            frames.push(Arc::new(Frame {
                gpu_engines,
                bucket,
                elapsed_ms,
                processes,
                totals,
                etw_active,
                lost_events,
                undecoded,
                sample_ms,
            }));
        }
        let count = Self::count(input, 65536)?;
        let mut io_totals = Vec::with_capacity(count);
        for _ in 0..count {
            let key = (Self::read_number(input)? as u32, Self::read_number(input)?);
            io_totals.push((key, Self::read_io(input)?));
        }
        Ok(Self {
            enabled: magic[4] != 0,
            status,
            frames,
            io_totals,
        })
    }
    fn optional(input: &mut impl Read) -> io::Result<Option<f64>> {
        let value = f64::from_bits(Self::read_number(input)?);
        Ok((value.is_finite() && value >= 0.0).then_some(value))
    }
    fn number(output: &mut impl Write, value: u64) -> io::Result<()> {
        output.write_all(&value.to_le_bytes())
    }
    fn read_number(input: &mut impl Read) -> io::Result<u64> {
        let mut value = [0; 8];
        input.read_exact(&mut value)?;
        Ok(u64::from_le_bytes(value))
    }
    fn count(input: &mut impl Read, maximum: usize) -> io::Result<usize> {
        let value = Self::read_number(input)?;
        if value > maximum as u64 {
            Err(Self::invalid())
        } else {
            Ok(value as usize)
        }
    }
    fn text(output: &mut impl Write, text: &str) -> io::Result<()> {
        Self::number(output, text.len() as u64)?;
        output.write_all(text.as_bytes())
    }
    fn read_text(input: &mut impl Read) -> io::Result<String> {
        let mut bytes = vec![0; Self::count(input, 32768)?];
        input.read_exact(&mut bytes)?;
        String::from_utf8(bytes).map_err(|_| Self::invalid())
    }
    fn invalid() -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, "Invalid history packet")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::process_history::store::Identity;
    #[test]
    fn packet_round_trip_preserves_identity_missing_values_and_io() {
        let mut sample = Sample::new(
            Arc::new(Identity {
                pid: 7,
                created: 42,
                name: "tést,\".exe".into(),
            }),
            Some(3.5),
            None,
            [Some(999), None, None],
            Sample::boxed_io(IoBytes {
                total: [1, 2, 3, 4],
                devices: vec![("disk@C:".into(), [1, 2]), ("net@Wi‑Fi".into(), [3, 4])],
            }),
        );
        sample.set_shared(Some(4096));
        let packet = Packet {
            enabled: true,
            status: "ok".into(),
            frames: vec![Arc::new(Frame {
                gpu_engines: vec![(
                    "gpu@1".into(),
                    crate::metrics::GpuEngineUsage {
                        name: "luid_test_phys_0_eng_0".into(),
                        label: "3D".into(),
                        total: 3.5,
                        raw_total: 3.5,
                        processes: std::collections::BTreeMap::from([(7, 3.5)]),
                    },
                )],
                bucket: 10,
                elapsed_ms: 500.0,
                processes: vec![sample],
                // More device totals than the version 2 limit of 64.
                totals: (0..100)
                    .map(|i| (format!("disk_read@{i}"), Some(i as f64)))
                    .collect(),
                etw_active: true,
                lost_events: 2,
                undecoded: 0,
                sample_ms: 4.0,
            })],
            io_totals: vec![(
                (7, 42),
                IoBytes {
                    total: [10, 20, 30, 40],
                    devices: vec![("disk@C:".into(), [10, 20])],
                },
            )],
        };
        let mut bytes = Vec::new();
        packet.write(&mut bytes).unwrap();
        let copy = Packet::read(&mut bytes.as_slice()).unwrap();
        assert_eq!(copy.io_totals, packet.io_totals);
        let p = &copy.frames[0].processes[0];
        let (device, engine) = &copy.frames[0].gpu_engines[0];
        assert_eq!(device, "gpu@1");
        assert_eq!(
            copy.frames[0].totals[99],
            ("disk_read@99".into(), Some(99.0))
        );
        assert_eq!(engine.processes[&7], 3.5);
        assert_eq!(engine.total, 3.5);
        assert_eq!((p.identity.pid, p.identity.created), (7, 42));
        assert_eq!(p.identity.name.as_ref(), "tést,\".exe");
        assert_eq!(p.gpu(), None);
        assert_eq!(
            (p.private_working_set(), p.shared()),
            (Some(999), Some(4096))
        );
        assert_eq!(p.io(), [1, 2, 3, 4]);
        assert_eq!(p.device_io()[1], ("net@Wi‑Fi".into(), [3, 4]));
        assert!(Packet::read(&mut &bytes[..bytes.len() - 1]).is_err());
        // Frames read by one viewer share the identity of a process.
        let mut identities = Identities::default();
        let [first, second] =
            [0, 1].map(|_| Packet::read_shared(&mut bytes.as_slice(), &mut identities).unwrap());
        assert!(Arc::ptr_eq(
            &first.frames[0].processes[0].identity,
            &second.frames[0].processes[0].identity
        ));
    }
}
