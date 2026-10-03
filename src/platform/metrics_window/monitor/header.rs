use super::{chart::WINDOW, design::Design, locale::Language, ui::Ui};

/// What the section header reports about the time axis.
pub enum Mode {
    /// `tail` is the quiet suffix: collected history or "totals only".
    Live { tail: Option<String> },
    /// `ago` is omitted in compact windows.
    History { bucket: u64, ago: Option<u64> },
}

/// Mode pill and the "Back to live" button content of the section header.
pub struct HeaderMarkup {
    pub design: Design,
    pub language: Language,
}
impl HeaderMarkup {
    pub fn pill(&self, mode: &Mode) -> String {
        let d = self.design;
        let (background, border, content) = match mode {
            Mode::Live { tail } => (
                d.color("subtle"),
                d.color("divider").to_owned(),
                format!(
                    r#"<Grid Width="14" Height="14" VerticalAlignment="Center"><Ellipse Fill="{}"/><Ellipse Width="8" Height="8" Fill="{}"/></Grid><TextBlock Text="{}" FontSize="12" Foreground="{}" VerticalAlignment="Center" Margin="0,-1,0,1"/>{}"#,
                    Design::alpha(d.color("accent"), 0.22),
                    d.color("accent"),
                    self.language.text("Live"),
                    d.color("text2"),
                    self.tail(tail.as_deref())
                ),
            ),
            Mode::History { bucket, ago } => (
                d.color("accentSoft"),
                Design::alpha(d.color("accent"), 0.35),
                format!(
                    r#"{}<TextBlock Text="{}" FontSize="12" FontWeight="SemiBold" VerticalAlignment="Center" Margin="0,-1,0,1"/><TextBlock Text="{}" FontSize="12" VerticalAlignment="Center" Margin="0,-1,0,1" Typography.NumeralAlignment="Tabular"/>{}"#,
                    d.icon_sized("history", d.color("accent"), 14.0, 1.3),
                    self.language.text("History"),
                    self.language.time(*bucket),
                    self.tail(ago.map(|s| self.language.ago(s)).as_deref())
                ),
            ),
        };
        format!(
            r#"<Border xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Background="{background}" BorderBrush="{border}" BorderThickness="1" Height="28" CornerRadius="14" Padding="10,0"><StackPanel Orientation="Horizontal" Spacing="7">{content}</StackPanel></Border>"#
        )
    }
    fn tail(&self, text: Option<&str>) -> String {
        text.map(|text| {
            format!(
                r#"<TextBlock Text="· {}" FontSize="12" Foreground="{}" VerticalAlignment="Center" Margin="0,-1,0,1" Typography.NumeralAlignment="Tabular"/>"#,
                Ui::xml(text),
                self.design.color("text3")
            )
        })
        .unwrap_or_default()
    }
    /// Icon, label and a badge with the amount of history collected since the freeze.
    pub fn back(&self, pending_seconds: u64) -> String {
        let foreground = self.design.color("onAccent");
        let badge = if pending_seconds >= WINDOW / 2 {
            format!("+{}", self.language.span(WINDOW / 2))
        } else {
            format!("+{}", self.language.span(pending_seconds))
        };
        format!(
            r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Orientation="Horizontal" Spacing="8">{}<TextBlock Text="{}" FontWeight="SemiBold" Foreground="{foreground}" VerticalAlignment="Center"/><Border Background="{}" CornerRadius="8" Padding="6,1" VerticalAlignment="Center"><TextBlock Text="{}" FontSize="11" Foreground="{foreground}" Margin="0,-1,0,1" Typography.NumeralAlignment="Tabular"/></Border></StackPanel>"#,
            self.design.icon("return", foreground),
            self.language.text("Back to live"),
            Design::alpha(foreground, 0.16),
            Ui::xml(&badge)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pills_show_tail_and_badge_caps_at_five_minutes() {
        let header = HeaderMarkup {
            design: Design { dark: false },
            language: Language::English,
        };
        let live = header.pill(&Mode::Live {
            tail: Some("0:42 of 5:00 collected".into()),
        });
        assert!(live.contains(r#"Text="Live""#) && live.contains("· 0:42 of 5:00 collected"));
        assert!(header
            .pill(&Mode::History {
                bucket: 0,
                ago: Some(266)
            })
            .contains("· 4 min 26 s ago"));
        assert!(header.back(38).contains("+38 s"));
        assert!(header.back(900).contains("+5 min"));
    }
}
