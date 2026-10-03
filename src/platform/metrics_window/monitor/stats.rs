use super::super::{Com, Result};
use super::{
    design::Design,
    devices::Facts,
    locale::Language,
    model::{Device, Resource},
    ui::Ui,
};

use crate::platform::xaml::events::Subscription;
use crate::platform::{
    process_history::store::Frame, providers::PhysicalMemory, xaml::AlertSettings,
};

use std::sync::Arc;

/// One statistic: label, value, an optional quiet caption and an optional hint
/// behind an info icon next to the label.
pub struct Stat {
    pub label: String,
    pub value: String,
    pub caption: Option<String>,
    pub hint: Option<String>,
    pub alert: bool,
}

/// Values shown above the chart for the selected moment.
pub struct StatsSource<'a> {
    pub language: Language,
    pub device: &'a Device,
    pub frame: Option<&'a Frame>,
    pub timeline: &'a [Arc<Frame>],
    pub alerts: AlertSettings,
    /// Current upper bound of the chart scale.
    pub scale: f64,
    /// Dedicated GPU memory `(in use, total)` in bytes.
    pub gpu_memory: Option<(Option<f64>, f64)>,
    /// Live values of the shown device; none for a past moment.
    pub facts: Facts,
}

impl StatsSource<'_> {
    fn text(&self, key: &str) -> String {
        self.language.text(key).to_owned()
    }

    fn stat(&self, label: &str, value: String, caption: Option<String>) -> Stat {
        Stat {
            label: self.text(label),
            value,
            caption,
            hint: None,
            alert: false,
        }
    }

    fn missing() -> String {
        "—".into()
    }

    /// Temperature or RAM value with the alert state of its upper threshold.
    fn alerting(
        &self,
        label: &str,
        value: Option<f64>,
        limit: f64,
        text: String,
        caption: Option<String>,
    ) -> Stat {
        let alert = value.is_some_and(|v| v >= limit);
        Stat {
            label: self.text(label),
            value: text,
            caption: if alert {
                let limit = if self.device.resource == Resource::Ram {
                    self.language.percent(limit, 0)
                } else {
                    self.language.celsius(limit)
                };
                Some(
                    self.text("above alert threshold {v}")
                        .replace("{v}", &limit),
                )
            } else {
                caption
            },
            hint: None,
            alert,
        }
    }

    fn temperature(&self) -> Stat {
        let value = self.frame.and_then(|f| self.device.temperature(f));
        let limit = if self.device.resource == Resource::Gpu {
            self.alerts.gpu_y
        } else {
            self.alerts.cpu_y
        };
        self.alerting(
            "Temperature",
            value,
            limit,
            value
                .map(|v| self.language.celsius(v))
                .unwrap_or_else(Self::missing),
            None,
        )
    }

    fn total(&self, id: &str) -> Option<f64> {
        self.frame?
            .totals
            .iter()
            .find(|(key, _)| key == id)
            .and_then(|(_, v)| *v)
    }

    /// The scale explains its minimum and growth on demand, not in a caption.
    fn scale(&self) -> Stat {
        let minimum = self.language.rate(self.device.resource.floor(), 0);
        Stat {
            hint: Some(
                self.text("min {v}, grows with peak")
                    .replace("{v}", &minimum),
            ),
            ..self.stat(
                "Scale",
                format!("0–{}", self.language.rate(self.scale, 0)),
                None,
            )
        }
    }

    /// Hottest moment of the 5-minute window, or of the part of it collected so far.
    fn temperature_peak(&self) -> Stat {
        let peak = self
            .timeline
            .iter()
            .filter_map(|f| Some((self.device.temperature(f)?, f.bucket)))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        match peak {
            Some((value, bucket)) => self.stat(
                "5-min peak",
                self.language.celsius(value),
                Some(self.language.clock(bucket)),
            ),
            None => self.stat("5-min peak", Self::missing(), Some(self.text("no data"))),
        }
    }

    pub fn stats(&self) -> Vec<Stat> {
        let values = self
            .frame
            .map(|f| self.device.total(f))
            .unwrap_or([None, None]);
        let percent = |v: Option<f64>| {
            v.map(|v| self.language.percent(v, 1))
                .unwrap_or_else(Self::missing)
        };
        let rate = |v: Option<f64>| {
            v.map(|v| self.language.rate(v, 2))
                .unwrap_or_else(Self::missing)
        };
        let mut stats = match self.device.resource {
            Resource::Cpu => vec![
                self.stat("Load", percent(values[0]), None),
                self.temperature(),
                self.temperature_peak(),
            ],
            Resource::Gpu => vec![
                // The adapter is in the header subtitle, engines are in the table.
                Stat {
                    hint: Some(
                        self.text("Load of the busiest GPU engine: 3D, compute, copy or video"),
                    ),
                    ..self.stat("Load", percent(values[0]), None)
                },
                self.temperature(),
                self.temperature_peak(),
            ]
            .into_iter()
            .chain(self.gpu_memory.map(|(used, total)| {
                let gb = |bytes: f64| self.language.number(bytes / (1u64 << 30) as f64, 1);
                self.stat(
                    "Dedicated memory",
                    format!(
                        "{} / {} GB",
                        used.map(gb).unwrap_or_else(Self::missing),
                        gb(total)
                    ),
                    None,
                )
            }))
            .collect(),
            Resource::Ram => {
                // «26.2 GB (85 %)»: the share is the one the taskbar alert watches.
                let usage = self.total("ram");
                let used = values[0].map(|v| format!("{} GB", self.language.number(v / 1024.0, 1)));
                let share = usage.map(|v| self.language.percent(v, 0));
                vec![
                    self.alerting(
                        "In use",
                        usage,
                        self.alerts.ram_y,
                        match (used, share) {
                            (Some(used), Some(share)) => format!("{used} ({share})"),
                            (used, share) => used.or(share).unwrap_or_else(Self::missing),
                        },
                        None,
                    ),
                    self.stat(
                        "Installed",
                        PhysicalMemory::total_bytes()
                            .map(|v| {
                                format!(
                                    "{} GB",
                                    self.language.number(v as f64 / (1u64 << 30) as f64, 1)
                                )
                            })
                            .unwrap_or_else(Self::missing),
                        None,
                    ),
                ]
            }
            Resource::Disk => vec![
                self.stat("Read", rate(values[0]), None),
                self.stat("Write", rate(values[1]), None),
            ],
            Resource::Net => vec![
                self.stat("Receive", rate(values[0]), None),
                self.stat("Send", rate(values[1]), None),
            ],
        };
        match self.device.resource {
            Resource::Disk | Resource::Net => stats.push(self.scale()),
            _ => {}
        }
        stats.extend(self.live());
        // The title names the link, as Task Manager does; the name Windows gives the
        // adapter («Ethernet0 2») is its tag.
        if let (Resource::Net, Some(name)) = (self.device.resource, &self.device.id.tag) {
            stats.push(self.stat("Adapter name", name.clone(), None));
        }
        stats
    }

    /// A value that may be missing, without a caption.
    fn plain(&self, label: &str, value: Option<String>) -> Stat {
        self.stat(label, value.unwrap_or_else(Self::missing), None)
    }

    /// Live values the window shows for the current moment only.
    fn live(&self) -> Vec<Stat> {
        let f = self.facts;
        let activity = f.activity;
        match self.device.resource {
            Resource::Cpu => vec![
                self.stat(
                    "Frequency",
                    f.frequency
                        .map(|mhz| format!("{} GHz", self.language.number(mhz / 1000.0, 2)))
                        .unwrap_or_else(Self::missing),
                    None,
                ),
                self.plain("Up time", activity.map(|a| self.language.uptime(a.uptime))),
            ],
            Resource::Ram => {
                let gb = |bytes: u64| self.language.number(bytes as f64 / (1u64 << 30) as f64, 1);
                vec![self.stat(
                    "Page file",
                    f.page_file
                        .map(|p| format!("{} / {} GB", gb(p.used), gb(p.size)))
                        .unwrap_or_else(Self::missing),
                    None,
                )]
            }
            Resource::Net if self.device.link == Some("Wi‑Fi") => {
                // dBm with the usual grades: −50 and up is excellent, below −70 weak.
                let grade = |dbm: i32| match dbm {
                    -50.. => "excellent",
                    -60..=-51 => "good",
                    -70..=-61 => "fair",
                    _ => "weak",
                };
                vec![self.stat(
                    "Signal",
                    f.signal
                        .map(|dbm| format!("{dbm} dBm").replace('-', "−"))
                        .unwrap_or_else(Self::missing),
                    f.signal.map(|dbm| self.text(grade(dbm))),
                )]
            }
            Resource::Net | Resource::Gpu | Resource::Disk => Vec::new(),
        }
    }
}

