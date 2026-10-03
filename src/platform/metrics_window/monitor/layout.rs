/// Width classes of the window (`docs/design/ui-kit/responsiveness.md`). XAML Islands do not
/// evaluate `AdaptiveTrigger`, so the template is expanded per class instead.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowLayout {
    /// ≥ 1100: full navigation, four statistics, PID column.
    Full,
    /// 760–1099: 56 px icon navigation with an overlay menu.
    Compact,
    /// < 760: navigation hidden behind a menu button in the header.
    Minimal,
}

impl WindowLayout {
    pub fn of(width: f64) -> Self {
        if width >= 1100.0 {
            Self::Full
        } else if width >= 760.0 {
            Self::Compact
        } else {
            Self::Minimal
        }
    }

    pub fn compact(self) -> bool {
        self != Self::Full
    }

    /// Navigation shows icons without titles, so its buttons need tooltips.
    pub fn icon_only(self) -> bool {
        self == Self::Compact
    }

    fn values(self) -> [(&'static str, &'static str); 13] {
        let pick = |full, compact, minimal| match self {
            Self::Full => full,
            Self::Compact => compact,
            Self::Minimal => minimal,
        };
        [
            ("NavWidth", pick("248", "56", "0")),
            ("NavPadding", pick("8,8,8,12", "8,4,8,12", "0")),
            ("NavSpacing", pick("2", "4", "2")),
            ("NavVisible", pick("Visible", "Visible", "Collapsed")),
            ("MenuVisible", pick("Collapsed", "Visible", "Collapsed")),
            ("MenuBarVisible", pick("Collapsed", "Collapsed", "Visible")),
            ("SubtitleVisible", pick("Visible", "Collapsed", "Collapsed")),
            ("TitleSize", pick("28", "28", "20")),
            ("SearchWidth", pick("240", "170", "150")),
            (
                "ContentPadding",
                pick("24,18,24,20", "16,18,16,20", "12,12,12,16"),
            ),
            ("ContentCorner", pick("8,0,0,0", "8,0,0,0", "0")),
            ("EscVisible", pick("Visible", "Visible", "Collapsed")),
            (
                "SettingsTip",
                pick("", r#"ToolTipService.ToolTip="@Settings@""#, ""),
            ),
        ]
    }

    pub fn markup(self, template: &str) -> String {
        self.values()
            .into_iter()
            .fold(template.to_owned(), |markup, (key, value)| {
                markup.replace(&format!("%{key}%"), value)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoints_and_placeholders_follow_the_kit() {
        assert_eq!(WindowLayout::of(1280.0), WindowLayout::Full);
        assert_eq!(WindowLayout::of(820.0), WindowLayout::Compact);
        assert_eq!(WindowLayout::of(480.0), WindowLayout::Minimal);
        let markup =
            WindowLayout::Compact.markup(r#"<A Width="%NavWidth%" P="%ContentPadding%"/>"#);
        assert_eq!(markup, r#"<A Width="56" P="16,18,16,20"/>"#);
    }
}
