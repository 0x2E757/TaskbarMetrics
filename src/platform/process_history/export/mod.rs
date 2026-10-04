//! `TaskbarMetrics.exe --dump`: the recorded history for scripts and agents. The
//! recorder's export pipe only sends the whole history, so the choice is made here.

mod metric_id;
mod query;
mod ranking;
mod raw_csv;
mod totals;

pub use query::DumpQuery;

use super::{ipc::HistoryServer, store::Frame, wire::Packet};
use ranking::ProcessRanking;
use raw_csv::RawCsv;
use std::{
    borrow::Cow,
    io::{self, BufWriter, Write},
    sync::Arc,
};
use totals::TotalsTable;

pub struct HistoryDump {
    query: DumpQuery,
}

impl HistoryDump {
    pub fn new(query: DumpQuery) -> Self {
        Self { query }
    }

    /// Writes the history of the recorder for the Explorer `pid`.
    pub fn run(&self, pid: u32) -> Result<(), String> {
        let packet = HistoryServer::read(pid).map_err(Self::unavailable)?;
        let mut out: Box<dyn Write> = match &self.query.output {
            Some(path) => Box::new(BufWriter::new(
                std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?,
            )),
            None => Box::new(BufWriter::new(io::stdout().lock())),
        };
        self.write(&packet, &mut out)
            .and_then(|_| out.flush())
            .map_err(|e| format!("Writing the history: {e}"))
    }

    fn write(&self, packet: &Packet, out: &mut dyn Write) -> io::Result<()> {
        if self.query.raw {
            return RawCsv::write(packet, out);
        }
        let frames = Self::window(&packet.frames, self.query.seconds);
        let Some(last) = frames.last() else {
            return writeln!(out, "# No history yet: the recorder has just started.");
        };
        writeln!(
            out,
            "# Taskbar Metrics history: {} frames of 500 ms, the last {} s up to timestamp_ms={}; recorder status: {}",
            frames.len(),
            self.query.seconds,
            last.bucket * 500,
            packet.status
        )?;
        let totals = TotalsTable::new(frames, &self.query.metrics);
        if self.query.series {
            totals.series(out)?;
        } else {
            totals.summary(out)?;
        }
        let Some(processes) = &self.query.processes else {
            return Ok(());
        };
        writeln!(out)?;
        if !packet.enabled {
            return writeln!(out, "# No processes: process history is off (process_monitoring=false in %LOCALAPPDATA%\\Taskbar Metrics\\taskbar-metrics.conf).");
        }
        if !frames.iter().any(|f| f.etw_active) {
            writeln!(out, "# Disk and network bytes of processes are missing: the recorder needs administrator rights for ETW.")?;
        }
        writeln!(
            out,
            "# The top {} processes by {} over the window; CPU and GPU are percent of the whole computer, bytes are sums.",
            processes.count,
            processes.by.name()
        )?;
        ProcessRanking::new(frames).write(processes, out)
    }

    /// The frames of the last `seconds`, up to the newest frame.
    fn window(frames: &[Arc<Frame>], seconds: u64) -> &[Arc<Frame>] {
        let Some(last) = frames.last() else {
            return frames;
        };
        let first = (last.bucket + 1).saturating_sub(seconds * 2);
        &frames[frames.partition_point(|f| f.bucket < first)..]
    }

    fn unavailable(error: io::Error) -> String {
        match error.raw_os_error() {
            Some(2) => "The history recorder is not running for this taskbar: run TaskbarMetrics.exe without arguments, then try again.".into(),
            _ if error.kind() == io::ErrorKind::InvalidData => "The running history recorder is of another build: use the TaskbarMetrics.exe beside it, or run this TaskbarMetrics.exe without arguments to take over.".into(),
            _ => format!("Reading the history: {error}"),
        }
    }
}

/// CSV fields.
struct Csv;

impl Csv {
    /// `text`, quoted when it holds a separator, a quote or a line break.
    fn field(text: &str) -> Cow<'_, str> {
        if text.contains([',', '"', '\r', '\n']) {
            Cow::Owned(format!("\"{}\"", text.replace('"', "\"\"")))
        } else {
            Cow::Borrowed(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(bucket: u64) -> Arc<Frame> {
        Arc::new(Frame {
            gpu_engines: Vec::new(),
            bucket,
            elapsed_ms: 500.0,
            processes: Vec::new(),
            totals: vec![("cpu".into(), Some(bucket as f64))],
            etw_active: false,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 1.0,
        })
    }

    fn dump(args: &[&str], enabled: bool) -> String {
        let args: Vec<_> = args.iter().map(|arg| arg.to_string()).collect();
        let packet = Packet {
            enabled,
            status: "running".into(),
            frames: (1..=10).map(frame).collect(),
            io_totals: Vec::new(),
        };
        let mut out = Vec::new();
        HistoryDump::new(DumpQuery::parse(&args).unwrap())
            .write(&packet, &mut out)
            .unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn keeps_the_last_seconds() {
        let frames: Vec<_> = (1..=10).map(frame).collect();
        let window = HistoryDump::window(&frames, 2);
        assert_eq!(
            window.iter().map(|f| f.bucket).collect::<Vec<_>>(),
            [7, 8, 9, 10]
        );
        assert_eq!(HistoryDump::window(&frames, 300).len(), 10);
        assert!(HistoryDump::window(&[], 60).is_empty());
    }

    #[test]
    fn explains_what_it_cannot_show() {
        let text = dump(&["--last", "1", "--processes", "3"], false);
        assert!(text.starts_with("# Taskbar Metrics history: 2 frames of 500 ms, the last 1 s up to timestamp_ms=5000; recorder status: running\n"));
        assert!(text.contains("cpu,percent,9.500,10.000,10.000\n"));
        assert!(text.ends_with("# No processes: process history is off (process_monitoring=false in %LOCALAPPDATA%\\Taskbar Metrics\\taskbar-metrics.conf).\n"));
        let text = dump(&["--processes", "3"], true);
        assert!(text.contains("# Disk and network bytes of processes are missing"));
        assert!(text.ends_with("net_send_bytes\n"));
        assert!(dump(&["--raw"], true).starts_with("# history_v3 enabled=true frames=10 "));
    }

    #[test]
    fn quotes_fields_that_need_it() {
        assert_eq!(Csv::field("disk_read@C:"), "disk_read@C:");
        assert_eq!(Csv::field("a,\"b\""), "\"a,\"\"b\"\"\"");
    }
}