/// Widest each statistic has been since its section was shown: a column never
/// narrows, so a value gaining a digit does not push its neighbours back and forth.
#[derive(Default)]
pub struct StatWidths {
    section: Option<(Resource, bool)>,
    widths: Vec<f64>,
}

impl StatWidths {
    /// Starts over for another section or window layout; true when it is the same one.
    pub fn track(&mut self, section: (Resource, bool)) -> bool {
        let same = self.section == Some(section);
        if !same {
            self.section = Some(section);
            self.widths.clear();
        }
        same
    }

    pub fn grow(&mut self, index: usize, width: f64) {
        if self.widths.len() <= index {
            self.widths.resize(index + 1, 0.0);
        }
        self.widths[index] = self.widths[index].max(width);
    }

    pub fn get(&self, index: usize) -> f64 {
        self.widths.get(index).copied().unwrap_or(0.0)
    }
}

/// Rows of statistics: label 12, value 20/600 (18 in compact windows), caption 11.
/// Items are named `Stat{index}` so their laid-out widths can feed `StatWidths`.
/// A statistic that does not fit in `available` px starts the next row.
pub struct StatsPanel {
    pub design: Design,
    pub compact: bool,
    pub available: f64,
}

impl StatsPanel {
    fn spacing(&self) -> f64 {
        if self.compact {
            28.0
        } else {
            36.0
        }
    }

