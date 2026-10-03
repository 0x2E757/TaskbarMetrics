use super::abi::wide;
use crate::metrics::{GpuEngineAggregator, MetricValue};
use std::ptr;

const MORE_DATA: u32 = 0x800007d2;
const DOUBLE: u32 = 0x200;
const NOCAP100: u32 = 0x8000;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CounterValue {
    status: u32,
    padding: u32,
    value: f64,
}

impl CounterValue {
    fn valid(&self) -> bool {
        self.status <= 1 && self.value.is_finite() && self.value >= 0.0
    }
}

#[repr(C)]
struct CounterItem {
    name: *const u16,
    value: CounterValue,
}

#[link(name = "pdh")]
extern "system" {
    fn PdhOpenQueryW(source: *const u16, context: usize, query: *mut isize) -> u32;
    fn PdhAddEnglishCounterW(
        query: isize,
        path: *const u16,
        context: usize,
        counter: *mut isize,
    ) -> u32;
    fn PdhCollectQueryData(query: isize) -> u32;
    fn PdhGetFormattedCounterValue(
        counter: isize,
        format: u32,
        kind: *mut u32,
        value: *mut CounterValue,
    ) -> u32;
    fn PdhGetFormattedCounterArrayW(
        counter: isize,
        format: u32,
        size: *mut u32,
        count: *mut u32,
        items: *mut CounterItem,
    ) -> u32;
    fn PdhCloseQuery(query: isize) -> u32;
}

/// Each provider owns an independent PDH query: failures cannot poison peers.
/// Counters of one object that are always read together may share a query.
pub struct PdhCounter {
    paths: Vec<&'static str>,
    query: isize,
    counters: Vec<isize>,
    primed: bool,
    retry_at: std::time::Instant,
}

