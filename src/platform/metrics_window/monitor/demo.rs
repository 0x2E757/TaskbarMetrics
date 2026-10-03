//! Deterministic synthetic history that reproduces the design artboards
//! (`--demo <scenario>`); also used by `--verify-monitor`.
use super::{chart::WINDOW, model::MIB};
use crate::{
    metrics::GpuEngineUsage,
    platform::{
        devices::DeviceId,
        process_history::{
            memory::{MemoryRows, SystemMemory},
            store::{Frame, Identity, IoBytes, Sample},
        },
    },
};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scenario {
    /// Frozen moment, hovered process and four pins (one exited).
    History,
    /// Live, no pins.
    Live,
    /// Live GPU: a 21 s interruption and temperature above 100 °C.
    Gpu,
    /// First 42 seconds after start.
    Empty,
    /// Background process monitoring is off; totals only.
    Off,
    /// The settings section.
    Settings,
}
impl Scenario {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "history" => Self::History,
            "live" => Self::Live,
            "gpu" => Self::Gpu,
            "empty" => Self::Empty,
            "off" => Self::Off,
            "settings" => Self::Settings,
            _ => return None,
        })
    }
}

struct Process {
    pid: u32,
    name: &'static str,
    cpu: f64,
    gpu: f64,
    memory: f64,
    io: [f64; 4],
}
const PROCESSES: &[Process] = &[
    Process {
        pid: 27564,
        name: "Code.exe",
        cpu: 6.0,
        gpu: 0.3,
        memory: 610.0,
        io: [0.04, 0.11, 0.02, 0.01],
    },
    Process {
        pid: 14292,
        name: "vmware-vmx.exe",
        cpu: 5.5,
        gpu: 0.0,
        memory: 1240.0,
        io: [0.2, 0.3, 0.05, 0.02],
    },
    Process {
        pid: 16548,
        name: "steam.exe",
        cpu: 0.5,
        gpu: 0.1,
        memory: 180.0,
        io: [0.0, 0.0, 0.02, 0.0],
    },
    Process {
        pid: 11968,
        name: "KeeWeb.exe",
        cpu: 1.2,
        gpu: 0.0,
        memory: 150.0,
        io: [0.0, 0.0, 0.0, 0.0],
    },
    Process {
        pid: 26312,
        name: "dwm.exe",
        cpu: 2.4,
        gpu: 1.9,
        memory: 120.0,
        io: [0.0, 0.0, 0.0, 0.0],
    },
    Process {
        pid: 4860,
        name: "MsMpEng.exe",
        cpu: 1.8,
        gpu: 0.0,
        memory: 300.0,
        io: [0.42, 0.08, 0.0, 0.0],
    },
    Process {
        pid: 18616,
        name: "AMDRSServ.exe",
        cpu: 1.8,
        gpu: 29.0,
        memory: 90.0,
        io: [0.0, 0.0, 0.0, 0.0],
    },
    Process {
        pid: 5912,
        name: "msedge.exe",
        cpu: 1.6,
        gpu: 1.8,
        memory: 420.0,
        io: [0.1, 0.5, 0.13, 0.02],
    },
    Process {
        pid: 22416,
        name: "ms-teams.exe",
        cpu: 0.8,
        gpu: 0.2,
        memory: 380.0,
        io: [0.0, 0.1, 0.26, 0.22],
    },
    Process {
        pid: 6680,
        name: "paintdotnet.exe",
        cpu: 0.6,
        gpu: 0.1,
        memory: 240.0,
        io: [0.42, 8.3, 0.0, 0.0],
    },
    Process {
        pid: 4,
        name: "System",
        cpu: 0.75,
        gpu: 0.0,
        memory: 20.0,
        io: [0.1, 0.53, 0.0, 0.0],
    },
    Process {
        pid: 14812,
        name: "explorer.exe",
        cpu: 0.5,
        gpu: 0.4,
        memory: 160.0,
        io: [0.12, 0.02, 0.0, 0.0],
    },
    Process {
        pid: 9120,
        name: "audiodg.exe",
        cpu: 0.3,
        gpu: 0.0,
        memory: 30.0,
        io: [0.0, 0.0, 0.0, 0.0],
    },
    Process {
        pid: 1552,
        name: "svchost.exe",
        cpu: 0.27,
        gpu: 0.0,
        memory: 40.0,
        io: [0.06, 0.21, 0.03, 0.01],
    },
    Process {
        pid: 7304,
        name: "SearchIndexer.exe",
        cpu: 0.2,
        gpu: 0.0,
        memory: 70.0,
        io: [0.42, 0.08, 0.0, 0.0],
    },
    Process {
        pid: 7788,
        name: "RuntimeBroker.exe",
        cpu: 0.05,
        gpu: 0.0,
        memory: 25.0,
        io: [0.0, 0.0, 0.0, 0.0],
    },
];
/// Memory outside processes: 44 % of 32 GB in use, as the demo RAM tile shows.
const MEMORY: SystemMemory = SystemMemory {
    used: 15_118_284_554,
    nonpaged_pool: 900 << 20,
    paged_pool: 1150 << 20,
    drivers: 62 << 20,
    modified: 120 << 20,
    file_cache: 740 << 20,
};
/// The identity a demo process has in every scenario.
pub fn identity(name: &str) -> Arc<Identity> {
    let pid = PROCESSES
        .iter()
        .find(|p| p.name == name)
        .map_or(0, |p| p.pid);
    Arc::new(Identity {
        pid,
        created: 1_000 + pid as u64,
        name: name.into(),
    })
}
/// Two of each device kind that can repeat, as in a desktop with two GPUs,
/// two disks and wired plus wireless network.
pub fn devices() -> Vec<DeviceId> {
    [
        "cpu",
        "gpu@0",
        "gpu@1",
        "ram",
        "disk@C:",
        "disk@D:",
        "net@Ethernet",
        "net@Wi‑Fi",
    ]
    .into_iter()
    .filter_map(DeviceId::parse)
    .collect()
}
/// Pins of each scenario, by process name.
pub fn pins(scenario: Scenario, resource: super::model::Resource) -> &'static [&'static str] {
    use super::model::Resource;
    match (scenario, resource) {
        (Scenario::Gpu, _) => &["AMDRSServ.exe"],
        (Scenario::Empty | Scenario::Live, Resource::Net) => &["msedge.exe", "ms-teams.exe"],
        (Scenario::Empty | Scenario::Live, _) => &[],
        (Scenario::History, Resource::Disk) => &["MsMpEng.exe", "paintdotnet.exe", "steam.exe"],
        (Scenario::Off, _) => &["Code.exe", "vmware-vmx.exe", "steam.exe"],
        _ => &["Code.exe", "vmware-vmx.exe", "steam.exe", "KeeWeb.exe"],
    }
}

