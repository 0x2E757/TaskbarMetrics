use std::collections::HashSet;

/// A stable identifier is separate from the label shown to the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricDescriptor {
    pub id: String,
    pub label: String,
    pub unit: String,
}

impl MetricDescriptor {
    pub fn new(id: &str, label: &str, unit: &str) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            unit: unit.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MetricValue {
    Available(f64),
    WarmingUp,
    Unavailable,
}

impl MetricValue {
    pub fn percent(value: f64) -> Self {
        if value.is_finite() && value >= 0.0 {
            Self::Available(value.min(100.0))
        } else {
            Self::Unavailable
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricReading {
    pub descriptor: MetricDescriptor,
    pub value: MetricValue,
}

/// Providers own their resources and run exclusively on the sampling thread.
/// Expected failures must be returned as Unavailable, never as a fake zero.
pub trait MetricProvider {
    fn descriptor(&self) -> &MetricDescriptor;
    fn sample(&mut self) -> MetricValue;
}

#[derive(Default)]
pub struct MetricRegistry {
    providers: Vec<Box<dyn MetricProvider>>,
    identifiers: HashSet<String>,
}

impl MetricRegistry {
    pub fn register(&mut self, provider: Box<dyn MetricProvider>) -> Result<(), String> {
        let id = &provider.descriptor().id;
        if id.is_empty() || !self.identifiers.insert(id.clone()) {
            return Err(format!("Empty or duplicate metric identifier: {id}"));
        }
        self.providers.push(provider);
        Ok(())
    }

    pub fn sample(&mut self) -> Vec<MetricReading> {
        self.providers
            .iter_mut()
            .map(|provider| {
                let descriptor = provider.descriptor().clone();
                // A faulty extension does not prevent other providers from sampling.
                let value =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| provider.sample()))
                        .unwrap_or(MetricValue::Unavailable);
                let value = match value {
                    MetricValue::Available(n) if !n.is_finite() => MetricValue::Unavailable,
                    other => other,
                };
                MetricReading { descriptor, value }
            })
            .collect()
    }
}

/// PDH reports GPU engine usage per process. Sum the same physical engine,
/// then take the busiest engine across adapters (not the sum of all engines).
#[derive(Default)]
pub struct GpuEngineAggregator {
    engines: std::collections::HashMap<String, f64>,
    processes: std::collections::HashMap<String, std::collections::BTreeMap<u32, f64>>,
    labels: std::collections::HashMap<String, String>,
}

/// Additive contributions to one physical engine, from one PDH query interval.
#[derive(Clone, Debug)]
pub struct GpuEngineUsage {
    pub name: String,
    pub label: String,
    pub total: f64,
    pub raw_total: f64,
    pub processes: std::collections::BTreeMap<u32, f64>,
}

/// Load of the busiest engine only, as `GpuEngineAggregator::value`, without the
/// labels and per-process shares: the recorder needs it for every process.
#[derive(Default)]
pub struct GpuEngineLoad<'a> {
    engines: std::collections::HashMap<&'a str, f64>,
}
impl<'a> GpuEngineLoad<'a> {
    pub fn add(&mut self, instance: &'a str, value: f64) {
        if let Some((key, _)) = GpuEngineAggregator::engine(instance, value) {
            *self.engines.entry(key).or_default() += value;
        }
    }
    pub fn value(&self) -> MetricValue {
        self.engines
            .values()
            .copied()
            .reduce(f64::max)
            .map(MetricValue::percent)
            .unwrap_or(MetricValue::Unavailable)
    }
}

impl GpuEngineAggregator {
    /// The physical engine (`luid_…_phys_…_eng_…`) of a counter instance and the
    /// rest of its name after `_engtype_`; none for invalid values or instances.
    fn engine(instance: &str, value: f64) -> Option<(&str, &str)> {
        if !value.is_finite() || value < 0.0 {
            return None;
        }
        let tail = &instance[instance.find("luid_")?..];
        let end = tail.find("_engtype_")?;
        let key = &tail[..end];
        (key.contains("_phys_") && key.contains("_eng_"))
            .then(|| (key, &tail[end + "_engtype_".len()..]))
    }
    pub fn add(&mut self, instance: &str, value: f64) {
        if let Some((key, engine_type)) = Self::engine(instance, value) {
            // Hundreds of instances share a few engines: the key is copied only
            // when an engine is first seen.
            match self.engines.get_mut(key) {
                Some(total) => *total += value,
                None => {
                    self.engines.insert(key.to_owned(), value);
                    self.labels.insert(
                        key.to_owned(),
                        engine_type.split('#').next().unwrap_or_default().to_owned(),
                    );
                }
            }
            if let Some(pid) = instance
                .strip_prefix("pid_")
                .and_then(|s| s.split('_').next())
                .and_then(|s| s.parse().ok())
            {
                if !self.processes.contains_key(key) {
                    self.processes.insert(key.to_owned(), Default::default());
                }
                if let Some(shares) = self.processes.get_mut(key) {
                    *shares.entry(pid).or_default() += value;
                }
            }
        }
    }
    pub fn busiest(&self) -> Option<GpuEngineUsage> {
        let (name, raw_total) = self
            .engines
            .iter()
            .max_by(|(ka, a), (kb, b)| a.total_cmp(b).then_with(|| kb.cmp(ka)))?;
        let scale = if *raw_total > 100.0 {
            100.0 / raw_total
        } else {
            1.0
        };
        Some(GpuEngineUsage {
            name: name.clone(),
            label: self.labels.get(name).cloned().unwrap_or_default(),
            total: raw_total.min(100.0),
            raw_total: *raw_total,
            processes: self
                .processes
                .get(name)
                .into_iter()
                .flatten()
                .map(|(pid, value)| (*pid, value * scale))
                .collect(),
        })
    }

