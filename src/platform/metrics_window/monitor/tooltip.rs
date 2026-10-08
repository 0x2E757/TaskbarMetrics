use super::{
    chart::{ChartLayout, ChartRenderer, MissedSamples},
    design::Design,
    groups::NameGroups,
    locale::Language,
    model::{Device, Resource},
    pins::PinnedLayer,
    ui::Ui,
};

use crate::platform::process_history::{memory::MemoryRows, store::Frame};
use std::sync::Arc;

/// Pointer guide and a card with the values under the cursor: 220 px, 300 px for
/// RAM in megabytes and the read/receive and write/send pair.
pub struct ChartTooltip {
    pub design: Design,
    pub language: Language,
    /// The busiest processes are added up by name, as the table rows are.
    pub grouped: bool,
}

impl ChartTooltip {
    fn width(resource: Resource) -> f64 {
        match resource {
            // Two rates on a line, or megabytes with hundredths.
            Resource::Ram | Resource::Disk | Resource::Net => 300.0,
            _ => 220.0,
        }
    }

    fn value(&self, resource: Resource, values: [Option<f64>; 2]) -> Option<String> {
        let number = |v: Option<f64>| {
            v.map(|v| self.language.number(v, 2))
                .unwrap_or_else(|| "—".into())
        };
        match resource {
            Resource::Cpu | Resource::Gpu => values[0].map(|v| self.language.percent(v, 1)),
            Resource::Ram => values[0].map(|v| format!("{} MB", self.language.number(v, 2))),
            // Read/receive and write/send on one line.
            _ => (values[0].is_some() || values[1].is_some())
                .then(|| format!("↓ {}  ↑ {}", number(values[0]), number(values[1]))),
        }
    }

    fn row(&self, mark: String, label: &str, value: Option<String>) -> String {
        let d = self.design;
        let value = match value {
            Some(value) => format!(
                r#"<TextBlock Grid.Column="2" Text="{}" FontSize="12" Typography.NumeralAlignment="Tabular"/>"#,
                Ui::xml(&value)
            ),
            None => format!(
                r#"<TextBlock Grid.Column="2" Text="{}" FontSize="12" FontStyle="Italic" Foreground="{}"/>"#,
                self.language.text("no data"),
                d.color("text3")
            ),
        };
        format!(
            // The value follows the name, as in the artboard, instead of a right-aligned column.
            r#"<Grid ColumnSpacing="6"><Grid.ColumnDefinitions><ColumnDefinition Width="12"/><ColumnDefinition Width="Auto"/><ColumnDefinition/></Grid.ColumnDefinitions><Grid Height="16" VerticalAlignment="Top">{mark}</Grid><TextBlock Grid.Column="1" Text="{}" MaxWidth="130" FontSize="12" TextTrimming="CharacterEllipsis" VerticalAlignment="Top"/>{value}</Grid>"#,
            Ui::xml(label)
        )
    }

    /// Rows of the three busiest processes at `frame`, shown while nothing is pinned;
    /// grouped as the table is, the three busiest names with their sums.
    /// Idle processes are left out and their places hold a dash, so the card keeps
    /// its height as the pointer moves. The grey rows of memory outside processes
    /// are left out too: the kernel and the file cache would nearly always win.
    fn busiest(&self, frame: &Frame, device: &Device) -> Vec<String> {
        const COUNT: usize = 3;
        // The sum of a pair, the only value elsewhere.
        const SUM: usize = 2;
        let mark = format!(
            r#"<Ellipse Width="6" Height="6" Fill="{}" HorizontalAlignment="Center" VerticalAlignment="Center"/>"#,
            self.design.color("text3")
        );
        let ranked = device
            .ranked(frame, "", SUM, None)
            .into_iter()
            .filter(|&index| frame.processes[index].identity.pid != MemoryRows::PID)
            .collect();
        let weight = |index: usize| device.weight(frame, &frame.processes[index], SUM, None);
        let mut rows: Vec<_> = NameGroups::rows(frame, ranked, self.grouped, weight)
            .into_iter()
            .take_while(|group| group.iter().map(|&index| weight(index)).sum::<f64>() > 0.0)
            .take(COUNT)
            .map(|group| {
                let values = group
                    .iter()
                    .map(|&index| device.plotted(frame, &frame.processes[index]));
                self.row(
                    mark.clone(),
                    self.language.process(&frame.processes[group[0]].identity),
                    self.value(device.resource, Device::sum(values)),
                )
            })
            .collect();
        rows.resize(COUNT, self.placeholder());
        rows
    }

