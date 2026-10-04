use super::super::{store::RETENTION, wire::Packet};
use std::io::{self, Write};

/// `--raw`: every frame of the recorder with every process, as `history_v3` CSV.
pub(super) struct RawCsv;

impl RawCsv {
    pub fn write(packet: &Packet, out: &mut dyn Write) -> io::Result<()> {
        writeln!(
            out,
            "# history_v3 enabled={} frames={} interval_ms=500 retention_ms={} status={}",
            packet.enabled,
            packet.frames.len(),
            RETENTION * 500,
            packet.status
        )?;
        writeln!(out, "timestamp_ms,elapsed_ms,pid,created_filetime,name,cpu_percent,gpu_percent,private_working_set_bytes,working_set_bytes,private_bytes,shared_share_bytes,disk_read_bytes,disk_write_bytes,net_receive_bytes,net_send_bytes,etw_active,events_lost,undecoded_events,sample_ms")?;
        for f in &packet.frames {
            for (device, engine) in &f.gpu_engines {
                writeln!(
                    out,
                    "# gpu_engine,{},{device},{},{},{},{}",
                    f.bucket * 500,
                    engine.name,
                    engine.label,
                    engine.total,
                    engine.raw_total
                )?;
                for (pid, value) in &engine.processes {
                    writeln!(
                        out,
                        "# gpu_share,{},{device},{},{}",
                        f.bucket * 500,
                        pid,
                        value
                    )?;
                }
            }
            for (id, value) in &f.totals {
                writeln!(
                    out,
                    "# total,{},{},{}",
                    f.bucket * 500,
                    id,
                    value.map(|v| v.to_string()).unwrap_or_default()
                )?;
            }
            for p in &f.processes {
                let name = p
                    .identity
                    .name
                    .replace('"', "\"\"")
                    .replace(['\r', '\n'], " ");
                let cpu = p.cpu().map(|n| format!("{n:.4}")).unwrap_or_default();
                let gpu = p.gpu().map(|n| format!("{n:.4}")).unwrap_or_default();
                writeln!(
                    out,
                    "{},{:.3},{},{},\"{}\",{},{},{},{},{},{},{},{},{},{},{},{},{},{:.3}",
                    f.bucket * 500,
                    f.elapsed_ms,
                    p.identity.pid,
                    p.identity.created,
                    name,
                    cpu,
                    gpu,
                    p.private_working_set()
                        .map(|n| n.to_string())
                        .unwrap_or_default(),
                    p.working_set().map(|n| n.to_string()).unwrap_or_default(),
                    p.private_bytes().map(|n| n.to_string()).unwrap_or_default(),
                    p.shared().map(|n| n.to_string()).unwrap_or_default(),
                    p.io()[0],
                    p.io()[1],
                    p.io()[2],
                    p.io()[3],
                    f.etw_active,
                    f.lost_events,
                    f.undecoded,
                    f.sample_ms
                )?;
                // The row's I/O split by disk and adapter.
                for (device, [first, second]) in p.device_io() {
                    writeln!(
                        out,
                        "# device_io,{},{},{device},{first},{second}",
                        f.bucket * 500,
                        p.identity.pid
                    )?;
                }
            }
        }
        Ok(())
    }
}
