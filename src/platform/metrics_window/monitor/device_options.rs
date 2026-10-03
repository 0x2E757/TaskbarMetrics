use crate::{config::Settings, platform::devices::DeviceId};

/// «Show on taskbar» and «Always monitor» of one device, kept in `metrics=` and
/// `history=` of the configuration. A bare kind in `metrics` (`disk`) stands for
/// the main device of that kind, resolved by `canonical`.
pub struct DeviceOptions<'a, F: Fn(&DeviceId) -> DeviceId> {
    pub settings: &'a Settings,
    pub canonical: F,
}
impl<F: Fn(&DeviceId) -> DeviceId> DeviceOptions<'_, F> {
    fn names(&self, id: &str, device: &DeviceId) -> bool {
        DeviceId::parse(id).is_some_and(|parsed| (self.canonical)(&parsed) == *device)
    }
    /// `(on the taskbar, always monitored)`; a tile implies monitoring.
    pub fn state(&self, device: &DeviceId) -> (bool, bool) {
        let taskbar = self
            .settings
            .metrics
            .iter()
            .any(|id| self.names(id, device));
        let always = taskbar
            || self
                .settings
                .history
                .iter()
                .any(|id| self.names(id, device));
        (taskbar, always)
    }
    /// Tiles with `device` added or removed; `None` when it is the last tile, which
    /// the configuration requires.
    pub fn taskbar(&self, device: &DeviceId, on: bool) -> Option<Vec<String>> {
        let mut metrics: Vec<String> = self
            .settings
            .metrics
            .iter()
            .filter(|id| !self.names(id, device))
            .cloned()
            .collect();
        if on {
            // The main device keeps the short id existing configurations use.
            let bare = DeviceId::new(&device.kind, None);
            metrics.push(if (self.canonical)(&bare) == *device {
                bare.to_string()
            } else {
                device.to_string()
            });
        }
        (!metrics.is_empty()).then_some(metrics)
    }
    /// Background history list with `device` added or removed.
    pub fn history(&self, device: &DeviceId, on: bool) -> Vec<String> {
        let mut history: Vec<String> = self
            .settings
            .history
            .iter()
            .filter(|id| !self.names(id, device))
            .cloned()
            .collect();
        if on {
            history.push(device.to_string());
        }
        history
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bare_kinds_name_the_main_device_and_the_last_tile_stays() {
        let settings = Settings::parse("metrics=cpu,disk\nhistory=net@Wi-Fi").unwrap();
        let options = DeviceOptions {
            settings: &settings,
            canonical: |id: &DeviceId| match id.kind.as_str() {
                "disk" if id.tag.is_none() => DeviceId::parse("disk@C:").unwrap(),
                _ => id.clone(),
            },
        };
        let c = DeviceId::parse("disk@C:").unwrap();
        let d = DeviceId::parse("disk@D:").unwrap();
        let wifi = DeviceId::parse("net@Wi-Fi").unwrap();
        assert_eq!(options.state(&c), (true, true));
        assert_eq!(options.state(&d), (false, false));
        assert_eq!(options.state(&wifi), (false, true));
        assert_eq!(
            options.taskbar(&d, true).unwrap(),
            ["cpu", "disk", "disk@D:"]
        );
        assert_eq!(options.taskbar(&c, false).unwrap(), ["cpu"]);
        assert_eq!(options.taskbar(&c, true).unwrap(), ["cpu", "disk"]);
        assert_eq!(options.history(&wifi, false), Vec::<String>::new());
        let single = Settings::parse("metrics=disk").unwrap();
        let options = DeviceOptions {
            settings: &single,
            canonical: options.canonical,
        };
        assert_eq!(options.taskbar(&c, false), None);
    }
}
