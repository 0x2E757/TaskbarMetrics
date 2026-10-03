use crate::platform::devices::DeviceId;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::Duration,
};

/// `monitors=none`: no taskbar shows the tiles.
pub const NO_MONITORS: &str = "none";

#[derive(Clone, Debug)]
pub struct Settings {
    /// Taskbar tiles: device ids, `kind` or `kind@tag`.
    pub metrics: Vec<String>,
    /// Devices whose history is recorded in the background without a tile.
    pub history: Vec<String>,
    /// Monitors whose taskbars show the tiles (`Display::id`); empty means all,
    /// [`NO_MONITORS`] none.
    pub monitors: Vec<String>,
    pub interval: Duration,
    pub process_monitoring: bool,
    pub width: f64,
    pub gap: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            metrics: ["cpu", "gpu", "ram", "net", "disk"]
                .map(String::from)
                .to_vec(),
            history: Vec::new(),
            monitors: Vec::new(),
            interval: Duration::from_millis(500),
            process_monitoring: true,
            width: 260.0,
            gap: 12.0,
        }
    }
}

impl Settings {
    /// Small, deliberately strict format: key=value, # comments, no dependencies.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut result = Self::default();
        let mut seen = HashSet::new();
        for (index, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("Line {}: expected key=value", index + 1))?;
            let (key, value) = (key.trim(), value.trim());
            if !seen.insert(key) {
                return Err(format!("Duplicate setting: {key}"));
            }
            match key {
                "process_monitoring" => {
                    result.process_monitoring = value
                        .parse()
                        .map_err(|_| "process_monitoring must be true or false")?;
                }
                "metrics" => {
                    result.metrics = Self::devices(value, false)
                        .ok_or("metrics must contain unique device identifiers")?;
                }
                "history" => {
                    result.history = Self::devices(value, true)
                        .ok_or("history must contain unique device identifiers")?;
                }
                "monitors" => {
                    result.monitors = Self::monitors(value)
                        .ok_or("monitors must contain unique monitor identifiers")?;
                }
                "interval_ms" => {
                    let ms: u64 = value.parse().map_err(|_| "Invalid interval_ms")?;
                    if !(250..=60_000).contains(&ms) {
                        return Err("interval_ms must be 250..60000".into());
                    }
                    result.interval = Duration::from_millis(ms);
                }
                "width" | "gap" => {
                    let number: f64 = value.parse().map_err(|_| format!("Invalid {key}"))?;
                    let valid = if key == "width" {
                        (100.0..=800.0).contains(&number)
                    } else {
                        (0.0..=100.0).contains(&number)
                    };
                    if !valid {
                        return Err(format!("{key} is outside its allowed range"));
                    }
                    if key == "width" {
                        result.width = number;
                    } else {
                        result.gap = number;
                    }
                }
                _ => return Err(format!("Unknown setting: {key}")),
            }
        }
        Ok(result)
    }

    /// Comma-separated unique device ids; `history` may be empty.
    fn devices(value: &str, empty: bool) -> Option<Vec<String>> {
        if empty && value.is_empty() {
            return Some(Vec::new());
        }
        let ids: Vec<String> = value.split(',').map(|v| v.trim().to_string()).collect();
        let mut unique = HashSet::new();
        ids.iter()
            .all(|id| DeviceId::parse(id).is_some() && unique.insert(id.clone()))
            .then_some(ids)
    }

    /// Comma-separated unique monitor ids; empty for every monitor, [`NO_MONITORS`]
    /// alone for none.
    fn monitors(value: &str) -> Option<Vec<String>> {
        if value.is_empty() {
            return Some(Vec::new());
        }
        let ids: Vec<String> = value.split(',').map(|v| v.trim().to_string()).collect();
        if ids.len() > 1 && ids.iter().any(|id| id == NO_MONITORS) {
            return None;
        }
        let mut unique = HashSet::new();
        ids.iter()
            .all(|id| !id.is_empty() && unique.insert(id.clone()))
            .then_some(ids)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(format!("{}: {error}", path.display())),
        }
    }

    /// The same taskbar tiles, perhaps reordered: nothing new to sample.
    pub fn same_tiles(&self, other: &Self) -> bool {
        let sorted = |ids: &[String]| {
            let mut ids = ids.to_vec();
            ids.sort();
            ids
        };
        sorted(&self.metrics) == sorted(&other.metrics)
    }
}

