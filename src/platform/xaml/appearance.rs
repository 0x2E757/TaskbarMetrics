use super::{Palette, TileStyle};
use std::path::Path;

pub(crate) struct Appearance;
struct Field {
    name: &'static str,
    min: f64,
    max: f64,
    get: fn(&TileStyle) -> f64,
    set: fn(&mut TileStyle, f64),
}
macro_rules! field {($min:expr,$max:expr,$($member:ident).+) => {Field{name:stringify!($($member).+),min:$min,max:$max,get:|s|s.$($member).+,set:|s,v|s.$($member).+ = v}};}
/// An on/off setting, stored as 0 or 1.
struct Switch {
    name: &'static str,
    get: fn(&TileStyle) -> bool,
    set: fn(&mut TileStyle, bool),
}
macro_rules! switch {
    ($member:ident) => {
        Switch {
            name: stringify!($member),
            get: |s| s.$member,
            set: |s, v| s.$member = v,
        }
    };
}

const SWITCHES: &[Switch] = &[
    switch!(cpu_temperature),
    switch!(gpu_temperature),
    switch!(dashed_temperature),
];
const FIELDS: &[Field] = &[
    field!(80.0, 180.0, width),
    field!(0.0, 20.0, gap),
    field!(0.0, 18.0, radius),
    field!(7.0, 15.0, label_size),
    field!(9.0, 20.0, value_size),
    field!(7.0, 16.0, secondary_size),
    field!(0.5, 3.0, stroke),
    field!(0.0, 100.0, opacity),
    field!(-5.0, 5.0, offset_x),
    field!(-5.0, 5.0, offset_y),
    field!(0.0, 60.0, graph_left),
    field!(-100.0, 40.0, fade_offset),
    field!(0.0, 100.0, fade_width),
    field!(0.0, 100.0, fade_opacity),
    field!(30.0, 119.0, alerts.cpu_x),
    field!(31.0, 120.0, alerts.cpu_y),
    field!(30.0, 119.0, alerts.gpu_x),
    field!(31.0, 120.0, alerts.gpu_y),
    field!(0.0, 99.0, alerts.ram_x),
    field!(1.0, 100.0, alerts.ram_y),
    field!(0.0, 80.0, alerts.alert_opacity),
    field!(0.5, 6.0, alerts.pulse_seconds),
    field!(50.0, 100.0, alerts.cpu_hot_x),
    field!(51.0, 101.0, alerts.cpu_hot_y),
    field!(50.0, 100.0, alerts.gpu_hot_x),
    field!(51.0, 101.0, alerts.gpu_hot_y),
    field!(50.0, 100.0, alerts.ram_hot_x),
    field!(51.0, 101.0, alerts.ram_hot_y),
];
impl Appearance {
    /// The disk tile showed busy time and reddened by it; it shows MB/s now.
    const RETIRED: [&'static str; 2] = ["alerts.disk_hot_x", "alerts.disk_hot_y"];

    pub fn read(path: &Path) -> std::io::Result<TileStyle> {
        Self::decode(&std::fs::read_to_string(path)?).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid appearance settings",
            )
        })
    }

    /// Persisted settings that differ between two styles; preview values are not persisted.
    pub fn differences(a: &TileStyle, b: &TileStyle) -> usize {
        FIELDS.iter().filter(|f| (f.get)(a) != (f.get)(b)).count()
            + SWITCHES.iter().filter(|s| (s.get)(a) != (s.get)(b)).count()
            + (0..14).filter(|i| a.light.0[*i] != b.light.0[*i]).count()
            + (0..14).filter(|i| a.dark.0[*i] != b.dark.0[*i]).count()
    }

    pub fn save(path: &Path, style: &TileStyle) -> std::io::Result<()> {
        let text = Self::encode(style);
        let temporary = path.with_extension("appearance.tmp");
        std::fs::write(&temporary, text)?;
        std::fs::rename(temporary, path)
    }

    fn encode(style: &TileStyle) -> String {
        let mut text = String::from("version=1\n");
        for field in FIELDS {
            text.push_str(&format!("{}={}\n", field.name, (field.get)(style)));
        }
        for switch in SWITCHES {
            text.push_str(&format!(
                "{}={}\n",
                switch.name,
                u8::from((switch.get)(style))
            ));
        }
        for (name, palette) in [("light", style.light), ("dark", style.dark)] {
            for index in 0..14 {
                text.push_str(&format!("{name}.{index}={}\n", palette.hex(index)));
            }
        }
        text
    }

    fn decode(text: &str) -> Option<TileStyle> {
        let mut style = TileStyle::default();
        let mut seen = std::collections::HashSet::new();
        for line in text.lines() {
            let (key, value) = line.split_once('=')?;
            if !seen.insert(key) {
                return None;
            }
            if key == "version" {
                if value != "1" {
                    return None;
                }
                continue;
            }
            // Settings of earlier versions: skipped, so their files still load.
            if Self::RETIRED.contains(&key) {
                continue;
            }
            // Missing switches keep their defaults, so earlier files still load.
            if let Some(switch) = SWITCHES.iter().find(|s| s.name == key) {
                let on = match value {
                    "0" => false,
                    "1" => true,
                    _ => return None,
                };
                (switch.set)(&mut style, on);
                continue;
            }
            if let Some(field) = FIELDS.iter().find(|f| f.name == key) {
                let value: f64 = value.parse().ok()?;
                if !value.is_finite() || !(field.min..=field.max).contains(&value) {
                    return None;
                }
                (field.set)(&mut style, value);
            } else {
                let (theme, index) = key.split_once('.')?;
                let index: usize = index.parse().ok()?;
                if index >= 14 {
                    return None;
                }
                let palette = match theme {
                    "light" => &mut style.light,
                    "dark" => &mut style.dark,
                    _ => return None,
                };
                palette.0[index] = Palette::parse(value)?;
            }
        }
        style.light.retire_defaults(false);
        style.dark.retire_defaults(true);
        let a = style.alerts;
        if !seen.contains("version")
            || [
                (a.cpu_x, a.cpu_y),
                (a.gpu_x, a.gpu_y),
                (a.ram_x, a.ram_y),
                (a.cpu_hot_x, a.cpu_hot_y),
                (a.gpu_hot_x, a.gpu_hot_y),
                (a.ram_hot_x, a.ram_hot_y),
            ]
            .iter()
            .any(|(x, y)| *y < x + 1.0)
        {
            return None;
        }
        Some(style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_and_reject_invalid_ranges() {
        let mut style = TileStyle {
            width: 125.0,
            dashed_temperature: true,
            ..TileStyle::default()
        };
        style.light.0[3] = 0xA0123456;
        let text = Appearance::encode(&style);
        assert_eq!(Appearance::decode(&text), Some(style));
        assert!(Appearance::decode(&text.replace("width=125", "width=NaN")).is_none());
        assert!(
            Appearance::decode(&text.replace("dashed_temperature=1", "dashed_temperature=2"))
                .is_none()
        );
        assert!(Appearance::decode(&text.replace("alerts.cpu_y=80", "alerts.cpu_y=60")).is_none());
        // A file saved with the disk thresholds still loads.
        let earlier = format!("{text}alerts.disk_hot_x=80\nalerts.disk_hot_y=85\n");
        assert_eq!(Appearance::decode(&earlier), Some(style));
        // A file from before the temperature switches shows the temperatures.
        let before = text.replace("cpu_temperature=1\n", "");
        assert_eq!(Appearance::decode(&before), Some(style));
        let hidden = TileStyle {
            gpu_temperature: false,
            ..style
        };
        let encoded = Appearance::encode(&hidden);
        assert!(encoded.contains("gpu_temperature=0\n"));
        assert_eq!(Appearance::decode(&encoded), Some(hidden));
        assert_eq!(Appearance::differences(&style, &hidden), 1);
        assert!(!hidden.temperature("gpu") && hidden.temperature("cpu"));
        assert!(!hidden.temperature("ram"));
        let mut changed = style;
        changed.gap += 1.0;
        changed.dark.0[2] = 0;
        changed.demo_cpu += 5.0;
        assert_eq!(Appearance::differences(&style, &changed), 2);
    }
}
