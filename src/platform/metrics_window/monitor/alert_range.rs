use super::{design::Design, locale::Language, ui::Ui};
use crate::platform::xaml::AlertSettings;

/// One alert rule of the settings artboard: a start threshold where the tile
/// begins to redden and a pulse threshold, edited on one shared track.
pub struct AlertRule {
    /// Name prefix of the lower and upper sliders, e.g. `Cpu` → `CpuX`, `CpuY`.
    pub name: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub minimum: f64,
    pub maximum: f64,
    /// The rule's start and pulse thresholds among the alert settings.
    pub thresholds: fn(&AlertSettings) -> (f64, f64),
    /// Above the upper threshold the tile pulses (hatched zone) rather than stays red.
    pub pulses: bool,
}

impl AlertRule {
    /// Thresholds restored when the rule is switched back on: the defaults of this
    /// computer, a desktop or a laptop.
    pub fn defaults(&self) -> (f64, f64) {
        (self.thresholds)(&AlertSettings::default())
    }

    /// A rule is off when both thresholds sit at the top of the track.
    pub fn off(&self) -> (f64, f64) {
        (self.maximum - 1.0, self.maximum)
    }

    pub fn enabled(&self, x: f64) -> bool {
        x < self.off().0
    }
}

/// Rules of the alert group; the rest redden tile numbers.
pub const ALERTS: usize = 3;
/// Tile alerts first, then tile numbers.
pub const RULES: [AlertRule; 6] = [
    AlertRule {
        name: "Cpu",
        label: "CPU temperature",
        unit: "°C",
        minimum: 30.0,
        maximum: 120.0,
        thresholds: |a| (a.cpu_x, a.cpu_y),
        pulses: true,
    },
    AlertRule {
        name: "Gpu",
        label: "GPU temperature",
        unit: "°C",
        minimum: 30.0,
        maximum: 120.0,
        thresholds: |a| (a.gpu_x, a.gpu_y),
        pulses: true,
    },
    AlertRule {
        name: "Ram",
        label: "RAM usage",
        unit: "%",
        minimum: 0.0,
        maximum: 100.0,
        thresholds: |a| (a.ram_x, a.ram_y),
        pulses: true,
    },
    AlertRule {
        name: "CpuHot",
        label: "CPU load",
        unit: "%",
        minimum: 50.0,
        maximum: 101.0,
        thresholds: |a| (a.cpu_hot_x, a.cpu_hot_y),
        pulses: false,
    },
    AlertRule {
        name: "GpuHot",
        label: "GPU load",
        unit: "%",
        minimum: 50.0,
        maximum: 101.0,
        thresholds: |a| (a.gpu_hot_x, a.gpu_hot_y),
        pulses: false,
    },
    AlertRule {
        name: "RamHot",
        label: "RAM usage",
        unit: "%",
        minimum: 50.0,
        maximum: 101.0,
        thresholds: |a| (a.ram_hot_x, a.ram_hot_y),
        pulses: false,
    },
];

/// Two thumbs over a track painted grey → reddening gradient → hatched pulse zone.
pub struct AlertRange {
    pub design: Design,
    pub language: Language,
}

impl AlertRange {
    const WIDTH: f64 = 260.0;
    const THUMB: f64 = 20.0;

    /// Settings row: label, the 260 px range and the rule switch.
    pub fn row(index: usize, rule: &AlertRule, divided: bool) -> String {
        let slider = |suffix: &str| {
            format!(
                r#"<Slider x:Name="{name}{suffix}" Style="{{StaticResource RangeThumb}}" Minimum="{min}" Maximum="{max}" StepFrequency="1" Height="20" VerticalAlignment="Top" AutomationProperties.Name="@{label}@"/>"#,
                name = rule.name,
                min = rule.minimum,
                max = rule.maximum,
                label = rule.label,
            )
        };
        format!(
            r#"<Grid MinHeight="66" Padding="48,0,16,0" ColumnSpacing="16" BorderBrush="$divider$" BorderThickness="0,{},0,0"><Grid.ColumnDefinitions><ColumnDefinition/><ColumnDefinition Width="260"/><ColumnDefinition Width="Auto"/></Grid.ColumnDefinitions><TextBlock Text="@{label}@" VerticalAlignment="Center" TextWrapping="Wrap"/><Grid Grid.Column="1" Height="52" Padding="0,16,0,0" VerticalAlignment="Center"><Canvas x:Name="AlertTrack{index}" IsHitTestVisible="False"/>{lower}{upper}</Grid><ToggleSwitch x:Name="AlertOn{index}" Grid.Column="2" OnContent="" OffContent="" Width="40" VerticalAlignment="Center" AutomationProperties.Name="@{label}@"/></Grid>"#,
            u8::from(divided),
            label = rule.label,
            lower = slider("X"),
            upper = slider("Y"),
        )
    }

    fn position(rule: &AlertRule, value: f64) -> f64 {
        let fraction = ((value - rule.minimum) / (rule.maximum - rule.minimum)).clamp(0.0, 1.0);
        Self::THUMB / 2.0 + fraction * (Self::WIDTH - Self::THUMB)
    }

