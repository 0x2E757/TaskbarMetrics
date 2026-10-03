//! File names of the program's executables in a package. Cargo builds them under
//! names of its own, which may hold no dots; packaging copies each to its name here
//! (tools\package-names.ps1 keeps the same list for the scripts). Plain std, so the
//! build script writes the same names into the version blocks.

pub struct Executables;

impl Executables {
    /// Attaches the tiles; the program people start and the Run value starts.
    pub const LAUNCHER: &'static str = "TaskbarMetrics.exe";
    /// The metrics window, opened from a tile.
    pub const WINDOW: &'static str = "TaskbarMetrics.Window.exe";
    pub const HISTORY: &'static str = "TaskbarMetrics.History.exe";
    pub const SENSORS: &'static str = "TaskbarMetrics.Sensors.exe";
    /// Loaded into Explorer.
    pub const HOST: &'static str = "TaskbarMetrics.Host.dll";
    /// Cargo's name of each packaged file, then the name it is packaged under.
    pub const PACKAGED: [(&'static str, &'static str); 5] = [
        ("taskbar-metrics.exe", Self::LAUNCHER),
        ("metrics-window.exe", Self::WINDOW),
        ("metrics-history.exe", Self::HISTORY),
        ("metrics-sensors.exe", Self::SENSORS),
        ("taskbar_metrics_host.dll", Self::HOST),
    ];
    /// The packaged name of the file Cargo builds as `built`.
    pub fn packaged(built: &str) -> Option<&'static str> {
        Self::PACKAGED
            .iter()
            .find(|(cargo, _)| *cargo == built)
            .map(|(_, packaged)| *packaged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scripts_package_the_same_names() {
        let script = include_str!("../../tools/package-names.ps1");
        let pairs: Vec<_> = script
            .lines()
            .filter(|line| line.trim_start().starts_with('\''))
            .filter_map(|line| {
                let (cargo, packaged) = line.trim().split_once(" = ")?;
                Some((cargo.trim_matches('\''), packaged.trim_matches('\'')))
            })
            .collect();
        assert_eq!(pairs, Executables::PACKAGED);
        assert_eq!(
            Executables::packaged("metrics-window.exe"),
            Some(Executables::WINDOW)
        );
    }
}
