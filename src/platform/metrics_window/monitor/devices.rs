use super::{
    locale::Language,
    model::{Device, Resource},
};
use crate::platform::{
    devices::{DiskName, NetworkAdapter},
    hardware::{DriveBus, GpuAdapter, GpuMemory, Processor},
    memory_modules::MemoryModules,
    system_activity::{CpuFrequency, PageFile, SystemActivity},
    wifi::Wlan,
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

/// Live values beside the chart that the recorder does not keep, so a past moment
/// has none of them.
#[derive(Clone, Copy, Default)]
pub struct Facts {
    /// Current processor frequency, MHz.
    pub frequency: Option<f64>,
    pub activity: Option<SystemActivity>,
    pub page_file: Option<PageFile>,
    /// Wi‑Fi signal strength, dBm.
    pub signal: Option<i32>,
}

/// Hardware named in the section headers ("AMD Ryzen 7 9800X3D · 8 cores ·
/// 16 threads", "2 × 16 GB DDR5-6000", "NVMe", "Intel(R) Wi‑Fi 6 AX201",
/// "Radeon RX 7800 · discrete") and the dedicated GPU memory in use. Slow system
/// queries are repeated at most every ten seconds.
pub struct Devices {
    checked: Option<Instant>,
    buses: HashMap<String, Option<&'static str>>,
    network: Vec<NetworkAdapter>,
    /// Read once: the processor does not change while the window is open.
    processor: Option<Processor>,
    /// «2 × 16 GB DDR5-6000», read once like the processor.
    modules: String,
    /// Adapter of the shown GPU's busiest engine and its DXGI description.
    gpu: Option<((i32, u32), Option<GpuAdapter>)>,
    memory: GpuMemory,
    used: Option<f64>,
    frequency: CpuFrequency,
    /// None without the WLAN service.
    wlan: Option<Wlan>,
    facts: Facts,
}
impl Devices {
    const PERIOD: Duration = Duration::from_secs(10);
    pub fn new() -> Self {
        Self {
            checked: None,
            buses: HashMap::new(),
            network: Vec::new(),
            processor: Processor::read(),
            modules: MemoryModules::summary(&MemoryModules::read()),
            gpu: None,
            memory: GpuMemory::new(),
            used: None,
            frequency: CpuFrequency::default(),
            wlan: Wlan::open(),
            facts: Facts::default(),
        }
    }
    /// `engine` is the PDH instance of the shown GPU's busiest engine, if known.
    pub fn refresh(&mut self, device: &Device, engine: Option<&str>) {
        let luid = engine.and_then(GpuAdapter::luid);
        if self.gpu.as_ref().map(|(known, _)| *known) != luid {
            self.gpu = luid.map(|luid| (luid, GpuAdapter::find(luid)));
        }
        if let Some((luid, _)) = self.gpu {
            self.used = self.memory.used(luid);
        }
        if let (Resource::Disk, Some(tag), false) =
            (device.resource, &device.id.tag, device.network_drive())
        {
            self.buses
                .entry(tag.clone())
                .or_insert_with(|| DriveBus::of(tag));
        }
        if self.checked.is_none_or(|at| at.elapsed() >= Self::PERIOD) {
            self.checked = Some(Instant::now());
            self.network = NetworkAdapter::list();
        }
        self.sample(device);
    }
    /// Live values of the shown device's kind.
    fn sample(&mut self, device: &Device) {
        match device.resource {
            Resource::Cpu => {
                self.facts.frequency = self.frequency.sample();
                self.facts.activity = SystemActivity::read();
            }
            Resource::Ram => self.facts.page_file = PageFile::read(),
            Resource::Net => {
                self.facts.signal = NetworkAdapter::find(&self.network, &device.id)
                    .filter(|a| a.kind == "Wi‑Fi")
                    .zip(self.wlan.as_ref())
                    .and_then(|(a, wlan)| wlan.rssi(&a.description));
            }
            Resource::Gpu | Resource::Disk => {}
        }
    }
    pub fn facts(&self) -> Facts {
        self.facts
    }
    /// Header subtitle of the CPU, GPU, disk and network pages.
    pub fn subtitle(&self, device: &Device, language: Language) -> String {
        match device.resource {
            Resource::Cpu => self.processor.as_ref().map_or_else(String::new, |cpu| {
                let name = match cpu.packages {
                    1 => cpu.name.clone(),
                    sockets => format!("{sockets} × {}", cpu.name),
                };
                [
                    name,
                    language.cores(cpu.cores),
                    language.threads(cpu.threads),
                ]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" · ")
            }),
            Resource::Gpu => match self.gpu.as_ref().and_then(|(_, gpu)| gpu.as_ref()) {
                Some(gpu) => format!(
                    "{} · {}",
                    gpu.name,
                    language.text(if gpu.discrete {
                        "discrete"
                    } else {
                        "integrated"
                    })
                ),
                None => String::new(),
            },
            // "NVMe · Drives C:, D:": the bus, then the volumes the title does not
            // name. The data sources are in the legend and "How it’s measured".
            Resource::Disk => [
                device
                    .id
                    .tag
                    .as_ref()
                    .and_then(|tag| *self.buses.get(tag)?)
                    .map(str::to_owned),
                device.disk.as_ref().and_then(|disk| disk.remote.clone()),
                device
                    .disk
                    .as_ref()
                    .and_then(|disk| Self::volumes(disk, language)),
            ]
            .into_iter()
            .flatten()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" · "),
            Resource::Net => NetworkAdapter::find(&self.network, &device.id)
                .map(|adapter| adapter.description.clone())
                .unwrap_or_default(),
            Resource::Ram => self.modules.clone(),
        }
    }
    /// "Drive C:" or "Drives C:, D:", as Explorer names drive letters; nothing for a
    /// disk without them.
    fn volumes(disk: &DiskName, language: Language) -> Option<String> {
        let label = match disk.volumes.len() {
            0 => return None,
            1 => "Drive {v}",
            _ => "Drives {v}",
        };
        Some(language.text(label).replace("{v}", &disk.letters()))
    }
    /// Dedicated memory `(in use, total)` in bytes; usage is live only.
    pub fn gpu_memory(&self) -> Option<(Option<f64>, f64)> {
        let (_, gpu) = self.gpu.as_ref()?;
        Some((self.used, gpu.as_ref()?.dedicated_bytes as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn volumes_are_named_in_the_singular_or_plural() {
        let disk = |volumes: &[&str]| DiskName {
            media: Some("SSD"),
            remote: None,
            volumes: volumes.iter().map(|v| (*v).into()).collect(),
        };
        let english = |volumes| Devices::volumes(&disk(volumes), Language::English);
        assert_eq!(english(&["C:"]).as_deref(), Some("Drive C:"));
        assert_eq!(english(&["D:", "E:"]).as_deref(), Some("Drives D:, E:"));
        assert_eq!(english(&[]), None);
        let russian = Language::Russian
            .text("Drives {v}")
            .replace("{v}", "D:, E:");
        assert_eq!(
            Devices::volumes(&disk(&["D:", "E:"]), Language::Russian),
            Some(russian)
        );
    }
}
