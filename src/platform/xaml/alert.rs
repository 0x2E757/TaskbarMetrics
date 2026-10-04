use super::*;
use crate::platform::form_factor::FormFactor;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AlertSettings {
    pub cpu_x: f64,
    pub cpu_y: f64,
    pub gpu_x: f64,
    pub gpu_y: f64,
    pub ram_x: f64,
    pub ram_y: f64,
    pub alert_opacity: f64,
    pub pulse_seconds: f64,
    /// Tile numbers start reddening at X and are fully red at Y; Y above 100 % is off.
    pub cpu_hot_x: f64,
    pub cpu_hot_y: f64,
    pub gpu_hot_x: f64,
    pub gpu_hot_y: f64,
    pub ram_hot_x: f64,
    pub ram_hot_y: f64,
}

/// The defaults of the computer this runs on.
impl Default for AlertSettings {
    fn default() -> Self {
        Self::defaults(FormFactor::current())
    }
}

impl AlertSettings {
    /// A laptop runs hotter: its temperature alerts start later.
    pub fn defaults(form_factor: FormFactor) -> Self {
        let desktop = Self {
            cpu_x: 65.0,
            cpu_y: 80.0,
            gpu_x: 70.0,
            gpu_y: 80.0,
            ram_x: 85.0,
            ram_y: 95.0,
            alert_opacity: 20.0,
            pulse_seconds: 2.0,
            cpu_hot_x: 80.0,
            cpu_hot_y: 85.0,
            gpu_hot_x: 80.0,
            gpu_hot_y: 85.0,
            ram_hot_x: 80.0,
            ram_hot_y: 85.0,
        };
        match form_factor {
            FormFactor::Desktop => desktop,
            FormFactor::Portable => Self {
                cpu_x: 85.0,
                cpu_y: 95.0,
                gpu_x: 75.0,
                gpu_y: 85.0,
                ..desktop
            },
        }
    }

    /// How red the number of a `kind` tile showing `value` % is, from 0 to 1.
    pub fn hot(&self, kind: &str, value: f64) -> f64 {
        let (x, y) = match kind {
            "cpu" => (self.cpu_hot_x, self.cpu_hot_y),
            "gpu" => (self.gpu_hot_x, self.gpu_hot_y),
            "ram" => (self.ram_hot_x, self.ram_hot_y),
            _ => return 0.0,
        };
        if y > 100.0 || !value.is_finite() {
            return 0.0;
        }
        ((value - x) / (y - x).max(1.0)).clamp(0.0, 1.0)
    }

    fn level(&self, id: &str, value: &crate::metrics::MetricValue) -> (f64, bool) {
        let (x, y) = match id {
            "cpu" => (self.cpu_x, self.cpu_y),
            "gpu" => (self.gpu_x, self.gpu_y),
            "ram" => (self.ram_x, self.ram_y),
            _ => return (0.0, false),
        };
        let crate::metrics::MetricValue::Available(value) = value else {
            return (0.0, false);
        };
        if !value.is_finite() {
            return (0.0, false);
        }
        let y = y.max(x + 1.0);
        (
            ((value - x) / (y - x)).clamp(0.0, 1.0) * self.alert_opacity / 100.0,
            *value >= y,
        )
    }
}

/// XAML runs the pulse on its animation clock, independently of sensor sampling.
#[derive(Clone)]
pub(super) struct AlertVisual {
    background: Com,
    pulse: Com,
    pulsing: bool,
}

impl AlertVisual {
    pub fn new(background: Com, pulse: Com) -> Result<Self> {
        Ok(Self {
            background: background.query(&UI_ELEMENT)?,
            pulse: pulse.query(&Guid::from_u128(0xd45c1e6e_3594_460e_981a_32271bd3aa06))?,
            pulsing: false,
        })
    }

    pub fn update(
        &mut self,
        settings: &AlertSettings,
        id: &str,
        value: &crate::metrics::MetricValue,
    ) -> Result<()> {
        let (opacity, pulsing) = settings.level(id, value);
        if self.pulsing != pulsing {
            unsafe {
                let call: unsafe extern "system" fn(Raw) -> Hr =
                    self.pulse.slot(if pulsing { 9 } else { 8 });
                check(call(self.pulse.raw()))?;
            }
            self.pulsing = pulsing;
        }
        if !pulsing {
            XamlElement(self.background.clone()).set_number(10, opacity)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::MetricValue::*;

    #[test]
    fn thresholds_ramp_pulse_and_clear_on_missing_data() {
        let s = AlertSettings::defaults(FormFactor::Desktop);
        assert_eq!(s.level("cpu", &Available(65.0)), (0.0, false));
        assert_eq!(s.level("cpu", &Available(72.5)), (0.1, false));
        assert_eq!(s.level("cpu", &Available(105.0)), (0.2, true));
        assert_eq!(s.level("ram", &Available(95.0)), (0.2, true));
        assert_eq!(s.level("gpu", &Unavailable), (0.0, false));
        assert_eq!(s.level("net", &Available(100.0)), (0.0, false));
    }

    #[test]
    fn numbers_redden_gradually_over_five_points_before_the_threshold() {
        let mut s = AlertSettings::defaults(FormFactor::Desktop);
        assert_eq!(s.hot("cpu", 80.0), 0.0);
        assert_eq!(s.hot("cpu", 82.5), 0.5);
        assert_eq!(s.hot("gpu", 85.0), 1.0);
        // Disk tiles show MB/s, which no percentage threshold fits.
        assert_eq!(s.hot("disk", 100.0), 0.0);
        assert_eq!(s.hot("net", 100.0), 0.0);
        // Both thresholds at the top of the track switch reddening off.
        (s.ram_hot_x, s.ram_hot_y) = (100.0, 101.0);
        assert_eq!(s.hot("ram", 100.0), 0.0);
    }

    #[test]
    fn laptops_alert_on_hotter_cpus_and_gpus_only() {
        let desktop = AlertSettings::defaults(FormFactor::Desktop);
        let laptop = AlertSettings::defaults(FormFactor::Portable);
        assert_eq!((laptop.cpu_x, laptop.cpu_y), (85.0, 95.0));
        assert_eq!((laptop.gpu_x, laptop.gpu_y), (75.0, 85.0));
        assert_eq!(laptop.level("cpu", &Available(80.0)), (0.0, false));
        assert_eq!(
            AlertSettings {
                cpu_x: desktop.cpu_x,
                cpu_y: desktop.cpu_y,
                gpu_x: desktop.gpu_x,
                gpu_y: desktop.gpu_y,
                ..laptop
            },
            desktop
        );
    }
}