    /// Painted track and threshold captions for the current `(x, y)`.
    pub fn track(&self, rule: &AlertRule, x: f64, y: f64) -> String {
        let d = self.design;
        let (danger, track) = (d.color("danger"), d.color("sliderTrack"));
        let (start, pulse) = (Self::position(rule, x), Self::position(rule, y));
        let mut markup = format!(
            r#"<Canvas xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Width="{w}" Height="38"><Rectangle Canvas.Top="8" Width="{start:.1}" Height="4" RadiusX="2" RadiusY="2" Fill="{track}"/><Rectangle Canvas.Left="{start:.1}" Canvas.Top="8" Width="{:.1}" Height="4"><Rectangle.Fill><LinearGradientBrush StartPoint="0,0" EndPoint="1,0"><GradientStop Color="{track}" Offset="0"/><GradientStop Color="{danger}" Offset="1"/></LinearGradientBrush></Rectangle.Fill></Rectangle>{}"#,
            (pulse - start).max(0.0),
            if rule.pulses {
                format!(
                    r#"<Rectangle Canvas.Left="{pulse:.1}" Canvas.Top="6" Width="{:.1}" Height="8" RadiusX="4" RadiusY="4" Fill="{}"/><Line X1="{pulse:.1}" X2="{w}" Y1="10" Y2="10" Stroke="{danger}" StrokeThickness="8" StrokeDashArray="0.75,0.5"/>"#,
                    (Self::WIDTH - pulse).max(0.0),
                    Design::alpha(danger, 0.35),
                    w = Self::WIDTH,
                )
            } else {
                // Fully red from the upper threshold on.
                format!(
                    r#"<Rectangle Canvas.Left="{pulse:.1}" Canvas.Top="8" Width="{:.1}" Height="4" RadiusX="2" RadiusY="2" Fill="{danger}"/>"#,
                    (Self::WIDTH - pulse).max(0.0),
                )
            },
            w = Self::WIDTH,
        );
        // A rule switched off parks both thumbs at the top: no captions to show.
        if !rule.enabled(x) {
            markup.push_str("</Canvas>");
            return markup;
        }
        // Captions of close thresholds move apart instead of overlapping.
        let gap = (44.0 - (pulse - start)).max(0.0) / 2.0;
        for (value, center) in [(x, start - gap), (y, pulse + gap)] {
            markup.push_str(&format!(
                r#"<TextBlock Canvas.Left="{:.1}" Canvas.Top="22" Width="60" Text="{}" TextAlignment="Center" FontSize="11" Foreground="{}" Typography.NumeralAlignment="Tabular"/>"#,
                center - 30.0,
                Ui::xml(&format!("{} {}", self.language.number(value, 0), rule.unit)),
                d.color("text2")
            ));
        }
        markup.push_str("</Canvas>");
        markup
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_rule_reads_its_own_thresholds() {
        let alerts = AlertSettings {
            cpu_x: 1.0,
            cpu_y: 2.0,
            gpu_x: 3.0,
            gpu_y: 4.0,
            ram_x: 5.0,
            ram_y: 6.0,
            cpu_hot_x: 7.0,
            cpu_hot_y: 8.0,
            gpu_hot_x: 9.0,
            gpu_hot_y: 10.0,
            ram_hot_x: 11.0,
            ram_hot_y: 12.0,
            ..AlertSettings::default()
        };
        let read: Vec<_> = RULES
            .iter()
            .map(|rule| (rule.name, (rule.thresholds)(&alerts)))
            .collect();
        assert_eq!(
            read,
            [
                ("Cpu", (1.0, 2.0)),
                ("Gpu", (3.0, 4.0)),
                ("Ram", (5.0, 6.0)),
                ("CpuHot", (7.0, 8.0)),
                ("GpuHot", (9.0, 10.0)),
                ("RamHot", (11.0, 12.0)),
            ]
        );
    }

    #[test]
    fn rules_switch_off_at_the_top_and_tracks_follow_thresholds() {
        let cpu = &RULES[0];
        assert!(cpu.enabled(65.0) && !cpu.enabled(cpu.off().0));
        let range = AlertRange {
            design: Design { dark: false },
            language: Language::Russian,
        };
        let track = range.track(cpu, 75.0, 105.0);
        assert!(track.contains("75 °C") && track.contains("105 °C"));
        assert!(
            track.contains(r#"Canvas.Left="130.0""#) && track.contains(r#"Canvas.Left="100.0""#)
        );
        assert!(AlertRange::row(0, cpu, false).contains(r#"x:Name="CpuX""#));
        // Numbers do not pulse: solid red above the upper threshold, no captions when off.
        let number = &RULES[ALERTS];
        assert!(!range.track(number, 80.0, 85.0).contains("StrokeDashArray"));
        assert!(!range.track(number, 100.0, 101.0).contains("TextBlock"));
    }
}