/// Key updates in the configuration file.
pub struct ConfigFile {
    path: PathBuf,
}
impl ConfigFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    /// The configuration in the data `directory`.
    pub fn locate(directory: &Path) -> Self {
        Self::new(directory.join("taskbar-metrics.conf"))
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Rewrites `values`, appending missing keys and creating a missing file; the
    /// file is replaced atomically, so readers never see a partial configuration.
    pub fn write(&self, values: &[(&str, String)]) -> std::io::Result<()> {
        let text = std::fs::read_to_string(&self.path).unwrap_or_default();
        let mut missing: Vec<_> = values.iter().collect();
        let mut lines = text
            .lines()
            .map(|line| {
                let key = line.split_once('=').map(|(key, _)| key.trim());
                match values.iter().find(|(name, _)| Some(*name) == key) {
                    Some((name, value)) => {
                        missing.retain(|(other, _)| other != name);
                        format!("{name}={value}")
                    }
                    None => line.to_owned(),
                }
            })
            .collect::<Vec<_>>();
        lines.extend(
            missing
                .iter()
                .map(|(name, value)| format!("{name}={value}")),
        );
        // Explorer and the settings window may write at the same time.
        let temporary = self
            .path
            .with_extension(format!("conf.{}.tmp", std::process::id()));
        std::fs::write(&temporary, lines.join("\n") + "\n")?;
        std::fs::rename(&temporary, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_monitoring_can_be_disabled_without_changing_graph_cadence() {
        let settings = Settings::parse("process_monitoring=false\ninterval_ms=500\n").unwrap();
        assert!(!settings.process_monitoring);
        assert_eq!(settings.interval.as_millis(), 500);
        assert!(Settings::default().process_monitoring);
        assert!(Settings::parse("process_monitoring=maybe").is_err());
    }

    #[test]
    fn config_rejects_typos_duplicates_and_unbounded_values() {
        for text in [
            "interval_ms=0",
            "interval_ms=60001",
            "width=NaN",
            "gap=-1",
            "metrics=cpu,cpu",
            "metrics=",
            "metrics=fan",
            "metrics=disk@",
            "history=net@Wi-Fi,net@Wi-Fi",
            "monitors=A/1,A/1",
            "monitors=A/1,",
            "monitors=none,A/1",
            "interval=1000",
            "width=200\nwidth=300",
            "malformed",
        ] {
            assert!(Settings::parse(text).is_err(), "{text}");
        }
        let settings =
            Settings::parse(" # comment\nmetrics=ram,cpu\ninterval_ms=500\nwidth=220\ngap=8 # px")
                .unwrap();
        assert_eq!(settings.metrics, ["ram", "cpu"]);
        let devices = Settings::parse("metrics=cpu,disk,disk@D:\nhistory=net@Ethernet 2").unwrap();
        assert_eq!(devices.metrics, ["cpu", "disk", "disk@D:"]);
        assert_eq!(devices.history, ["net@Ethernet 2"]);
        assert!(Settings::parse("history=").unwrap().history.is_empty());
        assert!(Settings::default().monitors.is_empty());
        assert!(Settings::parse("monitors=").unwrap().monitors.is_empty());
        assert_eq!(
            Settings::parse("monitors=none").unwrap().monitors,
            [NO_MONITORS]
        );
        assert_eq!(
            Settings::parse("monitors=DEL41A8/5&2f2c&0&UID4353, GSM5B7F/4&1&0&UID256")
                .unwrap()
                .monitors,
            ["DEL41A8/5&2f2c&0&UID4353", "GSM5B7F/4&1&0&UID256"]
        );
        assert_eq!(settings.interval.as_millis(), 500);
        assert_eq!(settings.width, 220.0);
    }

    #[test]
    fn a_dragged_order_rewrites_only_the_tile_key() {
        let directory = std::env::temp_dir().join(format!("tm-order-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("taskbar-metrics.conf");
        std::fs::write(&path, "# tiles\nmetrics=cpu,gpu,ram\ngap=8\n").unwrap();
        let order = ["ram".to_string(), "cpu".into(), "gpu".into()];
        ConfigFile::new(path.clone())
            .write(&[("metrics", order.join(","))])
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# tiles\nmetrics=ram,cpu,gpu\ngap=8\n"
        );
        let before = Settings::parse("metrics=cpu,gpu,ram").unwrap();
        let after = Settings::load(&path).unwrap();
        assert!(after.same_tiles(&before));
        assert!(!Settings::parse("metrics=cpu,gpu")
            .unwrap()
            .same_tiles(&before));
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn a_missing_configuration_is_created_with_only_the_changed_key() {
        let directory = std::env::temp_dir().join(format!("tm-locate-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let file = ConfigFile::locate(&directory);
        assert_eq!(file.path(), directory.join("taskbar-metrics.conf"));
        assert_eq!(
            Settings::load(file.path()).unwrap().metrics,
            ["cpu", "gpu", "ram", "net", "disk"]
        );
        file.write(&[("process_monitoring", "false".into())])
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(file.path()).unwrap(),
            "process_monitoring=false\n"
        );
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
