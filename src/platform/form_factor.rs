//! Whether the computer is a laptop. A laptop cools worse and runs hotter, so the
//! temperature alerts of its tiles start later.

use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum FormFactor {
    /// Desktops, mini PCs and virtual machines.
    Desktop,
    /// Laptops, tablets and convertibles.
    Portable,
}

impl FormFactor {
    /// Read from the power capabilities once a process.
    pub fn current() -> Self {
        static CURRENT: OnceLock<FormFactor> = OnceLock::new();
        *CURRENT.get_or_init(|| {
            let mut capabilities = PowerCapabilities::default();
            // SystemPowerCapabilities; STATUS_SUCCESS is 0.
            // SAFETY: the output buffer is the SDK's SYSTEM_POWER_CAPABILITIES.
            let status = unsafe {
                CallNtPowerInformation(
                    4,
                    std::ptr::null(),
                    0,
                    &mut capabilities,
                    std::mem::size_of::<PowerCapabilities>() as u32,
                )
            };
            if status == 0 {
                capabilities.form_factor()
            } else {
                Self::Desktop
            }
        })
    }
}

/// SYSTEM_POWER_CAPABILITIES; only the fields read here are named, the rest keeps
/// its size and the alignment of its ULONGs.
#[repr(C, align(4))]
struct PowerCapabilities {
    buttons: [u8; 2],
    lid_present: u8,
    states: [u8; 27],
    batteries_present: u8,
    /// A UPS: the battery of a desktop.
    batteries_short_term: u8,
    rest: [u8; 44],
}

impl Default for PowerCapabilities {
    fn default() -> Self {
        Self {
            buttons: [0; 2],
            lid_present: 0,
            states: [0; 27],
            batteries_present: 0,
            batteries_short_term: 0,
            rest: [0; 44],
        }
    }
}

impl PowerCapabilities {
    /// A lid, or a battery that is not a UPS.
    fn form_factor(&self) -> FormFactor {
        if self.lid_present != 0 || (self.batteries_present != 0 && self.batteries_short_term == 0)
        {
            FormFactor::Portable
        } else {
            FormFactor::Desktop
        }
    }
}

#[link(name = "powrprof")]
extern "system" {
    fn CallNtPowerInformation(
        level: i32,
        input: *const u8,
        input_size: u32,
        output: *mut PowerCapabilities,
        output_size: u32,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form_factor(lid: u8, batteries: u8, short_term: u8) -> FormFactor {
        PowerCapabilities {
            lid_present: lid,
            batteries_present: batteries,
            batteries_short_term: short_term,
            ..PowerCapabilities::default()
        }
        .form_factor()
    }

    #[test]
    fn a_lid_or_a_battery_that_is_not_a_ups_makes_a_laptop() {
        assert_eq!(form_factor(1, 1, 0), FormFactor::Portable);
        assert_eq!(form_factor(1, 0, 0), FormFactor::Portable);
        assert_eq!(form_factor(0, 1, 0), FormFactor::Portable);
        assert_eq!(form_factor(0, 1, 1), FormFactor::Desktop);
        assert_eq!(form_factor(0, 0, 0), FormFactor::Desktop);
    }

    #[test]
    fn layout_matches_the_windows_sdk() {
        assert_eq!(std::mem::size_of::<PowerCapabilities>(), 76);
        assert_eq!(std::mem::offset_of!(PowerCapabilities, lid_present), 2);
        assert_eq!(
            std::mem::offset_of!(PowerCapabilities, batteries_present),
            30
        );
    }
}