    /// An empty place among the busiest processes: a hollow mark and a dash.
    fn placeholder(&self) -> String {
        let color = self.design.color("text3");
        format!(
            r#"<Grid ColumnSpacing="6"><Grid.ColumnDefinitions><ColumnDefinition Width="12"/><ColumnDefinition/></Grid.ColumnDefinitions><Grid Height="16"><Ellipse Width="6" Height="6" Stroke="{color}" StrokeThickness="1" HorizontalAlignment="Center" VerticalAlignment="Center"/></Grid><TextBlock Grid.Column="1" Text="—" FontSize="12" Foreground="{color}"/></Grid>"#
        )
    }

    pub fn markup(
        &self,
        (x, y): (f64, f64),
        layout: ChartLayout,
        frames: &[Arc<Frame>],
        device: &Device,
        pinned: &[PinnedLayer],
    ) -> String {
        let resource = device.resource;
        let d = self.design;
        let mut out = String::from(
            r#"<Canvas xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" IsHitTestVisible="False">"#,
        );
        let Some(end) = frames.last().map(|f| f.bucket) else {
            out.push_str("</Canvas>");
            return out;
        };
        let guide = x.clamp(layout.x0(), layout.x1());
        out.push_str(&format!(
            r#"<Line X1="{guide:.2}" X2="{guide:.2}" Y1="{:.2}" Y2="{:.2}" Stroke="{}" StrokeThickness="1" StrokeDashArray="3,3"/>"#,
            layout.y0(),
            layout.y1(),
            d.color("text3")
        ));
        let bucket = layout.bucket(x, end);
        let frame = frames
            .iter()
            .min_by(|a, b| {
                (a.bucket as f64 - bucket)
                    .abs()
                    .total_cmp(&(b.bucket as f64 - bucket).abs())
            })
            .filter(|f| (f.bucket as f64 - bucket).abs() <= 2.0);
        let mut rows = Vec::new();
        match frame {
            Some(frame) => {
                rows.push(format!(
                    r#"<TextBlock Text="{}" FontSize="12" FontWeight="SemiBold" Typography.NumeralAlignment="Tabular"/>"#,
                    self.language.time(frame.bucket)
                ));
                if MissedSamples(frames).under(bucket) {
                    rows.push(format!(
                        r#"<TextBlock Text="{}" FontSize="11" FontStyle="Italic" Foreground="{}"/>"#,
                        Ui::xml(self.language.text("Some samples are missing here")),
                        d.color("text2")
                    ));
                }
                rows.push(self.row(
                    format!(
                        r#"<Border Height="2" VerticalAlignment="Center" Background="{}"/>"#,
                        d.color("total")
                    ),
                    self.language.text("Total"),
                    self.value(resource, device.total(frame)),
                ));
                if resource.temperature_axis() {
                    rows.push(self.row(
                        format!(
                            r#"<Line X1="0" X2="12" Y1="1" Y2="1" Height="2" Stroke="{}" StrokeThickness="2" StrokeDashArray="1.5,1" VerticalAlignment="Center"/>"#,
                            d.color("temp")
                        ),
                        self.language.text("Temperature"),
                        device.temperature(frame).map(|v| self.language.celsius(v)),
                    ));
                }
                for layer in pinned {
                    let values =
                        ChartRenderer::series_of(std::slice::from_ref(frame), device, layer.key)[0];
                    let value = self.value(resource, values);
                    let swatch = if value.is_some() {
                        format!(
                            r#"<Rectangle Width="10" Height="10" RadiusX="2.25" RadiusY="2.25" Fill="{}" Stroke="{}" StrokeThickness="1.5"/>"#,
                            Design::alpha(&layer.color, 0.35),
                            layer.color
                        )
                    } else {
                        format!(
                            r#"<Rectangle Width="10" Height="10" RadiusX="2.25" RadiusY="2.25" Stroke="{}" StrokeThickness="1.5" StrokeDashArray="2,1.5"/>"#,
                            layer.color
                        )
                    };
                    rows.push(self.row(swatch, &layer.name, value));
                }
                if pinned.is_empty() {
                    rows.extend(self.busiest(frame, device));
                }
            }
            None => rows.push(format!(
                r#"<TextBlock Text="{}" FontSize="12" FontWeight="SemiBold"/>"#,
                self.language.text("No samples")
            )),
        }
        let width = Self::width(resource);
        let height = 20.0 + rows.len() as f64 * 20.0;
        let left = if x + 12.0 + width > layout.x1() {
            x - 12.0 - width
        } else {
            x + 12.0
        };
        let top = (y + 12.0).min(layout.height - height).max(0.0);
        out.push_str(&format!(
            r#"<Border Canvas.Left="{:.2}" Canvas.Top="{top:.2}" Width="{}" Padding="10,8" CornerRadius="6" Background="{}" BorderBrush="{}" BorderThickness="1"><StackPanel Spacing="4">{}</StackPanel></Border></Canvas>"#,
            left.max(0.0),
            width,
            // 20 % transparent: the lines under the card stay visible.
            Design::alpha(d.color("card"), 0.8),
            d.color("border"),
            rows.concat()
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_flips_near_the_right_edge_and_reports_missing_samples() {
        let frame = Arc::new(Frame {
            bucket: 5000,
            elapsed_ms: 500.0,
            processes: vec![],
            gpu_engines: Vec::new(),
            totals: vec![("cpu".into(), Some(43.1))],
            etw_active: true,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 0.0,
        });
        let tooltip = ChartTooltip {
            design: Design { dark: false },
            language: Language::English,
            grouped: false,
        };
        let layout = ChartLayout::new(900.0, 250.0);
        let pinned = [PinnedLayer {
            key: (1, 2),
            color: "#C25400".into(),
            name: "Code.exe".into(),
        }];
        let near = tooltip.markup(
            (layout.x1(), 60.0),
            layout,
            std::slice::from_ref(&frame),
            &Device::of(Resource::Cpu),
            &pinned,
        );
        assert!(
            near.contains("43.1\u{A0}%")
                && near.contains("no data")
                && near.contains("Temperature")
                && !near.contains("Click")
        );
        assert!(near.contains(&format!(r#"Canvas.Left="{:.2}""#, layout.x1() - 232.0)));
        let far = tooltip.markup(
            (layout.x0(), 60.0),
            layout,
            &[frame],
            &Device::of(Resource::Cpu),
            &[],
        );
        assert!(far.contains("No samples"));
        let disk = Arc::new(Frame {
            bucket: 5000,
            elapsed_ms: 500.0,
            processes: vec![],
            gpu_engines: Vec::new(),
            totals: vec![
                ("disk_read".into(), Some(1.5)),
                ("disk_write".into(), Some(0.25)),
            ],
            etw_active: true,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 0.0,
        });
        let split = tooltip.markup(
            (layout.x1(), 60.0),
            layout,
            &[disk],
            &Device::of(Resource::Disk),
            &[],
        );
        assert!(split.contains("↓ 1.50  ↑ 0.25") && !split.contains("Temperature"));
    }

    #[test]
    fn a_missed_moment_shows_the_nearest_sample_with_a_note() {
        let frame = |bucket| {
            Arc::new(Frame {
                bucket,
                elapsed_ms: 500.0,
                processes: vec![],
                gpu_engines: Vec::new(),
                totals: vec![("cpu".into(), Some(43.1))],
                etw_active: true,
                lost_events: 0,
                undecoded: 0,
                sample_ms: 0.0,
            })
        };
        let tooltip = ChartTooltip {
            design: Design { dark: false },
            language: Language::English,
            grouped: false,
        };
        let layout = ChartLayout::new(900.0, 250.0);
        let frames = [frame(5000), frame(5002)];
        // The middle of the column of bucket 5001, the second to last of the window.
        let missed = layout.x0() + (layout.x1() - layout.x0()) * 598.5 / 600.0;
        let note = "Some samples are missing here";
        let markup =
            |x| tooltip.markup((x, 60.0), layout, &frames, &Device::of(Resource::Cpu), &[]);
        assert!(markup(missed).contains(note));
        assert!(!markup(layout.x1()).contains(note));
    }

    #[test]
    fn without_pins_the_three_busiest_processes_are_shown() {
        use crate::platform::process_history::store::{Identity, Sample};
        let sample = |pid, cpu| {
            Sample::new(
                Arc::new(Identity {
                    pid,
                    created: 1,
                    name: format!("p{pid}.exe").into(),
                }),
                Some(cpu),
                None,
                [None; 3],
                None,
            )
        };
        let frame = Arc::new(Frame {
            bucket: 5000,
            elapsed_ms: 500.0,
            processes: vec![
                sample(1, 5.0),
                sample(2, 40.0),
                sample(3, 0.0),
                sample(4, 12.5),
                sample(5, 1.0),
            ],
            gpu_engines: Vec::new(),
            totals: vec![("cpu".into(), Some(60.0))],
            etw_active: true,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 0.0,
        });
        let tooltip = ChartTooltip {
            design: Design { dark: false },
            language: Language::English,
            grouped: false,
        };
        let layout = ChartLayout::new(900.0, 250.0);
        let device = Device::of(Resource::Cpu);
        let markup = tooltip.markup(
            (layout.x1(), 60.0),
            layout,
            std::slice::from_ref(&frame),
            &device,
            &[],
        );
        let at = |name: &str| markup.find(name);
        assert!(at("p2.exe") < at("p4.exe") && at("p4.exe") < at("p1.exe"));
        assert!(at("p2.exe").is_some() && at("p5.exe").is_none() && at("p3.exe").is_none());
        // A quiet moment keeps three rows: dashes stand for the idle processes.
        let quiet = Arc::new(Frame {
            processes: vec![sample(1, 5.0), sample(3, 0.0)],
            ..(*frame).clone()
        });
        let markup = tooltip.markup((layout.x1(), 60.0), layout, &[quiet], &device, &[]);
        assert!(markup.contains("p1.exe") && markup.matches(r#"Text="—""#).count() == 2);
        let pinned = [PinnedLayer {
            key: (5, 1),
            color: "#C25400".into(),
            name: "p5.exe".into(),
        }];
        let markup = tooltip.markup((layout.x1(), 60.0), layout, &[frame], &device, &pinned);
        assert!(markup.contains("p5.exe") && !markup.contains("p2.exe"));
    }

    #[test]
    fn grouped_the_three_busiest_names_show_their_sums() {
        use crate::platform::process_history::store::{Identity, Sample};
        let sample = |pid, name: &str, cpu| {
            Sample::new(
                Arc::new(Identity {
                    pid,
                    created: 1,
                    name: name.into(),
                }),
                Some(cpu),
                None,
                [None; 3],
                None,
            )
        };
        let frame = Arc::new(Frame {
            bucket: 5000,
            elapsed_ms: 500.0,
            processes: vec![
                sample(1, "code.exe", 30.0),
                sample(2, "chrome.exe", 20.0),
                sample(3, "Chrome.exe", 15.0),
                sample(4, "chrome.exe", 5.0),
                sample(5, "steam.exe", 4.0),
                sample(6, "idle.exe", 0.0),
            ],
            gpu_engines: Vec::new(),
            totals: vec![("cpu".into(), Some(80.0))],
            etw_active: true,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 0.0,
        });
        let tooltip = ChartTooltip {
            design: Design { dark: false },
            language: Language::English,
            grouped: true,
        };
        let layout = ChartLayout::new(900.0, 250.0);
        let markup = tooltip.markup(
            (layout.x1(), 60.0),
            layout,
            &[frame],
            &Device::of(Resource::Cpu),
            &[],
        );
        let at = |text: &str| markup.find(text);
        // Chrome's 40 % outweigh Code's 30 %, under the name of its busiest process.
        assert!(at("chrome.exe") < at("code.exe") && at("code.exe") < at("steam.exe"));
        assert!(markup.contains("40.0\u{A0}%") && !markup.contains("20.0\u{A0}%"));
        assert!(at("Chrome.exe").is_none() && at("idle.exe").is_none());
    }

    #[test]
    fn memory_outside_processes_is_not_among_the_busiest() {
        use crate::platform::process_history::{
            memory::SystemMemory,
            store::{Identity, Sample},
        };
        let mut processes: Vec<_> = [(10, 300u64 << 20), (11, 200 << 20)]
            .map(|(pid, bytes)| {
                Sample::new(
                    Arc::new(Identity {
                        pid,
                        created: 1,
                        name: format!("p{pid}.exe").into(),
                    }),
                    None,
                    None,
                    [Some(bytes), None, None],
                    None,
                )
            })
            .into();
        let memory = SystemMemory {
            used: 8 << 30,
            nonpaged_pool: 1 << 30,
            paged_pool: 1 << 30,
            drivers: 1 << 30,
            modified: 1 << 30,
            file_cache: 1 << 30,
        };
        MemoryRows::new().append(&mut processes, &memory, 0);
        let frame = Arc::new(Frame {
            bucket: 5000,
            elapsed_ms: 500.0,
            processes,
            gpu_engines: Vec::new(),
            totals: vec![("ram".into(), Some(8192.0))],
            etw_active: true,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 0.0,
        });
        let tooltip = ChartTooltip {
            design: Design { dark: false },
            language: Language::English,
            grouped: false,
        };
        let layout = ChartLayout::new(900.0, 250.0);
        let markup = tooltip.markup(
            (layout.x1(), 60.0),
            layout,
            &[frame],
            &Device::of(Resource::Ram),
            &[],
        );
        assert!(markup.contains("p10.exe") && markup.contains("p11.exe"));
        assert!(!markup.contains("Kernel") && markup.matches(r#"Text="—""#).count() == 1);
    }
}
