use super::{
    memory::{MemoryRows, SystemMemoryCounters},
    shared_pages::SharedPages,
    snapshot::{Process, SnapshotReader},
    store::{Frame, Identity, Sample},
    trace::{IoBuckets, IoTrace},
};
use crate::{
    application::MonitoringService,
    metrics::{GpuEngineAggregator, GpuEngineLoad, GpuEngineUsage, MetricValue},
    platform::{
        abi::*,
        devices::{Binding, DeviceId, IoDevices},
        gpu_temperature::GpuAdapters,
        hardware::GpuAdapter,
        pdh::PdhCounter,
        providers::PhysicalMemory,
    },
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

/// Busiest engine of every recorded GPU and per-process GPU use, from one query.
struct GpuSampler {
    counter: PdhCounter,
    adapters: Vec<Binding<(i32, u32)>>,
}
/// One GPU sample: engines by device id, the busiest engine load of each process,
/// query success.
type GpuSample = (
    Vec<(String, GpuEngineUsage)>,
    HashMap<u32, MetricValue>,
    bool,
);
impl GpuSampler {
    fn new(devices: &[DeviceId]) -> Self {
        let mut counter = PdhCounter::new(r"\GPU Engine(*)\Utilization Percentage");
        let _ = counter.entries();
        Self {
            counter,
            adapters: devices
                .iter()
                .filter(|d| d.kind == "gpu")
                .map(|d| Binding::new(d.clone()))
                .collect(),
        }
    }
    fn sample(&mut self) -> GpuSample {
        let entries = self.counter.entries().ok();
        let available = entries.is_some();
        let entries = entries.unwrap_or_default();
        let mut loads: HashMap<u32, GpuEngineLoad> = HashMap::new();
        for (name, value) in &entries {
            if let Some(pid) = name
                .strip_prefix("pid_")
                .and_then(|s| s.split('_').next())
                .and_then(|s| s.parse::<u32>().ok())
            {
                loads.entry(pid).or_default().add(name, *value);
            }
        }
        let processes = loads
            .into_iter()
            .map(|(pid, load)| (pid, load.value()))
            .collect();
        let mut engines = Vec::new();
        for adapter in &mut self.adapters {
            let Some(luid) =
                adapter.get(|id| GpuAdapters::list().get(GpuAdapters::index(id)?).copied())
            else {
                continue;
            };
            let mut aggregate = GpuEngineAggregator::default();
            for (name, value) in &entries {
                if GpuAdapter::luid(name) == Some(luid) {
                    aggregate.add(name, *value);
                }
            }
            if let Some(engine) = aggregate.busiest() {
                engines.push((adapter.id.to_string(), engine));
            }
        }
        (engines, processes, available)
    }
}

/// Per-process CPU, memory and I/O attribution; exists only while process
/// monitoring is on. ETW needs elevation; without it I/O stays unattributed.
struct ProcessSampler {
    reader: SnapshotReader,
    previous: HashMap<(u32, u64), (u64, Arc<Identity>)>,
    last: Instant,
    processors: f64,
    trace: Option<IoTrace>,
    trace_error: Option<i32>,
    /// Disk instances for attributing I/O events to disks, re-read every 10 s.
    disks: PdhCounter,
    mapped: Option<Instant>,
    /// Shared pages of each process while a window shows the history.
    shared: SharedPages,
}
impl ProcessSampler {
    fn new() -> Result<Self> {
        let mut reader = SnapshotReader::default();
        let (processes, threads) = reader.read()?;
        let (trace, trace_error) = match IoTrace::start(threads) {
            Ok(t) => (Some(t), None),
            Err(e) => (None, Some(e)),
        };
        let previous = processes
            .into_iter()
            .map(|p| ((p.pid, p.created), (p.cpu_ticks, Self::identity(&p))))
            .collect();
        Ok(Self {
            reader,
            previous,
            last: Instant::now(),
            processors: unsafe { GetActiveProcessorCount(0xffff) }.max(1) as f64,
            trace,
            trace_error,
            disks: {
                let mut disks = PdhCounter::new(r"\PhysicalDisk(*)\Current Disk Queue Length");
                let _ = disks.entries();
                disks
            },
            mapped: None,
            shared: SharedPages::default(),
        })
    }
    fn identity(p: &Process) -> Arc<Identity> {
        Arc::new(Identity {
            pid: p.pid,
            created: p.created,
            name: p.name.clone().into(),
        })
    }
    /// `shared`: also scan the working sets for shared pages, once a second.
    fn sample(
        &mut self,
        gpu: &HashMap<u32, MetricValue>,
        gpu_available: bool,
        shared: bool,
    ) -> Result<(Vec<Sample>, IoBuckets, u32, u64)> {
        let begin = Instant::now();
        let elapsed = begin.duration_since(self.last).as_secs_f64();
        // Disks and adapters come and go; events map to the current set.
        if let Some(trace) = &self.trace {
            if self
                .mapped
                .is_none_or(|at| at.elapsed() >= Duration::from_secs(10))
            {
                self.mapped = Some(begin);
                trace.set_devices(IoDevices::current(&mut self.disks));
            }
        }
        let (processes, threads) = self.reader.read()?;
        if shared {
            self.shared.update(&processes);
        } else {
            self.shared.clear();
        }
        let mut next = HashMap::with_capacity(processes.len());
        // Room for the few rows added later (GPU only, late I/O of exited or
        // unattributed processes): growing would double the frame's 5-minute footprint.
        let mut samples = Vec::with_capacity(processes.len() + 8);
        for p in processes {
            let key = (p.pid, p.created);
            let old = self.previous.get(&key);
            let identity = old
                .map(|(_, i)| i.clone())
                .unwrap_or_else(|| Self::identity(&p));
            let cpu = old
                .and_then(|(ticks, _)| p.cpu_ticks.checked_sub(*ticks))
                .map(|ticks| {
                    (ticks as f64 / 10_000_000.0 / elapsed / self.processors * 100.0)
                        .clamp(0.0, 100.0)
                });
            let gpu = match gpu.get(&p.pid) {
                Some(MetricValue::Available(value)) => Some(*value),
                _ if gpu_available => Some(0.0),
                _ => None,
            };
            next.insert(key, (p.cpu_ticks, identity.clone()));
            let mut sample = Sample::new(
                identity,
                cpu,
                gpu,
                [
                    Some(p.private_working_set),
                    Some(p.working_set),
                    Some(p.private_bytes),
                ],
                None,
            );
            sample.set_shared(self.shared.of(&key).map(|shared| shared.data));
            samples.push(sample);
        }
        self.previous = next;
        for (pid, load) in gpu {
            if !samples.iter().any(|sample| sample.identity.pid == *pid) {
                if let MetricValue::Available(value) = *load {
                    samples.push(Sample::new(
                        Arc::new(Identity {
                            pid: *pid,
                            created: 0,
                            name: format!("PID {pid} (GPU only)").into(),
                        }),
                        None,
                        Some(value),
                        [None, None, None],
                        None,
                    ));
                }
            }
        }
        self.last = begin;
        let (events, lost, undecoded) = self
            .trace
            .as_ref()
            .map(|t| t.drain(threads))
            .unwrap_or_default();
        Ok((samples, events, lost, undecoded))
    }
    fn etw_active(&self) -> bool {
        self.trace.as_ref().is_some_and(|trace| trace.is_running())
    }
}

/// Totals of the recorded devices, always; processes while monitoring is on.
pub(super) struct Collector {
    pub devices: Vec<DeviceId>,
    totals: MonitoringService,
    gpu: GpuSampler,
    processes: Option<ProcessSampler>,
    retry: Instant,
    process_error: Option<i32>,
    last: Instant,
    memory: SystemMemoryCounters,
    rows: MemoryRows,
    /// A window shows the history: attribute shared pages to processes.
    pub shared: bool,
}
impl Collector {
    pub fn new(devices: Vec<DeviceId>) -> Result<Self> {
        let mut totals = crate::platform::composition::Composition::history_monitor(&devices)
            .map_err(|_| E_FAIL)?;
        let _ = totals.sample_readings();
        Ok(Self {
            gpu: GpuSampler::new(&devices),
            devices,
            totals,
            processes: None,
            retry: Instant::now(),
            process_error: None,
            last: Instant::now(),
            memory: SystemMemoryCounters::new(),
            rows: MemoryRows::new(),
            shared: false,
        })
    }
    /// Starts or drops per-process sampling; a failed start is retried after 30 s.
    pub fn monitor_processes(&mut self, enabled: bool) {
        if !enabled {
            self.processes = None;
            self.process_error = None;
        } else if self.processes.is_none() && Instant::now() >= self.retry {
            self.retry = Instant::now() + Duration::from_secs(30);
            match ProcessSampler::new() {
                Ok(sampler) => {
                    self.processes = Some(sampler);
                    self.process_error = None;
                }
                Err(error) => self.process_error = Some(error),
            }
        }
    }
    pub fn status(&self, enabled: bool) -> String {
        match (&self.processes, self.process_error) {
            _ if !enabled => "totals only".into(),
            (_, Some(error)) => format!("Sampling unavailable 0x{error:08X}"),
            (Some(sampler), _) => sampler
                .trace_error
                .map(|e| format!("ETW unavailable 0x{e:08X}"))
                .unwrap_or_else(|| "running".into()),
            (None, None) => "starting".into(),
        }
    }
    pub fn sample(&mut self, bucket: u64) -> Result<(Frame, IoBuckets)> {
        let begin = Instant::now();
        let elapsed = begin.duration_since(self.last).as_secs_f64();
        self.last = begin;
        let (engines, gpu_processes, gpu_available) = self.gpu.sample();
        let memory = self.memory.sample();
        let (samples, events, lost, undecoded) = match &mut self.processes {
            Some(sampler) => match sampler.sample(&gpu_processes, gpu_available, self.shared) {
                Ok(mut result) => {
                    if let Some(memory) = &memory {
                        self.rows
                            .append(&mut result.0, memory, sampler.shared.readonly_floor());
                    }
                    result
                }
                Err(error) => {
                    self.processes = None;
                    self.process_error = Some(error);
                    Default::default()
                }
            },
            None => Default::default(),
        };
        let mut totals: Vec<_> = self
            .totals
            .sample_readings()
            .into_iter()
            .map(|r| {
                (
                    r.descriptor.id,
                    match r.value {
                        MetricValue::Available(v) => Some(v),
                        _ => None,
                    },
                )
            })
            .collect();
        for device in self.devices.iter().filter(|d| d.kind == "gpu") {
            let id = device.to_string();
            let total = engines
                .iter()
                .find(|(engine, _)| *engine == id)
                .map(|(_, usage)| usage.total);
            totals.push((device.reading("gpu"), total.filter(|_| gpu_available)));
        }
        // In bytes: the figure the memory rows add up to.
        let used = memory.map(|m| m.used).or_else(PhysicalMemory::used_bytes);
        totals.push(("ram_used".into(), used.map(|bytes| bytes as f64)));
        Ok((
            Frame {
                gpu_engines: engines,
                bucket,
                elapsed_ms: elapsed * 1000.0,
                processes: samples,
                totals,
                etw_active: self
                    .processes
                    .as_ref()
                    .is_some_and(ProcessSampler::etw_active),
                lost_events: lost,
                undecoded,
                sample_ms: begin.elapsed().as_secs_f64() * 1000.0,
            },
            events,
        ))
    }
}
#[link(name = "kernel32")]
extern "system" {
    fn GetActiveProcessorCount(group: u16) -> u32;
}
