use super::{
    pawnio::PawnModule,
    sensor::{Celsius, CpuIdentity, ModuleFolder, PciMutex, TemperatureSensor, NOT_SUPPORTED},
};

use crate::platform::abi::*;

/// CurTmp of K10 and Zen: bits 31:21 in 1/8 °C, read as the control temperature (Tctl).
struct Tctl;

impl Tctl {
    /// `range_select` is Zen's bit 19; both generations shift by 49 °C when bits 17:16 are set.
    fn decode(raw: u64, range_select: bool) -> Result<f64> {
        let raw = u32::try_from(raw).map_err(|_| E_FAIL)?;
        if raw == 0 || raw == u32::MAX {
            return Err(E_FAIL);
        }
        let extended = (range_select && raw & (1 << 19) != 0) || raw & 0x30000 == 0x30000;
        Celsius::checked(f64::from(raw >> 21) * 0.125 - if extended { 49.0 } else { 0.0 })
    }
}

/// Families 17h–1Ah. Reports Tctl, without guessing SKU-specific Tdie offsets.
pub(super) struct ZenTemperature {
    module: PawnModule,
    pci: PciMutex,
}

impl ZenTemperature {
    pub fn open(modules: &ModuleFolder) -> Result<Self> {
        Ok(Self {
            module: modules.load("AMDFamily17.bin")?,
            pci: PciMutex::new()?,
        })
    }
}

impl TemperatureSensor for ZenTemperature {
    fn name(&self) -> &'static str {
        "AMD CPU Tctl"
    }

    fn sample(&self) -> Result<f64> {
        let _guard = self.pci.lock()?;
        Tctl::decode(self.module.read(c"ioctl_read_smn", &[0x59800])?, true)
    }
}

/// Families 10h–16h: the northbridge's reported temperature register,
/// which models 60h–7Fh of family 15h moved behind the SMU.
pub(super) struct K10Temperature {
    module: PawnModule,
    pci: PciMutex,
    smu: bool,
}

impl K10Temperature {
    pub fn open(modules: &ModuleFolder, cpu: &CpuIdentity) -> Result<Self> {
        if Self::unreliable(cpu) {
            return Err(NOT_SUPPORTED);
        }
        Ok(Self {
            module: modules.load("AMDFamily10.bin")?,
            pci: PciMutex::new()?,
            smu: cpu.family == 0x15 && matches!(cpu.model & 0xf0, 0x60 | 0x70),
        })
    }

    /// Erratum 319: the sensor of family 10h on sockets F and AM2+ may be unreliable.
    fn unreliable(cpu: &CpuIdentity) -> bool {
        cpu.family == 0x10
            && match cpu.brand >> 28 {
                0 => true,
                1 => cpu.model < 4 || (cpu.model == 4 && cpu.stepping <= 2),
                _ => false,
            }
    }
}

impl TemperatureSensor for K10Temperature {
    fn name(&self) -> &'static str {
        "AMD CPU Tctl"
    }

    fn sample(&self) -> Result<f64> {
        let _guard = self.pci.lock()?;
        let raw = if self.smu {
            self.module.read(c"ioctl_read_smu", &[0xd820_0ca4])?
        } else {
            self.module.read(c"ioctl_read_miscctl", &[0, 0xa4])?
        };
        Tctl::decode(raw, false)
    }
}

/// Family 0Fh: the THERMTRIP register of each core; the hottest core wins.
pub(super) struct K8Temperature {
    module: PawnModule,
    pci: PciMutex,
    cores: u64,
    offset: f64,
}

impl K8Temperature {
    pub fn open(modules: &ModuleFolder, cpu: &CpuIdentity) -> Result<Self> {
        // The sensor exists since revision SH-C0.
        if (cpu.model == 4 && cpu.stepping == 0) || (cpu.model == 5 && cpu.stepping <= 1) {
            return Err(NOT_SUPPORTED);
        }
        Ok(Self {
            module: modules.load("AMDFamily0F.bin")?,
            pci: PciMutex::new()?,
            cores: u64::from((CpuIdentity::leaf(0x8000_0008).ecx & 0xff) + 1).min(2),
            offset: if Self::revision_g_desktop(cpu) {
                21.0
            } else {
                0.0
            },
        })
    }

