use super::{
    pawnio::PawnModule,
    sensor::{Celsius, CpuIdentity, ModuleFolder, TemperatureSensor},
};

use crate::platform::abi::*;

const TEMPERATURE_TARGET: u64 = 0x1a2;
const THERM_STATUS: u64 = 0x19c;
const PACKAGE_THERM_STATUS: u64 = 0x1b1;

/// The digital thermal sensor counts degrees below TjMax. The package sensor is read when
/// the CPU has one; older CPUs report per core, and the hottest core wins.
pub(super) struct IntelTemperature {
    module: PawnModule,
    tjmax: f64,
    package: bool,
}

impl IntelTemperature {
    pub fn open(modules: &ModuleFolder) -> Result<Self> {
        let module = modules.load("IntelMSR.bin")?;
        Ok(Self {
            tjmax: Self::tjmax(module.read(c"ioctl_read_msr", &[TEMPERATURE_TARGET]).ok()),
            package: CpuIdentity::leaf(6).eax & (1 << 6) != 0,
            module,
        })
    }

    /// 100 °C when the CPU predates the TjMax register, as Linux assumes.
    fn tjmax(target: Option<u64>) -> f64 {
        match target.map(|value| (value >> 16) & 0xff) {
            Some(value) if value != 0 => value as f64,
            _ => 100.0,
        }
    }

    fn decode(status: u64, tjmax: f64) -> Result<f64> {
        Celsius::checked(tjmax - ((status >> 16) & 0x7f) as f64)
    }

    fn read(&self, register: u64) -> Result<f64> {
        Self::decode(
            self.module.read(c"ioctl_read_msr", &[register])?,
            self.tjmax,
        )
    }
}

impl TemperatureSensor for IntelTemperature {
    fn name(&self) -> &'static str {
        if self.package {
            "Intel CPU package"
        } else {
            "Intel CPU hottest core"
        }
    }

    fn sample(&self) -> Result<f64> {
        if self.package {
            return self.read(PACKAGE_THERM_STATUS);
        }
        Celsius::hottest((0..ProcessorPin::count()).map(|processor| {
            let _pin = ProcessorPin::new(processor)?;
            self.read(THERM_STATUS)
        }))
    }
}

/// Keeps the current thread on one logical processor of group 0 while a per-core MSR is read.
struct ProcessorPin(usize);

impl ProcessorPin {
    fn count() -> u32 {
        unsafe { GetActiveProcessorCount(0) }.min(usize::BITS)
    }

    fn new(processor: u32) -> Result<Self> {
        match unsafe { SetThreadAffinityMask(GetCurrentThread(), 1 << processor) } {
            0 => Err(last_error()),
            previous => Ok(Self(previous)),
        }
    }
}

impl Drop for ProcessorPin {
    fn drop(&mut self) {
        unsafe {
            SetThreadAffinityMask(GetCurrentThread(), self.0);
        }
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn GetActiveProcessorCount(group: u16) -> u32;
    fn GetCurrentThread() -> Raw;
    fn SetThreadAffinityMask(thread: Raw, mask: usize) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readout_counts_down_from_tjmax() {
        assert_eq!(IntelTemperature::tjmax(Some(0x0064_0000)), 100.0);
        assert_eq!(IntelTemperature::tjmax(Some(0x0f69_0000)), 105.0);
        assert_eq!(IntelTemperature::tjmax(Some(0)), 100.0);
        assert_eq!(IntelTemperature::tjmax(None), 100.0);
        assert_eq!(
            IntelTemperature::decode(0x8800_0000 | (40 << 16), 100.0),
            Ok(60.0)
        );
        assert!(IntelTemperature::decode(100 << 16, 100.0).is_err());
    }
}