/// Linear congruential noise: stable between runs, so screenshots are comparable.
struct Noise(u64);
impl Noise {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

pub struct DemoHistory;
impl DemoHistory {
    /// I/O since a recorder start an hour ago: twelve times that of the 5 minutes.
    pub fn io_totals(frames: &[Arc<Frame>]) -> HashMap<(u32, u64), IoBytes> {
        let mut totals: HashMap<_, IoBytes> = HashMap::new();
        for process in frames.iter().flat_map(|f| &f.processes) {
            if let Some(io) = &process.io_bytes {
                totals
                    .entry((process.identity.pid, process.identity.created))
                    .or_default()
                    .merge(io);
            }
        }
        for io in totals.values_mut() {
            let window = io.clone();
            for _ in 0..11 {
                io.merge(&window);
            }
        }
        totals
    }
    pub fn frames(scenario: Scenario, end: u64) -> Vec<Arc<Frame>> {
        let mut noise = Noise(7);
        let count = if scenario == Scenario::Empty {
            84
        } else {
            WINDOW
        };
        let identities: Vec<_> = PROCESSES
            .iter()
            .map(|p| {
                Arc::new(Identity {
                    pid: p.pid,
                    created: 1_000 + p.pid as u64,
                    name: p.name.into(),
                })
            })
            .chain((0..229).map(|i| {
                Arc::new(Identity {
                    pid: 30_000 + i,
                    created: 40_000 + i as u64,
                    name: "svchost.exe".into(),
                })
            }))
            .collect();
        (0..count)
            .filter(|i| !(scenario == Scenario::Gpu && (480..522).contains(i)))
            .map(|i| {
                let bucket = end - (count - 1 - i);
                let t = i as f64 / WINDOW as f64;
                let burst = |center: f64, width: f64| (-((t - center) / width).powi(2)).exp();
                let spike = burst(0.83, 0.004) * 2.2 + burst(0.45, 0.01);
                let cpu = (24.0 + noise.next() * 10.0 + spike * 12.0).min(100.0);
                let warm = 59.0 + noise.next() * 1.5 + spike * 6.0;
                let hot = if scenario == Scenario::Gpu {
                    75.0 + ((t - 0.72) * 400.0).clamp(0.0, 31.0)
                } else {
                    warm + 4.0
                };
                let mut processes: Vec<Sample> = identities
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| {
                        scenario != Scenario::Off
                            && !(PROCESSES
                                .get(*index)
                                .is_some_and(|p| p.name == "KeeWeb.exe")
                                && t > 0.5)
                    })
                    .map(|(index, identity)| {
                        let p = PROCESSES.get(index);
                        let jitter = 0.5 + noise.next();
                        let wave =
                            if p.is_some_and(|p| p.name == "Code.exe" || p.name == "MsMpEng.exe") {
                                1.0 + spike * 2.5
                            } else {
                                1.0
                            };
                        let io = p
                            .map_or([0.0; 4], |p| p.io)
                            .map(|v| (v * jitter * wave * 500_000.0) as u64);
                        let memory = p.map_or(9.0, |p| p.memory) * MIB;
                        let mut sample = Sample::new(
                            identity.clone(),
                            Some(p.map_or(0.0015, |p| p.cpu) * jitter * wave),
                            Some(p.map_or(0.0, |p| p.gpu)),
                            [Some(memory as u64), None, None],
                            // Most disk I/O on the system disk; all traffic over Ethernet.
                            Sample::boxed_io(IoBytes {
                                total: io,
                                devices: vec![
                                    ("disk@C:".into(), [io[0] * 9 / 10, io[1] * 9 / 10]),
                                    ("disk@D:".into(), [io[0] / 10, io[1] / 10]),
                                    ("net@Ethernet".into(), [io[2], io[3]]),
                                ],
                            }),
                        );
                        // The guest memory of a virtual machine is a shared section.
                        let shared = if p.is_some_and(|p| p.name == "vmware-vmx.exe") {
                            3072.0 * MIB
                        } else {
                            memory * 0.3
                        };
                        sample.set_shared(Some(shared as u64));
                        sample
                    })
                    .collect();
                if !processes.is_empty() {
                    MemoryRows::new().append(&mut processes, &MEMORY, 300 << 20);
                }
                let gpu_total = 30.0 + noise.next() * 3.0;
                let idle = 2.0 + noise.next();
                Arc::new(Frame {
                    gpu_engines: vec![
                        (
                            "gpu@0".into(),
                            GpuEngineUsage {
                                name: "luid_0x0_0x1_phys_0_eng_4".into(),
                                label: "Video Codec".into(),
                                total: gpu_total,
                                raw_total: gpu_total,
                                processes: PROCESSES
                                    .iter()
                                    .map(|p| {
                                        (
                                            p.pid,
                                            if p.name == "AMDRSServ.exe" {
                                                gpu_total - 1.0
                                            } else {
                                                p.gpu * 0.03
                                            },
                                        )
                                    })
                                    .collect(),
                            },
                        ),
                        (
                            "gpu@1".into(),
                            GpuEngineUsage {
                                name: "luid_0x0_0x2_phys_0_eng_0".into(),
                                label: "3D".into(),
                                total: idle,
                                raw_total: idle,
                                processes: [(26312, idle)].into(),
                            },
                        ),
                    ],
                    bucket,
                    elapsed_ms: 500.0,
                    processes,
                    totals: vec![
                        ("cpu".into(), Some(cpu)),
                        ("cpu_temperature".into(), Some(warm)),
                        ("gpu@0".into(), Some(gpu_total)),
                        ("gpu_temperature@0".into(), Some(hot)),
                        ("gpu@1".into(), Some(idle)),
                        ("gpu_temperature@1".into(), Some(41.0 + idle)),
                        ("ram".into(), Some(44.0)),
                        ("ram_used".into(), Some(MEMORY.used as f64)),
                        ("disk@C:".into(), Some(38.0)),
                        (
                            "disk_read@C:".into(),
                            Some(0.6 + noise.next() * 0.8 + spike * 12.0),
                        ),
                        ("disk_write@C:".into(), Some(0.5 + noise.next() * 0.6)),
                        ("disk@D:".into(), Some(3.0)),
                        ("disk_read@D:".into(), Some(0.05 + noise.next() * 0.1)),
                        ("disk_write@D:".into(), Some(0.02)),
                        (
                            "net_down@Ethernet".into(),
                            Some(0.25 + noise.next() * 0.2 + spike * 0.6),
                        ),
                        ("net_up@Ethernet".into(), Some(0.2 + noise.next() * 0.1)),
                        ("net_down@Wi‑Fi".into(), Some(0.01 + noise.next() * 0.02)),
                        ("net_up@Wi‑Fi".into(), Some(0.005)),
                    ],
                    etw_active: true,
                    lost_events: 0,
                    undecoded: 0,
                    sample_ms: 1.0,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scenarios_cover_gaps_short_history_and_disabled_monitoring() {
        let gpu = DemoHistory::frames(Scenario::Gpu, 10_000);
        assert_eq!(gpu.len(), 558);
        assert!(gpu.windows(2).any(|p| p[1].bucket - p[0].bucket == 43));
        assert_eq!(DemoHistory::frames(Scenario::Empty, 10_000).len(), 84);
        assert!(DemoHistory::frames(Scenario::Off, 10_000)[0]
            .processes
            .is_empty());
        let history = DemoHistory::frames(Scenario::History, 10_000);
        assert_eq!(history.last().map(|f| f.bucket), Some(10_000));
        assert_eq!(history[0].processes.len(), PROCESSES.len() + 229 + 6);
        // The processes leave room for the shared-memory row.
        let rest = history[0].processes.last().unwrap();
        assert!(rest.identity.pid == MemoryRows::PID && rest.shared() > Some(0));
    }
}
