use crate::platform::abi::*;
use std::{
    ptr,
    sync::atomic::{AtomicU64, Ordering},
};

#[repr(C)]
struct Snapshot {
    sequence: AtomicU64,
    tick: AtomicU64,
    value: AtomicU64,
}

/// One-way shared snapshot: only the elevated collector maps it writable.
/// Atomic versioning prevents readers from combining two different samples.
pub(super) struct TemperatureChannel {
    _handle: Handle,
    view: *mut Snapshot,
}
impl TemperatureChannel {
    fn name(pid: u32) -> Vec<u16> {
        wide(&format!("Local\\TaskbarMetrics.CpuTemperature.v1.{pid}"))
    }
    pub fn create(pid: u32) -> Result<Self> {
        unsafe {
            let handle = Handle::new(CreateFileMappingW(
                -1isize as Raw,
                ptr::null_mut(),
                4,
                0,
                4096,
                Self::name(pid).as_ptr(),
            ))?;
            if GetLastError() == 183 {
                return Err(E_FAIL);
            }
            let view = MapViewOfFile(handle.0, 2, 0, 0, 4096).cast::<Snapshot>();
            if view.is_null() {
                return Err(last_error());
            }
            ptr::write(
                view,
                Snapshot {
                    sequence: AtomicU64::new(0),
                    tick: AtomicU64::new(0),
                    value: AtomicU64::new(f64::NAN.to_bits()),
                },
            );
            Ok(Self {
                _handle: handle,
                view,
            })
        }
    }
    pub fn open(pid: u32) -> Result<Self> {
        unsafe {
            let handle = Handle::new(OpenFileMappingW(4, 0, Self::name(pid).as_ptr()))?;
            let view = MapViewOfFile(handle.0, 4, 0, 0, 4096).cast::<Snapshot>();
            if view.is_null() {
                return Err(last_error());
            }
            Ok(Self {
                _handle: handle,
                view,
            })
        }
    }
    pub fn publish(&mut self, value: Option<f64>) {
        unsafe {
            let view = &*self.view;
            view.sequence.fetch_add(1, Ordering::SeqCst);
            view.value
                .store(value.unwrap_or(f64::NAN).to_bits(), Ordering::SeqCst);
            view.tick.store(GetTickCount64(), Ordering::SeqCst);
            view.sequence.fetch_add(1, Ordering::SeqCst);
        }
    }
    /// The collector published within 5 s, a reading or not. Explorer keeps the
    /// mapping open for a while after the collector ended, so its existence says less.
    pub fn fresh(&self) -> bool {
        unsafe {
            let tick = (*self.view).tick.load(Ordering::SeqCst);
            let now = GetTickCount64();
            tick != 0 && now >= tick && now - tick <= 5000
        }
    }
    pub fn read(&self) -> Option<f64> {
        unsafe {
            let view = &*self.view;
            let first = view.sequence.load(Ordering::SeqCst);
            let value = f64::from_bits(view.value.load(Ordering::SeqCst));
            let tick = view.tick.load(Ordering::SeqCst);
            let last = view.sequence.load(Ordering::SeqCst);
            Snapshot::validate(first, last, value, tick, GetTickCount64())
        }
    }
}
impl Snapshot {
    fn validate(first: u64, last: u64, value: f64, tick: u64, now: u64) -> Option<f64> {
        (first != 0
            && first == last
            && first & 1 == 0
            && now >= tick
            && now - tick <= 5000
            && value.is_finite()
            && (0.0..=125.0).contains(&value))
        .then_some(value)
    }
}
impl Drop for TemperatureChannel {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(self.view.cast());
        }
    }
}
#[link(name = "kernel32")]
extern "system" {
    fn CreateFileMappingW(
        file: Raw,
        attributes: Raw,
        protection: u32,
        high: u32,
        low: u32,
        name: *const u16,
    ) -> Raw;
    fn OpenFileMappingW(access: u32, inherit: i32, name: *const u16) -> Raw;
    fn MapViewOfFile(mapping: Raw, access: u32, high: u32, low: u32, size: usize) -> Raw;
    fn UnmapViewOfFile(view: Raw) -> i32;
    fn GetTickCount64() -> u64;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_mapping_publishes_to_read_only_client_and_rejects_second_writer() {
        let pid = unsafe { GetCurrentProcessId() };
        let mut writer = TemperatureChannel::create(pid).unwrap();
        let reader = TemperatureChannel::open(pid).unwrap();
        assert_eq!(reader.read(), None);
        assert!(TemperatureChannel::create(pid).is_err());
        writer.publish(Some(62.125));
        assert_eq!(reader.read(), Some(62.125));
        writer.publish(None);
        assert_eq!(reader.read(), None);
    }
    #[test]
    fn rejects_torn_stale_invalid_and_future_samples() {
        assert_eq!(Snapshot::validate(2, 2, 62.125, 100, 110), Some(62.125));
        for (first, last, value, tick, now) in [
            (0, 0, 60.0, 100, 110),
            (1, 1, 60.0, 100, 110),
            (2, 4, 60.0, 100, 110),
            (2, 2, f64::NAN, 100, 110),
            (2, 2, 126.0, 100, 110),
            (2, 2, 60.0, 100, 5101),
            (2, 2, 60.0, 111, 110),
        ] {
            assert_eq!(Snapshot::validate(first, last, value, tick, now), None);
        }
    }
}