    /// Compact windows keep one row: the statistics that do not fit it are left out,
    /// the last first.
    pub fn fitting(&self, mut stats: Vec<Stat>, widths: &StatWidths) -> Vec<Stat> {
        if let (true, Some(&next)) = (self.compact, self.rows(&stats, widths).get(1)) {
            stats.truncate(next);
        }
        stats
    }

    /// Index of the first statistic of each row. Columns not laid out yet are
    /// estimated from their text; once measured they only widen, so the rows settle.
    pub fn rows(&self, stats: &[Stat], widths: &StatWidths) -> Vec<usize> {
        let mut starts = vec![0];
        let mut used = 0.0;
        for (index, stat) in stats.iter().enumerate() {
            let width = match widths.get(index) {
                0.0 => (stat.label.chars().count() as f64 * 6.5)
                    .max(stat.value.chars().count() as f64 * 11.0),
                measured => measured,
            };
            if index > 0 && used + self.spacing() + width > self.available {
                starts.push(index);
                used = width;
            } else {
                used += if index > 0 { self.spacing() } else { 0.0 } + width;
            }
        }
        starts
    }

    pub fn markup(&self, stats: &[Stat], widths: &StatWidths) -> String {
        let starts = self.rows(stats, widths);
        let rows: String = starts
            .iter()
            .zip(starts.iter().skip(1).copied().chain([stats.len()]))
            .map(|(&first, end)| {
                let row = &stats[first..end];
                // A row with a hint card stays above the rows below it.
                let above = if row.iter().any(|stat| stat.hint.is_some()) {
                    r#" Canvas.ZIndex="1""#
                } else {
                    ""
                };
                let items: String = row
                    .iter()
                    .enumerate()
                    .map(|(offset, stat)| self.item(first + offset, stat, widths))
                    .collect();
                format!(
                    r#"<StackPanel{above} Orientation="Horizontal" Spacing="{}">{items}</StackPanel>"#,
                    self.spacing()
                )
            })
            .collect();
        format!(
            r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Spacing="12">{rows}</StackPanel>"#
        )
    }

