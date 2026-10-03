use super::super::*;
use super::{locale::Language, ui::Ui};
use crate::{
    config::{ConfigFile, Settings},
    platform::{
        data_directory::DataDirectory,
        displays::{Display, Displays},
    },
};

const STATUS: &str = "MonitorsStatus";
const HINT: &str = "If none of the chosen monitors is connected, the tiles show on the main one";

/// "Monitors with tiles": a check box for each connected monitor, kept in
/// `monitors=` of the configuration. Checks and demos show the boxes without
/// writing anything.
pub(super) struct MonitorOptions {
    root: Com,
    displays: Displays,
    checks: Vec<Com>,
    shown: Vec<bool>,
    writable: bool,
    language: Language,
}
impl MonitorOptions {
    /// Check boxes of `displays`, for the `@MONITORS@` place of the page.
    pub fn markup(displays: &Displays, language: Language) -> String {
        displays
            .list()
            .iter()
            .enumerate()
            .map(|(index, display)| {
                let label = Ui::xml(&Self::label(display, language)).replace('@', "&#64;");
                format!(
                    r#"<CheckBox x:Name="Monitor{index}" AutomationProperties.Name="{label}"><TextBlock Text="{label}"/></CheckBox>"#
                )
            })
            .collect()
    }
    /// `LG ULTRAGEAR · 2560 × 1440 · main`.
    fn label(display: &Display, language: Language) -> String {
        let name = match (display.name.as_str(), display.internal) {
            ("", true) => language.text("Built-in display").to_string(),
            ("", false) => language.text("Monitor").to_string(),
            (name, _) => name.to_string(),
        };
        let mut label = format!("{name} · {} × {}", display.width, display.height);
        if display.primary {
            label = format!("{label} · {}", language.text("main"));
        }
        label
    }
    pub fn new(root: &Com, displays: Displays, language: Language) -> Result<Self> {
        let chosen = Self::settings().monitors;
        let mut options = Self {
            root: root.clone(),
            checks: (0..displays.list().len())
                .map(|index| Ui::find(root, &format!("Monitor{index}")))
                .collect::<Result<_>>()?,
            shown: displays
                .list()
                .iter()
                .map(|display| displays.shows(&chosen, display))
                .collect(),
            displays,
            writable: super::persistent(),
            language,
        };
        options.render()?;
        Ok(options)
    }
    fn settings() -> Settings {
        DataDirectory::file("taskbar-metrics.conf")
            .ok()
            .and_then(|path| Settings::load(&path).ok())
            .unwrap_or_default()
    }
    fn render(&mut self) -> Result<()> {
        for (check, on) in self.checks.iter().zip(&self.shown) {
            Ui::checked(check, *on)?;
        }
        Ok(())
    }
    fn caption(&self, text: &str) -> Result<()> {
        Ui::text(&self.root, STATUS, self.language.text(text))
    }
    /// Stores a click. The last checked box stays, and a failed write puts the
    /// boxes back and says why.
    pub fn refresh(&mut self) -> Result<()> {
        let checked = self
            .checks
            .iter()
            .map(Ui::is_checked)
            .collect::<Result<Vec<_>>>()?;
        if checked == self.shown {
            return Ok(());
        }
        if !checked.contains(&true) {
            self.render()?;
            return self.caption("At least one monitor shows the tiles");
        }
        if self.writable {
            if let Err(error) = self.store(&checked) {
                self.render()?;
                let failed = self.language.text("Could not save setting");
                return Ui::text(&self.root, STATUS, &format!("{failed}: {error}"));
            }
        }
        self.shown = checked;
        self.caption(HINT)
    }
    /// Every monitor checked is an empty choice, so a monitor connected later
    /// shows the tiles too; chosen monitors that are away now stay chosen.
    fn store(&self, checked: &[bool]) -> std::io::Result<()> {
        let list = self.displays.list();
        let monitors = if checked.iter().all(|on| *on) {
            Vec::new()
        } else {
            let mut monitors: Vec<String> = Self::settings()
                .monitors
                .into_iter()
                .filter(|id| list.iter().all(|display| display.id != *id))
                .collect();
            monitors.extend(
                list.iter()
                    .zip(checked)
                    .filter(|(_, on)| **on)
                    .map(|(display, _)| display.id.clone()),
            );
            monitors
        };
        ConfigFile::locate(&DataDirectory::path()?).write(&[("monitors", monitors.join(","))])
    }
}