    pub fn value(&self) -> MetricValue {
        self.engines
            .values()
            .copied()
            .reduce(f64::max)
            .map(MetricValue::percent)
            .unwrap_or(MetricValue::Unavailable)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A source with a fixed value, or one that panics like a faulty extension.
    pub(crate) struct FakeProvider {
        descriptor: MetricDescriptor,
        value: MetricValue,
        panic: bool,
    }
    impl FakeProvider {
        pub(crate) fn new(id: &str, value: MetricValue) -> Box<Self> {
            Box::new(Self {
                descriptor: MetricDescriptor::new(id, id, "%"),
                value,
                panic: false,
            })
        }
    }
    impl MetricProvider for FakeProvider {
        fn descriptor(&self) -> &MetricDescriptor {
            &self.descriptor
        }
        fn sample(&mut self) -> MetricValue {
            assert!(!self.panic, "test extension failure");
            self.value.clone()
        }
    }

    #[test]
    fn provider_failure_is_isolated_and_invalid_numbers_do_not_escape() {
        let mut registry = MetricRegistry::default();
        let mut broken = FakeProvider::new("broken", MetricValue::Available(1.0));
        broken.panic = true;
        registry.register(broken).unwrap();
        registry
            .register(FakeProvider::new("nan", MetricValue::Available(f64::NAN)))
            .unwrap();
        registry
            .register(FakeProvider::new("working", MetricValue::Available(75.0)))
            .unwrap();
        let samples = registry.sample();
        assert_eq!(samples[0].value, MetricValue::Unavailable);
        assert_eq!(samples[1].value, MetricValue::Unavailable);
        assert_eq!(samples[2].value, MetricValue::Available(75.0));
    }

    #[test]
    fn duplicate_metric_ids_are_rejected() {
        let mut registry = MetricRegistry::default();
        registry
            .register(FakeProvider::new("cpu", MetricValue::Unavailable))
            .unwrap();
        assert!(registry
            .register(FakeProvider::new("cpu", MetricValue::Unavailable))
            .is_err());
        assert_eq!(registry.sample().len(), 1);
    }

    #[test]
    fn gpu_sums_processes_on_same_engine_but_keeps_adapters_and_engines_separate() {
        let mut gpu = GpuEngineAggregator::default();
        let mut load = GpuEngineLoad::default();
        assert_eq!(gpu.value(), MetricValue::Unavailable);
        assert_eq!(load.value(), MetricValue::Unavailable);
        for (name, value) in [
            ("pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_3D", 30.0),
            ("pid_2_luid_0x0_0x1_phys_0_eng_0_engtype_3D", 40.0),
            ("pid_3_luid_0x0_0x1_phys_0_eng_1_engtype_3D", 60.0),
            ("pid_4_luid_0x0_0x2_phys_0_eng_0_engtype_3D", 50.0),
            ("pid_5_luid_0x0_0x1_phys_1_eng_0_engtype_3D", 65.0),
            ("invalid", 100.0),
            ("pid_6_luid_0x0_0x1_phys_0_eng_0_engtype_3D", f64::NAN),
        ] {
            gpu.add(name, value);
            load.add(name, value);
        }
        assert_eq!(gpu.value(), MetricValue::Available(70.0));
        // The busiest-load shortcut agrees with the full aggregation.
        assert_eq!(load.value(), gpu.value());
        let engine = gpu.busiest().unwrap();
        assert_eq!(engine.total, 70.0);
        assert_eq!(engine.processes.values().sum::<f64>(), 70.0);
        assert_eq!(engine.processes.get(&1), Some(&30.0));
        assert!(!engine.processes.contains_key(&3));
        assert!(!engine.processes.contains_key(&4));
        gpu.add("pid_9_luid_0x0_0x1_phys_0_eng_0_engtype_3D", 80.0);
        assert_eq!(gpu.value(), MetricValue::Available(100.0));
        let engine = gpu.busiest().unwrap();
        assert_eq!(engine.raw_total, 150.0);
        assert!((engine.processes.values().sum::<f64>() - 100.0).abs() < 1e-9);
        assert!((engine.processes[&2] / engine.processes[&1] - 4.0 / 3.0).abs() < 1e-9);
    }
}