    /// One statistic: label, value with an alert icon, caption, and an optional hint.
    fn item(&self, index: usize, stat: &Stat, widths: &StatWidths) -> String {
        let d = self.design;
        let color = d.color(if stat.alert { "danger" } else { "text" });
        // A tooltip waits for the pointer to rest, so the hint is a card of the
        // row that `hints` shows as soon as the pointer enters the icon. The
        // transparent target catches the pointer: the icon is not hit-testable.
        let hint = stat.hint.as_deref().map_or_else(String::new, |hint| {
            format!(
                r#"<Grid x:Name="HintTarget{index}" Background="Transparent" Padding="2" Margin="-2" AutomationProperties.Name="{text}">{}<Canvas><Border x:Name="HintCard{index}" Canvas.Left="-8" Canvas.Top="20" Visibility="Collapsed" IsHitTestVisible="False" Padding="8,5,8,6" CornerRadius="4" Background="{}" BorderBrush="{}" BorderThickness="1"><TextBlock Text="{text}" FontSize="12" Foreground="{}" MaxWidth="320" TextWrapping="Wrap"/></Border></Canvas></Grid>"#,
                d.icon_sized("info", d.color("text3"), 12.0, 1.3),
                d.color("card"),
                d.color("border"),
                d.color("text"),
                text = Ui::xml(hint),
            )
        });
        // The card covers the value below and the columns to the right.
        let above = if stat.hint.is_some() {
            r#" Canvas.ZIndex="1""#
        } else {
            ""
        };
        format!(
            r#"<StackPanel x:Name="Stat{index}"{above} MinWidth="{:.1}" Spacing="1" VerticalAlignment="Bottom"><StackPanel{above} Orientation="Horizontal" Spacing="4"><TextBlock Text="{}" FontSize="12" Foreground="{}"/>{hint}</StackPanel><StackPanel Orientation="Horizontal" Spacing="6">{}<TextBlock x:Name="Value{index}" Text="{}" FontSize="{}" FontWeight="SemiBold" LineHeight="{}" LineStackingStrategy="BlockLineHeight" Margin="0,1,0,-1" Foreground="{color}" Typography.NumeralAlignment="Tabular"/></StackPanel><TextBlock x:Name="Caption{index}" Text="{}" FontSize="11" Foreground="{}" Typography.NumeralAlignment="Tabular"/></StackPanel>"#,
            widths.get(index),
            Ui::xml(&stat.label),
            d.color("text2"),
            if stat.alert {
                d.icon_sized("alert", color, 16.0, 1.6)
            } else {
                String::new()
            },
            Ui::xml(&stat.value),
            if self.compact { 18 } else { 20 },
            // Matches the CSS line box (1.25 × font): XAML puts the whole extra
            // height above the glyphs, so the compact value needs a tighter block.
            if self.compact { 21.0 } else { 25.0 },
            // An empty caption still takes its line, so all values share a baseline.
            Ui::xml(stat.caption.as_deref().unwrap_or("\u{A0}")),
            d.color("text3")
        )
    }

    /// What the markup holds apart from values and captions: rows of the same shape
    /// are updated in place, so a hint under the pointer is not rebuilt away.
    pub fn shape(&self, stats: &[Stat], widths: &StatWidths) -> String {
        let items: Vec<_> = stats
            .iter()
            .map(|stat| format!("{}\u{1F}{:?}\u{1F}{}", stat.label, stat.hint, stat.alert))
            .collect();
        format!(
            "{}\u{1D}{:?}",
            items.join("\u{1E}"),
            self.rows(stats, widths)
        )
    }

    /// Shows each hint card while the pointer is over its icon.
    pub fn hints(row: &Com, stats: &[Stat]) -> Result<Vec<Subscription>> {
        let mut events = Vec::new();
        for (index, _) in stats.iter().enumerate().filter(|(_, s)| s.hint.is_some()) {
            let target = Ui::find(row, &format!("HintTarget{index}"))?;
            let card = Ui::find(row, &format!("HintCard{index}"))?;
            // PointerEntered, PointerExited.
            for (slot, shown) in [(63, true), (65, false)] {
                let card = card.clone();
                events.push(Subscription::pointer(&target, slot, move |_, _| {
                    Ui::show(&card, shown)
                })?);
            }
        }
        Ok(events)
    }

