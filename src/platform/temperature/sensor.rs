use super::{amd, intel, pawnio::PawnModule, zhaoxin};
use crate::platform::abi::*;
use std::{
    arch::x86_64::{__cpuid, CpuidResult},
    path::{Path, PathBuf},
    ptr,
};

pub(super) trait TemperatureSensor {
    fn name(&self) -> &'static str;
    fn sample(&self) -> Result<f64>;
}

pub(super) const NOT_SUPPORTED: Hr = 0x80070032u32 as i32;

/// Picks the sensor for this processor. Every PawnIO module rechecks the CPU when it loads.
pub(super) struct SensorCatalog;
impl SensorCatalog {
    pub fn open(directory: &Path) -> Result<Box<dyn TemperatureSensor>> {
        let modules = ModuleFolder(directory.join("pawnio"));
        let cpu = CpuIdentity::current();
        Ok(match (cpu.vendor, cpu.family) {
            (Vendor::Amd, 0x17..=0x1a) => Box::new(amd::ZenTemperature::open(&modules)?),
            (Vendor::Amd, 0x10..=0x16) => Box::new(amd::K10Temperature::open(&modules, &cpu)?),
            (Vendor::Amd, 0x0f) => Box::new(amd::K8Temperature::open(&modules, &cpu)?),
            (Vendor::Intel, _) => Box::new(intel::IntelTemperature::open(&modules)?),
            (Vendor::Via, 7) => Box::new(zhaoxin::ZhaoxinTemperature::open(&modules)?),
            _ => return Err(NOT_SUPPORTED),
        })
    }
}

/// The signed modules shipped beside the collector.
pub(super) struct ModuleFolder(PathBuf);
impl ModuleFolder {
    pub fn load(&self, name: &str) -> Result<PawnModule> {
        PawnModule::load(&self.0.join(name))
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Vendor {
    Amd,
    Intel,
    Via,
    Other,
}

pub(super) struct CpuIdentity {
    pub vendor: Vendor,
    pub family: u32,
    pub model: u32,
    pub stepping: u32,
    /// CPUID 8000_0001h EBX: AMD package type in bits 31:28, K8 brand id in 13:9.
    pub brand: u32,
}
impl CpuIdentity {
    fn current() -> Self {
        let vendor = Self::leaf(0);
        let id = Self::leaf(1).eax;
        let name: Vec<u8> = [vendor.ebx, vendor.edx, vendor.ecx]
            .iter()
            .flat_map(|part| part.to_le_bytes())
            .collect();
        Self {
            vendor: match name.as_slice() {
                b"AuthenticAMD" => Vendor::Amd,
                b"GenuineIntel" => Vendor::Intel,
                b"CentaurHauls" => Vendor::Via,
                _ => Vendor::Other,
            },
            family: ((id >> 8) & 15) + ((id >> 20) & 255),
            model: ((id >> 4) & 15) | ((id >> 12) & 240),
            stepping: id & 15,
            brand: Self::leaf(0x8000_0001).ebx,
        }
    }
    pub fn leaf(leaf: u32) -> CpuidResult {
        __cpuid(leaf)
    }
}

/// Readings outside this window are sensor faults, never real temperatures.
pub(super) struct Celsius;
impl Celsius {
    pub fn checked(value: f64) -> Result<f64> {
        if value > 0.0 && value <= 125.0 {
            Ok(value)
        } else {
            Err(E_FAIL)
        }
    }
    pub fn hottest(readings: impl Iterator<Item = Result<f64>>) -> Result<f64> {
        readings.flatten().reduce(f64::max).ok_or(E_FAIL)
    }
}

/// Same cross-process PCI lock used by other hardware monitors (per PawnIO module contract).
pub(super) struct PciMutex(Handle);
impl PciMutex {
    pub fn new() -> Result<Self> {
        Ok(Self(Handle::new(unsafe {
            CreateMutexW(ptr::null_mut(), 0, wide("Global\\Access_PCI").as_ptr())
        })?))
    }
    pub fn lock(&self) -> Result<PciGuard<'_>> {
        match unsafe { WaitForSingleObject(self.0 .0, 100) } {
            0 | 0x80 => Ok(PciGuard(self)),
            _ => Err(E_FAIL),
        }
    }
}
pub(super) struct PciGuard<'a>(&'a PciMutex);
impl Drop for PciGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            ReleaseMutex(self.0 .0 .0);
        }
    }
}
#[link(name = "kernel32")]
extern "system" {
    fn ReleaseMutex(handle: Raw) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn faulty_readings_are_rejected_and_the_hottest_valid_one_wins() {
        assert_eq!(Celsius::checked(60.0), Ok(60.0));
        for bad in [-49.0, 0.0, 125.5] {
            assert!(Celsius::checked(bad).is_err());
        }
        assert_eq!(
            Celsius::hottest([Ok(50.0), Err(E_FAIL), Ok(64.5)].into_iter()),
            Ok(64.5)
        );
        assert!(Celsius::hottest([Err(E_FAIL)].into_iter()).is_err());
    }
}
