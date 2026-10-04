use super::{
    super::store::Frame,
    query::{ProcessQuery, Ranking},
    Csv,
};
use std::{
    collections::HashMap,
    io::{self, Write},
    sync::Arc,
};

/// What one process used over a window of frames.
struct ProcessUsage<'a> {
    pid: u32,
    name: &'a str,
    /// CPU and GPU percent added up over the frames.
    cpu: f64,
    cpu_maximum: f64,
    gpu: f64,
    private_working_set_maximum: u64,
    /// Disk read, disk write, net receive and net send bytes.
    io: [u64; 4],
}

impl ProcessUsage<'_> {
    fn amount(&self, by: Ranking) -> f64 {
        match by {
            Ranking::Cpu => self.cpu,
            Ranking::Gpu => self.gpu,
            Ranking::Ram => self.private_working_set_maximum as f64,
            Ranking::Disk => (self.io[0] + self.io[1]) as f64,
            Ranking::Net => (self.io[2] + self.io[3]) as f64,
        }
    }
}

/// The processes that used the most of a resource over a window of frames.
pub(super) struct ProcessRanking<'a> {
    frames: usize,
    usage: Vec<ProcessUsage<'a>>,
}

impl<'a> ProcessRanking<'a> {
    pub fn new(frames: &'a [Arc<Frame>]) -> Self {
        let mut usage: Vec<ProcessUsage> = Vec::new();
        // A process is its PID and creation time: a PID can be reused within the window.
        let mut rows = HashMap::new();
        for sample in frames.iter().flat_map(|f| &f.processes) {
            let identity = &sample.identity;
            let row = *rows
                .entry((identity.pid, identity.created))
                .or_insert_with(|| {
                    usage.push(ProcessUsage {
                        pid: identity.pid,
                        name: &identity.name,
                        cpu: 0.0,
                        cpu_maximum: 0.0,
                        gpu: 0.0,
                        private_working_set_maximum: 0,
                        io: [0; 4],
                    });
                    usage.len() - 1
                });
            let row = &mut usage[row];
            let cpu = sample.cpu().unwrap_or(0.0);
            row.cpu += cpu;
            row.cpu_maximum = row.cpu_maximum.max(cpu);
            row.gpu += sample.gpu().unwrap_or(0.0);
            row.private_working_set_maximum = row
                .private_working_set_maximum
                .max(sample.private_working_set().unwrap_or(0));
            for (total, bytes) in row.io.iter_mut().zip(sample.io()) {
                *total = total.saturating_add(bytes);
            }
        }
        Self {
            frames: frames.len(),
            usage,
        }
    }

    /// The first `query.count` processes that used any of the resource, most first.
    pub fn write(mut self, query: &ProcessQuery, out: &mut dyn Write) -> io::Result<()> {
        self.usage.retain(|row| row.amount(query.by) > 0.0);
        self.usage
            .sort_by(|a, b| b.amount(query.by).total_cmp(&a.amount(query.by)));
        writeln!(out, "pid,name,cpu_percent_average,cpu_percent_maximum,gpu_percent_average,private_working_set_bytes_maximum,disk_read_bytes,disk_write_bytes,net_receive_bytes,net_send_bytes")?;
        let frames = self.frames.max(1) as f64;
        for row in self.usage.iter().take(query.count) {
            writeln!(
                out,
                "{},{},{:.3},{:.3},{:.3},{},{},{},{},{}",
                row.pid,
                Csv::field(row.name),
                row.cpu / frames,
                row.cpu_maximum,
                row.gpu / frames,
                row.private_working_set_maximum,
                row.io[0],
                row.io[1],
                row.io[2],
                row.io[3]
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::process_history::store::{Identity, IoBytes, Sample};

    fn sample(pid: u32, name: &str, cpu: f64, disk_read: u64) -> Sample {
        Sample::new(
            Arc::new(Identity {
                pid,
                created: 1,
                name: name.into(),
            }),
            Some(cpu),
            None,
            [Some(4096), None, None],
            Sample::boxed_io(IoBytes {
                total: [disk_read, 0, 0, 0],
                devices: Vec::new(),
            }),
        )
    }

    fn frame(bucket: u64, processes: Vec<Sample>) -> Arc<Frame> {
        Arc::new(Frame {
            gpu_engines: Vec::new(),
            bucket,
            elapsed_ms: 500.0,
            processes,
            totals: Vec::new(),
            etw_active: true,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 1.0,
        })
    }

    fn ranked(frames: &[Arc<Frame>], count: usize, by: Ranking) -> Vec<String> {
        let mut out = Vec::new();
        ProcessRanking::new(frames)
            .write(&ProcessQuery { count, by }, &mut out)
            .unwrap();
        String::from_utf8(out)
            .unwrap()
            .lines()
            .skip(1)
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn ranks_processes_over_the_window() {
        let frames = [
            frame(
                1,
                vec![
                    sample(1, "idle.exe", 0.0, 0),
                    sample(2, "a,b.exe", 40.0, 10),
                ],
            ),
            frame(
                2,
                vec![
                    sample(3, "copy.exe", 10.0, 900),
                    sample(2, "a,b.exe", 20.0, 0),
                ],
            ),
        ];
        assert_eq!(
            ranked(&frames, 5, Ranking::Cpu),
            [
                "2,\"a,b.exe\",30.000,40.000,0.000,4096,10,0,0,0",
                "3,copy.exe,5.000,10.000,0.000,4096,900,0,0,0",
            ]
        );
        assert_eq!(
            ranked(&frames, 1, Ranking::Disk)[0],
            "3,copy.exe,5.000,10.000,0.000,4096,900,0,0,0"
        );
        assert!(ranked(&frames, 5, Ranking::Net).is_empty());
    }
}