    /// Values and captions of a row built from stats of the same shape; columns
    /// keep the widest width they have had.
    pub fn update(row: &Com, stats: &[Stat], widths: &StatWidths) -> Result<()> {
        for (index, stat) in stats.iter().enumerate() {
            Ui::text(row, &format!("Value{index}"), &stat.value)?;
            Ui::text(
                row,
                &format!("Caption{index}"),
                stat.caption.as_deref().unwrap_or("\u{A0}"),
            )?;
            Ui::min_width(&Ui::find(row, &format!("Stat{index}"))?, widths.get(index))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(temperature: f64) -> Frame {
        Frame {
            bucket: 10,
            elapsed_ms: 500.0,
            processes: vec![],
            gpu_engines: Vec::new(),
            totals: vec![
                ("cpu".into(), Some(37.26)),
                ("cpu_temperature".into(), Some(temperature)),
            ],
            etw_active: true,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 0.0,
        }
    }

    fn source<'a>(frame: &'a Frame) -> StatsSource<'a> {
        StatsSource {
            language: Language::English,
            // Leaked: the source borrows its device for the whole test.
            device: Box::leak(Box::new(Device::of(Resource::Cpu))),
            frame: Some(frame),
            timeline: &[],
            alerts: AlertSettings::default(),
            scale: 100.0,
            gpu_memory: Some((Some(2.1 * (1u64 << 30) as f64), 8.0 * (1u64 << 30) as f64)),
            facts: Facts::default(),
        }
    }

