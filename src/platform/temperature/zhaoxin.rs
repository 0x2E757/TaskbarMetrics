use super::{
    pawnio::PawnModule,
    sensor::{Celsius, ModuleFolder, TemperatureSensor},
};
use crate::platform::abi::*;

/// VIA family 7 (Zhaoxin) reports whole degrees in the low 24 bits of MSR 1423h.
pub(super) struct ZhaoxinTemperature {
    module: PawnModule,
}
impl ZhaoxinTemperature {
    pub fn open(modules: &ModuleFolder) -> Result<Self> {
        Ok(Self {
            module: modules.load("ZhaoxinMSR.bin")?,
        })
    }
}
impl TemperatureSensor for ZhaoxinTemperature {
    fn name(&self) -> &'static str {
        "Zhaoxin CPU"
    }
    fn sample(&self) -> Result<f64> {
        let raw = self.module.read(c"ioctl_read_msr", &[0x1423])?;
        Celsius::checked((raw & 0xff_ffff) as f64)
    }
}
