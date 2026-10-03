use super::channel::TemperatureChannel;
use crate::{
    metrics::{MetricDescriptor, MetricProvider, MetricValue},
    platform::abi::*,
};

use std::ptr;

/// Read-only, nonblocking adapter. Explorer never opens the hardware driver.
pub(crate) struct CpuTemperatureProvider {
    descriptor: MetricDescriptor,
    channel: Option<TemperatureChannel>,
}

impl CpuTemperatureProvider {
    pub fn new() -> Self {
        Self {
            descriptor: MetricDescriptor::new("cpu_temperature", "CPU Tctl", "°C"),
            channel: None,
        }
    }
}

impl MetricProvider for CpuTemperatureProvider {
    fn descriptor(&self) -> &MetricDescriptor {
        &self.descriptor
    }

    fn sample(&mut self) -> MetricValue {
        if self.channel.is_none() {
            let mut pid = 0;
            unsafe {
                GetWindowThreadProcessId(
                    FindWindowW(wide("Shell_TrayWnd").as_ptr(), ptr::null()),
                    &mut pid,
                );
            }
            if pid != 0 {
                self.channel = TemperatureChannel::open(pid).ok();
            }
        }
        match self.channel.as_ref().and_then(TemperatureChannel::read) {
            Some(value) => MetricValue::Available(value),
            None => {
                self.channel = None;
                MetricValue::Unavailable
            }
        }
    }
}