    #[test]
    fn cpu_stats_mark_alerts_and_a_peak_without_samples() {
        let hot = frame(105.0);
        let stats = source(&hot).stats();
        assert_eq!(stats[0].value, "37.3\u{A0}%");
        assert!(stats[1].alert && stats[1].value == "105 °C");
        assert_eq!(
            stats[1].caption.as_deref(),
            Some("above alert threshold 80 °C")
        );
        // Load, temperature, peak, speed and up time.
        assert_eq!(stats.len(), 5);
        assert_eq!(stats[2].value, "—");
        assert_eq!(stats[2].caption.as_deref(), Some("no data"));
        let cool = frame(50.0);
        assert!(!source(&cool).stats()[1].alert);
        let gpu = StatsSource {
            device: Box::leak(Box::new(Device::of(Resource::Gpu))),
            ..source(&cool)
        }
        .stats();
        assert_eq!(gpu[3].value, "2.1 / 8.0 GB");
        assert!(gpu[0]
            .hint
            .as_deref()
            .is_some_and(|h| h.starts_with("Load of the busiest GPU engine")));
        let markup = StatsPanel {
            design: Design { dark: false },
            compact: false,
            available: f64::INFINITY,
        }
        .markup(&stats, &StatWidths::default());
        assert!(markup.contains("#C42B1C") && markup.contains(r#"FontSize="20""#));
    }

    #[test]
    fn peak_is_the_hottest_moment_and_the_scale_explains_itself_in_a_hint() {
        let timeline = [frame(61.0), frame(74.0), frame(58.0)].map(Arc::new);
        let now = frame(58.0);
        let cpu = StatsSource {
            timeline: &timeline,
            ..source(&now)
        }
        .stats();
        assert_eq!(cpu[2].value, "74 °C");
        let disk = StatsSource {
            device: Box::leak(Box::new(Device::of(Resource::Disk))),
            ..source(&now)
        }
        .stats();
        let scale = disk.last().unwrap();
        assert_eq!(
            (scale.caption.as_deref(), scale.hint.is_some()),
            (None, true)
        );
        let markup = StatsPanel {
            design: Design { dark: false },
            compact: false,
            available: f64::INFINITY,
        }
        .markup(&disk, &StatWidths::default());
        assert_eq!(markup.matches(r#"x:Name="HintCard"#).count(), 1);
    }

    #[test]
    fn memory_in_use_carries_its_share() {
        let mut memory = frame(50.0);
        memory.totals = vec![
            ("ram_used".into(), Some(26.2 * (1u64 << 30) as f64)),
            ("ram".into(), Some(96.0)),
        ];
        let ram = StatsSource {
            device: Box::leak(Box::new(Device::of(Resource::Ram))),
            ..source(&memory)
        }
        .stats();
        assert_eq!(ram[0].value, "26.2 GB (96\u{A0}%)");
        assert!(ram[0].alert && ram.len() == 3);
    }

    #[test]
    fn live_values_are_formatted_and_wifi_shows_its_signal() {
        use crate::platform::system_activity::SystemActivity;
        let now = frame(50.0);
        let facts = Facts {
            frequency: Some(4620.0),
            activity: Some(SystemActivity {
                processes: 312,
                uptime: std::time::Duration::from_secs(((3 * 24 + 4) * 60 + 15) * 60),
            }),
            signal: Some(-54),
            ..Facts::default()
        };
        let cpu = StatsSource {
            facts,
            ..source(&now)
        }
        .stats();
        let values: Vec<_> = cpu[3..].iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, ["4.62 GHz", "3\u{A0}d 4\u{A0}h"]);
        let wifi = Device::new(crate::platform::devices::DeviceId::parse("net@Wi‑Fi").unwrap())
            .unwrap()
            .with_link(Some("Wi‑Fi"));
        let net = StatsSource {
            device: Box::leak(Box::new(wifi)),
            facts,
            ..source(&now)
        }
        .stats();
        let signal = &net[3];
        assert_eq!(
            (signal.value.as_str(), signal.caption.as_deref()),
            ("−54 dBm", Some("good"))
        );
        assert_eq!(net.len(), 5);
        assert_eq!(net[4].value, "Wi‑Fi");
        // A past moment has no live values.
        assert_eq!(StatsSource { ..source(&now) }.stats()[3].value, "—");
    }

    #[test]
    fn compact_windows_keep_the_statistics_that_fit_one_row() {
        let now = frame(50.0);
        let stats = || source(&now).stats();
        let mut widths = StatWidths::default();
        for index in 0..stats().len() {
            widths.grow(index, 100.0);
        }
        let shown = |compact, available| {
            StatsPanel {
                design: Design { dark: false },
                compact,
                available,
            }
            .fitting(stats(), &widths)
            .len()
        };
        // 100 px columns 28 px apart: three fit in 400 px.
        assert_eq!(shown(true, 400.0), 3);
        assert_eq!(shown(true, f64::INFINITY), 5);
        // Full windows wrap instead.
        assert_eq!(shown(false, 400.0), 5);
    }

    #[test]
    fn statistics_wrap_before_the_available_width() {
        let now = frame(50.0);
        let stats = source(&now).stats();
        let mut widths = StatWidths::default();
        for index in 0..stats.len() {
            widths.grow(index, 100.0);
        }
        let panel = |available| StatsPanel {
            design: Design { dark: false },
            compact: false,
            available,
        };
        // 100 px columns 36 px apart: three fit in 400 px, all five in one row without a limit.
        assert_eq!(panel(400.0).rows(&stats, &widths), [0, 3]);
        assert_eq!(panel(f64::INFINITY).rows(&stats, &widths), [0]);
        let markup = panel(400.0).markup(&stats, &widths);
        assert_eq!(
            markup
                .matches(r#"Orientation="Horizontal" Spacing="36""#)
                .count(),
            2
        );
        assert_ne!(
            panel(400.0).shape(&stats, &widths),
            panel(f64::INFINITY).shape(&stats, &widths)
        );
    }

    #[test]
    fn stat_columns_only_widen_within_a_section() {
        let mut widths = StatWidths::default();
        assert!(!widths.track((Resource::Cpu, false)));
        widths.grow(1, 80.0);
        widths.grow(1, 60.0);
        assert!(widths.track((Resource::Cpu, false)));
        assert_eq!((widths.get(0), widths.get(1)), (0.0, 80.0));
        assert!(!widths.track((Resource::Disk, false)));
        assert_eq!(widths.get(1), 0.0);
    }
}
