//! Windows WDDM telemetry, using the ABI from SDK d3dkmthk.h.
use crate::metrics::{MetricDescriptor, MetricProvider, MetricValue};
use std::{
    ffi::c_void,
    mem::size_of,
    time::{Duration, Instant},
};

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct AdapterInfo {
    handle: u32,
    luid: [u32; 2],
    sources: u32,
    precise_regions: i32,
}
#[repr(C)]
struct Enumeration {
    count: u32,
    adapters: *mut AdapterInfo,
}
#[repr(C)]
struct Query {
    handle: u32,
    kind: u32,
    data: *mut c_void,
    size: u32,
}
#[repr(C)]
#[derive(Default)]
struct Performance {
    physical_index: u32,
    memory_frequency: u64,
    max_memory_frequency: u64,
    max_memory_frequency_oc: u64,
    memory_bandwidth: u64,
    pcie_bandwidth: u64,
    fan_rpm: u32,
    power: u32,
    temperature: u32,
    power_state_override: u8,
}

struct Adapter {
    handle: u32,
    physical_count: u32,
    /// `(HighPart, LowPart)`, as in PDH instance names.
    luid: (i32, u32),
}
impl Adapter {
    fn query<T>(&self, kind: u32, data: &mut T) -> bool {
        let query = Query {
            handle: self.handle,
            kind,
            data: (data as *mut T).cast(),
            size: size_of::<T>() as u32,
        };
        unsafe { D3DKMTQueryAdapterInfo(&query) >= 0 }
    }
    fn temperatures(&self) -> impl Iterator<Item = f64> + '_ {
        (0..self.physical_count).filter_map(|physical_index| {
            let mut data = Performance {
                physical_index,
                ..Default::default()
            };
            self.query(62, &mut data)
                .then(|| Temperature::decode(data.temperature))
                .flatten()
        })
    }
}
impl Drop for Adapter {
    fn drop(&mut self) {
        unsafe {
            D3DKMTCloseAdapter(&self.handle);
        }
    }
}
struct AdapterCatalog;
impl AdapterCatalog {
    fn enumerate() -> Vec<Adapter> {
        // Bounded allocation also handles topology changes between queries.
        let mut entries = [AdapterInfo::default(); 64];
        let mut enumeration = Enumeration {
            count: entries.len() as u32,
            adapters: entries.as_mut_ptr(),
        };
        let status = unsafe { D3DKMTEnumAdapters2(&mut enumeration) };
        let mut adapters = Vec::new();
        // Own every returned handle immediately, including error/filtered paths.
        for entry in entries.into_iter().filter(|entry| entry.handle != 0) {
            let mut adapter = Adapter {
                handle: entry.handle,
                physical_count: 0,
                luid: (entry.luid[1] as i32, entry.luid[0]),
            };
            let mut kind = 0u32;
            let mut count = 0u32;
            if status >= 0
                && adapter.query(15, &mut kind)
                && kind & 4 == 0
                && adapter.query(30, &mut count)
                && (1..=64).contains(&count)
            {
                adapter.physical_count = count;
                adapters.push(adapter);
            }
        }
        adapters
    }
}
struct Temperature;
impl Temperature {
    fn decode(deci_celsius: u32) -> Option<f64> {
        // Zero is commonly returned by drivers without a temperature sensor.
        (1..=1500)
            .contains(&deci_celsius)
            .then_some(f64::from(deci_celsius) / 10.0)
    }
}

/// Hardware GPUs in WDDM order; a GPU device is tagged by its index in this list.
pub(crate) struct GpuAdapters;
impl GpuAdapters {
    /// LUIDs of the hardware adapters.
    pub fn list() -> Vec<(i32, u32)> {
        AdapterCatalog::enumerate().iter().map(|a| a.luid).collect()
    }
    /// Index named by a GPU device tag; a bare `gpu` is the main adapter.
    pub fn index(id: &super::devices::DeviceId) -> Option<usize> {
        match id.tag.as_deref() {
            Some(tag) => tag.parse().ok(),
            None => Some(Self::main()),
        }
    }
    /// A discrete GPU before an integrated one, which WDDM often lists first;
    /// among equals, the one with the most dedicated memory.
    pub fn main() -> usize {
        Self::list()
            .into_iter()
            .enumerate()
            .max_by_key(|(index, luid)| {
                let adapter = super::hardware::GpuAdapter::find(*luid);
                (
                    adapter.as_ref().is_some_and(|a| a.discrete),
                    adapter.map_or(0, |a| a.dedicated_bytes),
                    std::cmp::Reverse(*index),
                )
            })
            .map_or(0, |(index, _)| index)
    }
}

/// Temperature of one GPU: the maximum valid main-sensor reading of its physical GPUs.
pub(crate) struct GpuTemperatureProvider {
    descriptor: MetricDescriptor,
    index: Option<usize>,
    adapter: Option<Adapter>,
    refresh: Instant,
}
impl GpuTemperatureProvider {
    pub fn new(id: &super::devices::DeviceId) -> Self {
        Self {
            descriptor: MetricDescriptor::new(&id.reading("gpu_temperature"), "GPU TEMP", "°C"),
            index: GpuAdapters::index(id),
            adapter: None,
            refresh: Instant::now(),
        }
    }
}
impl MetricProvider for GpuTemperatureProvider {
    fn descriptor(&self) -> &MetricDescriptor {
        &self.descriptor
    }
    fn sample(&mut self) -> MetricValue {
        if Instant::now() >= self.refresh {
            self.adapter = self
                .index
                .and_then(|index| AdapterCatalog::enumerate().into_iter().nth(index));
            self.refresh = Instant::now() + Duration::from_secs(30);
        }
        self.adapter
            .iter()
            .flat_map(Adapter::temperatures)
            .reduce(f64::max)
            .map_or(MetricValue::Unavailable, MetricValue::Available)
    }
}
#[link(name = "gdi32")]
extern "system" {
    fn D3DKMTEnumAdapters2(data: *mut Enumeration) -> i32;
    fn D3DKMTQueryAdapterInfo(query: *const Query) -> i32;
    fn D3DKMTCloseAdapter(handle: *const u32) -> i32;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sdk_layouts_and_temperature_units() {
        assert_eq!(size_of::<AdapterInfo>(), 20);
        assert_eq!(size_of::<Enumeration>(), 16);
        assert_eq!(size_of::<Query>(), 24);
        assert_eq!(size_of::<Performance>(), 64);
        assert_eq!(std::mem::offset_of!(Performance, temperature), 56);
        assert_eq!(Temperature::decode(657), Some(65.7));
        for invalid in [0, 1501, u32::MAX] {
            assert_eq!(Temperature::decode(invalid), None);
        }
    }
}
