use super::{
    devices::{DeviceId, NetworkDrive},
    gpu_temperature::GpuTemperatureProvider,
    providers::{
        CpuProvider, DiskThroughputProvider, GpuProvider, NetworkProvider, NetworkTraffic,
        RamProvider, ShareThroughputProvider,
    },
};
use crate::{
    application::MonitoringService,
    config::Settings,
    metrics::{MetricProvider, MetricRegistry},
    presentation::CompactFormatter,
};

/// The only place that knows concrete providers. Extend this catalog to add a metric.
pub struct Composition;
impl Composition {
    /// Taskbar tiles: a bare kind (`disk`) keeps its unsuffixed reading ids.
    pub fn monitor(settings: &Settings) -> Result<MonitoringService, String> {
        let devices = settings
            .metrics
            .iter()
            .map(|id| DeviceId::parse(id).ok_or_else(|| format!("Unknown metric: {id}")))
            .collect::<Result<Vec<_>, _>>()?;
        Self::build(&devices, false)
    }
    /// Recorder totals for the given devices.
    pub fn history_monitor(devices: &[DeviceId]) -> Result<MonitoringService, String> {
        Self::build(devices, true)
    }
    fn build(devices: &[DeviceId], detailed: bool) -> Result<MonitoringService, String> {
        let mut registry = MetricRegistry::default();
        for id in devices {
            let providers: Vec<Box<dyn MetricProvider>> = match id.kind.as_str() {
                "cpu" => vec![
                    Box::new(CpuProvider::new()),
                    Box::new(super::temperature::CpuTemperatureProvider::new()),
                ],
                // The recorder derives GPU totals and process shares from one query.
                "gpu" if detailed => vec![Box::new(GpuTemperatureProvider::new(id))],
                "gpu" => vec![
                    Box::new(GpuProvider::new(id)),
                    Box::new(GpuTemperatureProvider::new(id)),
                ],
                "ram" => vec![Box::new(RamProvider::new())],
                // Read and write in MB/s, as the tile shows them like the network.
                "disk" if id.tag.as_deref().and_then(NetworkDrive::of).is_some() => vec![
                    Box::new(ShareThroughputProvider::new(id, false)),
                    Box::new(ShareThroughputProvider::new(id, true)),
                ],
                "disk" => vec![
                    Box::new(DiskThroughputProvider::new(id, false)),
                    Box::new(DiskThroughputProvider::new(id, true)),
                ],
                "net" => {
                    let traffic = NetworkTraffic::shared();
                    vec![
                        Box::new(NetworkProvider::new(id, false, traffic.clone())),
                        Box::new(NetworkProvider::new(id, true, traffic)),
                    ]
                }
                _ => return Err(format!("Unknown metric: {id}")),
            };
            for provider in providers {
                registry.register(provider)?;
            }
        }
        Ok(MonitoringService::new(registry, Box::new(CompactFormatter)))
    }
}
