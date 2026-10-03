use super::{super::SystemTheme, *};
use std::path::PathBuf;

/// The window's theme as chosen in the settings, kept in the data directory.
/// «System» follows the Windows app theme as it changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    fn path() -> Option<PathBuf> {
        crate::platform::data_directory::DataDirectory::file("taskbar-metrics.theme").ok()
    }

    /// Checks and demos follow the system, unless `--theme` forces one.
    pub fn load() -> Self {
        persistent()
            .then(Self::path)
            .flatten()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    pub fn save(self) -> std::io::Result<()> {
        if !persistent() {
            return Ok(());
        }
        let path = Self::path().ok_or(std::io::ErrorKind::NotFound)?;
        std::fs::write(path, self.key())
    }

    fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|choice| choice.key() == text.trim())
    }

    /// Catalog key of the name shown in the picker.
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    /// Name of the picker's menu item.
    pub fn item(self) -> String {
        format!("Theme{:?}", self)
    }

    pub fn dark(self) -> Result<bool> {
        match self {
            Self::System => Ok(SystemTheme::read()?.dark),
            Self::Light => Ok(false),
            Self::Dark => Ok(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_choices_read_back_and_anything_else_follows_the_system() {
        for choice in ThemeChoice::ALL {
            assert_eq!(ThemeChoice::parse(choice.key()), Some(choice));
        }
        assert_eq!(ThemeChoice::parse("dark\r\n"), Some(ThemeChoice::Dark));
        assert_eq!(ThemeChoice::parse("blue"), None);
        assert_eq!(ThemeChoice::Light.item(), "ThemeLight");
    }
}