impl PdhCounter {
    pub fn new(path: &'static str) -> Self {
        Self::several(&[path])
    }
    /// Counters collected by one query; read them with `collected`.
    pub fn several(paths: &[&'static str]) -> Self {
        Self {
            paths: paths.to_vec(),
            query: 0,
            counters: Vec::new(),
            primed: false,
            retry_at: std::time::Instant::now(),
        }
    }

    fn open(&mut self) -> bool {
        if self.query != 0 {
            return true;
        }
        if std::time::Instant::now() < self.retry_at {
            return false;
        }
        self.retry_at = std::time::Instant::now() + std::time::Duration::from_secs(30);
        // SAFETY: out pointers reference initialized storage and path is NUL terminated.
        unsafe {
            if PdhOpenQueryW(ptr::null(), 0, &mut self.query) != 0 {
                self.query = 0;
                return false;
            }
            for path in &self.paths {
                let mut counter = 0;
                if PdhAddEnglishCounterW(self.query, wide(path).as_ptr(), 0, &mut counter) != 0 {
                    self.close();
                    return false;
                }
                self.counters.push(counter);
            }
        }
        true
    }

    fn close(&mut self) {
        if self.query != 0 {
            unsafe {
                PdhCloseQuery(self.query);
            }
        }
        self.query = 0;
        self.counters.clear();
        self.primed = false;
    }

    /// Takes a new sample of every counter of the query.
    pub fn collect(&mut self) -> Result<(), MetricValue> {
        if !self.open() {
            return Err(MetricValue::Unavailable);
        }
        if unsafe { PdhCollectQueryData(self.query) } != 0 {
            self.close();
            return Err(MetricValue::Unavailable);
        }
        if !std::mem::replace(&mut self.primed, true) {
            return Err(MetricValue::WarmingUp);
        }
        Ok(())
    }

    pub fn scalar(&mut self) -> MetricValue {
        match self.scalar_raw() {
            MetricValue::Available(value) => MetricValue::percent(value),
            other => other,
        }
    }
    pub fn scalar_raw(&mut self) -> MetricValue {
        if let Err(value) = self.collect() {
            return value;
        }
        let mut value = CounterValue::default();
        if unsafe {
            PdhGetFormattedCounterValue(self.counters[0], DOUBLE, ptr::null_mut(), &mut value)
        } == 0
            && value.valid()
        {
            MetricValue::Available(value.value)
        } else {
            MetricValue::Unavailable
        }
    }

    /// Value of the first instance of a wildcard counter accepted by `matches`.
    pub fn instance(&mut self, matches: impl Fn(&str) -> bool) -> MetricValue {
        match self.entries() {
            Ok(values) => values
                .into_iter()
                .find(|(name, _)| matches(name))
                .map_or(MetricValue::Unavailable, |(_, value)| {
                    MetricValue::Available(value)
                }),
            Err(value) => value,
        }
    }

    /// Busiest engine of the GPU with `luid`, from `\GPU Engine(*)`.
    pub fn gpu(&mut self, luid: (i32, u32)) -> MetricValue {
        let values = match self.entries() {
            Ok(values) => values,
            Err(value) => return value,
        };
        let mut aggregator = GpuEngineAggregator::default();
        for (name, value) in values {
            if super::hardware::GpuAdapter::luid(&name) == Some(luid) {
                aggregator.add(&name, value);
            }
        }
        aggregator.value()
    }
    /// Value of the single-instance counter at `index` as of the last `collect`.
    pub fn collected_value(&self, index: usize) -> Option<f64> {
        let counter = *self.counters.get(index)?;
        let mut value = CounterValue::default();
        (unsafe {
            PdhGetFormattedCounterValue(counter, DOUBLE | NOCAP100, ptr::null_mut(), &mut value)
        } == 0
            && value.valid())
        .then_some(value.value)
    }
    pub(crate) fn entries(&mut self) -> Result<Vec<(String, f64)>, MetricValue> {
        self.collect()?;
        self.collected(0)
    }
    /// Instances of the counter at `index` as of the last `collect`.
    pub fn collected(&self, index: usize) -> Result<Vec<(String, f64)>, MetricValue> {
        let counter = *self.counters.get(index).ok_or(MetricValue::Unavailable)?;
        let (mut size, mut count) = (0, 0);
        let mut status = unsafe {
            PdhGetFormattedCounterArrayW(
                counter,
                DOUBLE | NOCAP100,
                &mut size,
                &mut count,
                ptr::null_mut(),
            )
        };
        for _ in 0..4 {
            // Bound memory consumption even if the counter API returns an invalid size.
            if status != MORE_DATA || size == 0 || size > 64 * 1024 * 1024 {
                return Err(MetricValue::Unavailable);
            }
            let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
            let items = buffer.as_mut_ptr().cast::<CounterItem>();
            status = unsafe {
                PdhGetFormattedCounterArrayW(
                    counter,
                    DOUBLE | NOCAP100,
                    &mut size,
                    &mut count,
                    items,
                )
            };
            if status == MORE_DATA {
                continue;
            }
            if status != 0 || count as usize > buffer.len() * 8 / std::mem::size_of::<CounterItem>()
            {
                return Err(MetricValue::Unavailable);
            }
            let mut result = Vec::with_capacity(count as usize);
            for i in 0..count as usize {
                // Buffer is aligned for CounterItem and the count was checked above.
                let item = unsafe { &*items.add(i) };
                if item.name.is_null() || !item.value.valid() {
                    continue;
                }
                let start = item.name as usize;
                let base = buffer.as_ptr() as usize;
                let end = base + buffer.len() * 8;
                if start < base || start >= end || !start.is_multiple_of(2) {
                    continue;
                }
                let units = unsafe { std::slice::from_raw_parts(item.name, (end - start) / 2) };
                if let Some(length) = units.iter().position(|n| *n == 0) {
                    result.push((String::from_utf16_lossy(&units[..length]), item.value.value));
                }
            }
            return Ok(result);
        }
        Err(MetricValue::Unavailable)
    }
}

impl Drop for PdhCounter {
    fn drop(&mut self) {
        self.close();
    }
}
