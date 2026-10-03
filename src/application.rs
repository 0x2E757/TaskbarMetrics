use crate::{metrics::MetricRegistry, presentation::MetricFormatter};

pub trait MetricSink {
    fn publish(&self, text: &str);
    fn publish_readings(&self, readings: &[crate::metrics::MetricReading], text: &str) {
        let _ = readings;
        self.publish(text);
    }
}

/// Composition of data sources, presentation, and output. No Windows API calls.
pub struct MonitoringService {
    metrics: MetricRegistry,
    formatter: Box<dyn MetricFormatter>,
}

impl MonitoringService {
    pub fn new(metrics: MetricRegistry, formatter: Box<dyn MetricFormatter>) -> Self {
        Self { metrics, formatter }
    }

    pub fn tick(&mut self, sink: &dyn MetricSink) {
        let readings = self.metrics.sample();
        sink.publish_readings(&readings, &self.formatter.format(&readings));
    }

    pub fn sample_text(&mut self) -> String {
        self.formatter.format(&self.metrics.sample())
    }
    pub fn sample_readings(&mut self) -> Vec<crate::metrics::MetricReading> {
        self.metrics.sample()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{tests::FakeProvider, MetricRegistry, MetricValue};
    use crate::presentation::CompactFormatter;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Capture(RefCell<String>);
    impl MetricSink for Capture {
        fn publish(&self, text: &str) {
            self.0.replace(text.into());
        }
    }

    #[test]
    fn arbitrary_metric_reaches_output_without_changes_to_monitor_or_formatter() {
        let mut registry = MetricRegistry::default();
        registry
            .register(FakeProvider::new(
                "new-metric",
                MetricValue::Available(42.0),
            ))
            .unwrap();
        registry
            .register(FakeProvider::new("missing", MetricValue::Unavailable))
            .unwrap();
        registry
            .register(FakeProvider::new("warmup", MetricValue::WarmingUp))
            .unwrap();
        let mut service = MonitoringService::new(registry, Box::new(CompactFormatter));
        let output = Capture::default();
        service.tick(&output);
        assert_eq!(
            &*output.0.borrow(),
            "new-metric 42%   missing --   warmup --"
        );
    }
}
