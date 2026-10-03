use super::{
    design::Design,
    locale::Language,
    model::{Device, Resource},
    ui::Ui,
};
use crate::platform::process_history::store::Frame;

/// Left navigation: section buttons with their values at the shared moment.
pub struct Navigation {
    pub design: Design,
    pub language: Language,
    pub compact: bool,
}
impl Navigation {
    /// Button declarations for the XAML template, one per menu entry titled `titles`;
    /// content is rendered by `item`. Only `icon_only` buttons repeat their title in a tooltip.
    pub fn buttons(prefix: &str, titles: &[String], icon_only: bool) -> String {
        titles
            .iter()
            .enumerate()
            .map(|(index, title)| {
                let title = Ui::xml(title);
                let tooltip = if icon_only {
                    format!(r#" ToolTipService.ToolTip="{title}""#)
                } else {
                    String::new()
                };
                format!(
                    r#"<Button Style="{{StaticResource NavigationButton}}" x:Name="{prefix}Tab{index}"{tooltip} AutomationProperties.Name="{title}"/>"#
                )
            })
            .collect()
    }
    /// A network adapter shows its link; demo adapters, which the machine does not
    /// list, are told by their name.
    pub fn icon(device: &Device) -> &'static str {
        match device.resource {
            Resource::Ram => "mem",
            Resource::Net => {
                let wireless = match device.link {
                    Some(link) => link != "Ethernet",
                    None => device
                        .id
                        .tag
                        .as_deref()
                        .is_some_and(|tag| tag.starts_with("Wi")),
                };
                if wireless {
                    "wifi"
                } else {
                    "ethernet"
                }
            }
            other => other.id(),
        }
    }
    pub fn value(&self, device: &Device, frame: Option<&Frame>) -> String {
        let Some(frame) = frame else {
            return String::new();
        };
        let resource = device.resource;
        let values = device.total(frame);
        let number = |value: Option<f64>, precision| {
            value
                .map(|v| self.language.number(v, precision))
                .unwrap_or_else(|| "—".into())
        };
        let rate =
            |value: Option<f64>| number(value, if value.unwrap_or(0.0) < 10.0 { 1 } else { 0 });
        match resource {
            Resource::Cpu | Resource::Gpu => values[0]
                .map(|v| self.language.percent(v, 0))
                .unwrap_or_else(|| "—".into()),
            // Memory in use system-wide, like the other system totals of the list, so the
            // value keeps its meaning when process monitoring is off.
            Resource::Ram => values[0]
                .map(|v| format!("{} GB", self.language.number(v / 1024.0, 1)))
                .unwrap_or_else(|| "—".into()),
            // Read or receive · write or send, with the unit like the other items.
            _ => format!("{} · {} MB/s", rate(values[0]), rate(values[1])),
        }
    }
    /// Content of one navigation button; `value` is empty for Settings.
    pub fn item(&self, icon: &str, title: &str, value: &str, selected: bool) -> String {
        let text = self.design.color("text");
        let indicator = format!(
            r#"<Border Width="3" Height="16" CornerRadius="2" Background="{}" HorizontalAlignment="Left" VerticalAlignment="Top" Margin="0,10,0,0"/>"#,
            if selected {
                self.design.color("accent")
            } else {
                "Transparent"
            }
        );
        let background = if selected {
            self.design.color("navSel")
        } else {
            "Transparent"
        };
        // Menu icons are drawn on half pixels for a crisp 1 px line.
        let icon = self.design.icon_sized(icon, text, 16.0, 1.0);
        if self.compact {
            return format!(
                r#"<Grid xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Background="{background}" CornerRadius="4">{indicator}<Grid HorizontalAlignment="Center" VerticalAlignment="Center">{icon}</Grid></Grid>"#
            );
        }
        format!(
            r#"<Grid xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Background="{background}" CornerRadius="4">{indicator}<Grid Padding="14,0,12,0" ColumnSpacing="12"><Grid.ColumnDefinitions><ColumnDefinition Width="16"/><ColumnDefinition/><ColumnDefinition Width="Auto"/></Grid.ColumnDefinitions>{icon}<TextBlock Grid.Column="1" Text="{}" FontWeight="{}" Foreground="{text}" VerticalAlignment="Center" TextTrimming="CharacterEllipsis"/><TextBlock Grid.Column="2" Text="{}" FontSize="12" Foreground="{}" VerticalAlignment="Center" Typography.NumeralAlignment="Tabular"/></Grid></Grid>"#,
            Ui::xml(title),
            if selected { "SemiBold" } else { "Normal" },
            Ui::xml(value),
            self.design.color("text2")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::devices::DeviceId;
    #[test]
    fn network_adapters_show_their_link() {
        let adapter = |tag: &str, link| {
            Device::new(DeviceId::new("net", Some(tag.into())))
                .unwrap()
                .with_link(link)
        };
        assert_eq!(Navigation::icon(&adapter("Wi‑Fi", Some("Wi‑Fi"))), "wifi");
        assert_eq!(
            Navigation::icon(&adapter("LAN", Some("Ethernet"))),
            "ethernet"
        );
        assert_eq!(Navigation::icon(&adapter("Ethernet", None)), "ethernet");
        assert_eq!(Navigation::icon(&adapter("Wi‑Fi", None)), "wifi");
        assert_eq!(Navigation::icon(&Device::of(Resource::Ram)), "mem");
    }
    #[test]
    fn buttons_are_named_per_prefix_and_items_mark_selection() {
        let titles = ["CPU".into(), "Disk D:".into()];
        let buttons = Navigation::buttons("Overlay", &titles, false);
        assert!(buttons.contains(r#"x:Name="OverlayTab1""#));
        assert!(!buttons.contains("ToolTip"));
        let icons = Navigation::buttons("", &titles, true);
        assert!(icons.contains(r#"ToolTipService.ToolTip="Disk D:""#));
        let nav = Navigation {
            design: Design { dark: false },
            language: Language::Russian,
            compact: false,
        };
        let item = nav.item("cpu", "CPU", "37\u{A0}%", true);
        assert!(item.contains("#E4E7EB") && item.contains("#005FB8"));
        assert!(item.contains(r#"FontWeight="SemiBold""#));
        assert!(!Navigation {
            compact: true,
            ..nav
        }
        .item("cpu", "CPU", "", false)
        .contains("TextBlock"));
    }
}