    /// Desktop revision G parts read 21 °C below ambient without an offset.
    fn revision_g_desktop(cpu: &CpuIdentity) -> bool {
        let brand = (cpu.brand >> 9) & 0x1f;
        cpu.model >= 0x69
            && !matches!(cpu.model, 0xc1 | 0x6c | 0x7c)
            && !(matches!(cpu.model, 0x6f | 0x7f) && matches!(brand, 0x7 | 0x9 | 0xc))
            && !(cpu.model == 0x6b && matches!(brand, 0xb | 0xc))
    }

    fn decode(raw: u64, offset: f64) -> Result<f64> {
        match (raw >> 16) & 0xff {
            0 => Err(E_FAIL),
            value => Celsius::checked(value as f64 - 49.0 + offset),
        }
    }
}

impl TemperatureSensor for K8Temperature {
    fn name(&self) -> &'static str {
        "AMD K8 CPU"
    }

    fn sample(&self) -> Result<f64> {
        let _guard = self.pci.lock()?;
        Celsius::hottest((0..self.cores).map(|core| {
            Self::decode(
                self.module.read(c"ioctl_get_thermtrip", &[0, core])?,
                self.offset,
            )
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::temperature::sensor::Vendor;

    fn cpu(family: u32, model: u32, stepping: u32, brand: u32) -> CpuIdentity {
        CpuIdentity {
            vendor: Vendor::Amd,
            family,
            model,
            stepping,
            brand,
        }
    }

    #[test]
    fn decodes_both_temperature_ranges_and_rejects_bad_reads() {
        assert_eq!(Tctl::decode(480 << 21, true), Ok(60.0));
        assert_eq!(Tctl::decode((872 << 21) | (1 << 19), true), Ok(60.0));
        assert_eq!(Tctl::decode((872 << 21) | 0x30000, true), Ok(60.0));
        for bad in [0, u64::from(u32::MAX), 2047 << 21, (1 << 21) | (1 << 19)] {
            assert!(Tctl::decode(bad, true).is_err());
        }
    }

    #[test]
    fn range_select_bit_shifts_only_zen() {
        assert_eq!(Tctl::decode((480 << 21) | (1 << 19), false), Ok(60.0));
        assert_eq!(Tctl::decode((872 << 21) | 0x30000, false), Ok(60.0));
    }

    #[test]
    fn family_10h_on_sockets_f_and_am2_plus_is_unreliable() {
        assert!(K10Temperature::unreliable(&cpu(0x10, 4, 3, 0)));
        assert!(K10Temperature::unreliable(&cpu(0x10, 2, 3, 1 << 28)));
        assert!(K10Temperature::unreliable(&cpu(0x10, 4, 2, 1 << 28)));
        assert!(!K10Temperature::unreliable(&cpu(0x10, 4, 3, 1 << 28)));
        assert!(!K10Temperature::unreliable(&cpu(0x15, 2, 0, 0)));
    }

    #[test]
    fn k8_offsets_only_revision_g_desktop_parts() {
        assert_eq!(K8Temperature::decode(100 << 16, 0.0), Ok(51.0));
        assert!(K8Temperature::decode(0, 21.0).is_err());
        assert!(K8Temperature::revision_g_desktop(&cpu(0x0f, 0x6b, 1, 0)));
        assert!(!K8Temperature::revision_g_desktop(&cpu(
            0x0f,
            0x6b,
            1,
            0xb << 9
        )));
        assert!(!K8Temperature::revision_g_desktop(&cpu(0x0f, 0x7c, 1, 0)));
        assert!(!K8Temperature::revision_g_desktop(&cpu(0x0f, 0x43, 1, 0)));
    }
}
