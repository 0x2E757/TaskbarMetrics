use super::{design::Design, locale::Language, model::Resource, ui::Ui};
use crate::platform::process_history::memory::MemoryRows;

/// How a row relates to the pinned set and the selected moment.
#[derive(Clone, Copy, PartialEq)]
pub enum RowKind {
    Regular,
    Pinned,
    /// Pinned instance without a sample at the moment.
    PinnedMissing,
}

/// One process row: identity, values at the moment and pin state.
pub struct RowContent<'a> {
    pub index: usize,
    pub name: &'a str,
    pub pid: u32,
    /// Other processes of the same name added up in the row.
    pub more: usize,
    pub kind: RowKind,
    /// Pin palette color of pinned rows.
    pub color: Option<&'a str>,
    pub values: Option<[Option<f64>; 2]>,
    /// Bytes read and written (received and sent) since the recorder started.
    pub total: Option<[u64; 2]>,
    /// Pinning is refused because the palette is exhausted.
    pub locked: bool,
    /// The row's history is highlighted on the chart.
    pub hovered: bool,
}

/// One column contract for header, process rows and the fixed aggregate footer.
pub struct TableLayout {
    /// Value column the rows are sorted by.
    pub sort: usize,
    pub resource: Resource,
    pub compact: bool,
    /// Rows stand for every process of a name: the PID column fits «27564 (+12)».
    pub grouped: bool,
    pub design: Design,
    pub language: Language,
    /// Width of the table card, used to trim long process names.
    pub width: f64,
}

impl TableLayout {
    fn widths(&self) -> &'static [u32] {
        match self.resource {
            Resource::Cpu if self.compact => &[90],
            Resource::Cpu => &[110],
            Resource::Gpu => &[110],
            Resource::Ram => &[104, 104, 104],
            // One grid for both: switching between them moves no column.
            Resource::Disk | Resource::Net if self.compact => &[104, 104, 104],
            Resource::Disk | Resource::Net => &[104, 104, 104, 196],
        }
    }

    /// Disk and network pages show the bytes since the recorder started, where the
    /// window is wide enough.
    pub fn totals(&self) -> bool {
        self.resource.dual() && !self.compact
    }

    /// Space before the total and its title, taken from the PID column: the sort
    /// chevron (12 px and 4 px apart) appears in it without moving the title.
    const INDENT: u32 = 20;
    const CHEVRON: u32 = 16;
    /// Room of « (+12)» after a grouped row's PID.
    const MORE: u32 = 24;

    fn pid_width(&self) -> u32 {
        let width = if self.grouped { 72 + Self::MORE } else { 72 };
        if self.totals() {
            width - Self::INDENT
        } else {
            width
        }
    }

    /// The busiest process's PID, and how many more of its name the row adds up.
    fn pid(row: &RowContent) -> String {
        match row.more {
            0 => row.pid.to_string(),
            more => format!("{} (+{more})", row.pid),
        }
    }

    /// Columns the header sorts by: both values, their sum and the total.
    pub fn sortable(&self) -> usize {
        3 + usize::from(self.totals())
    }

    /// Sorted by `column`, or by the sum where this table has no total column.
    pub fn sorted_by(mut self, column: usize) -> Self {
        self.sort = column.min(self.sortable() - 1);
        self
    }

    /// The sum and «(1500.00 + 320.00)», each direction: all in MB like the header,
    /// so that rows compare at a glance, always to hundredths and without
    /// thousands separators; «0.00» and nothing else without I/O.
    fn amount([first, second]: [u64; 2]) -> (String, String) {
        let number = |bytes: u64| format!("{:.2}", bytes as f64 / 1e6);
        let sum = first + second;
        if sum == 0 {
            return (number(0), String::new());
        }
        (
            number(sum),
            format!("({} + {})", number(first), number(second)),
        )
    }

    /// Left-aligned, as the lengths differ a lot: the sum's whole part keeps the
    /// room of «00000» right-aligned and its fraction that of «.00», so that the
    /// decimal points and the brackets after them stand in one line.
    fn total(&self, total: Option<[u64; 2]>, color: &str) -> String {
        let Some(total) = total.filter(|_| self.totals()) else {
            return String::new();
        };
        let (sum, parts) = Self::amount(total);
        let (whole, fraction) = Self::split(&sum);
        let (whole_room, fraction_room) = Self::split("00000.00");
        let text = |column: usize, text: &str, extra: &str| {
            format!(
                r#"<TextBlock Grid.Column="{column}" Text="{}" Foreground="{color}" VerticalAlignment="Center" FontSize="13" Typography.NumeralAlignment="Tabular" {extra}/>"#,
                Ui::xml(text)
            )
        };
        let hidden = r#"Opacity="0" AutomationProperties.AccessibilityView="Raw""#;
        format!(
            r#"<Grid Grid.Column="{}" Margin="{},0,0,0"><Grid.ColumnDefinitions><ColumnDefinition Width="Auto"/><ColumnDefinition Width="Auto"/><ColumnDefinition/></Grid.ColumnDefinitions>{}{}{}{}{}</Grid>"#,
            self.first_value() + 3,
            Self::INDENT,
            text(0, whole_room, hidden),
            text(0, whole, r#"HorizontalAlignment="Right""#),
            text(1, fraction_room, hidden),
            text(1, fraction, ""),
            text(
                2,
                &parts,
                r#"Margin="4,0,0,0" TextTrimming="CharacterEllipsis""#
            ),
        )
    }

    /// «12.5» → «12» and «.5».
    fn split(number: &str) -> (&str, &str) {
        number.split_at(number.find('.').unwrap_or(number.len()))
    }

    fn first_value(&self) -> usize {
        if self.compact {
            2
        } else {
            3
        }
    }

    pub fn columns(&self) -> String {
        let mut result = String::from(
            r#"<Grid.ColumnDefinitions><ColumnDefinition Width="28"/><ColumnDefinition/>"#,
        );
        if !self.compact {
            result.push_str(&format!(
                r#"<ColumnDefinition Width="{}"/>"#,
                self.pid_width()
            ));
        }
        for width in self.widths() {
            result.push_str(&format!(r#"<ColumnDefinition Width="{width}"/>"#));
        }
        result.push_str("</Grid.ColumnDefinitions>");
        result
    }

    /// Space left for the name after the pin, swatch, PID and value columns.
    fn name_width(&self) -> f64 {
        let fixed: u32 = self.widths().iter().map(|w| w + 12).sum::<u32>()
            + if self.compact {
                0
            } else {
                self.pid_width() + 12
            };
        (self.width - 16.0 - 16.0 - 40.0 - 18.0 - fixed as f64).max(60.0)
    }

    pub fn values(&self, values: [Option<f64>; 2], color: &str) -> String {
        let number = |v: Option<f64>| {
            v.map(|v| self.language.number(v, 2))
                .unwrap_or_else(|| "—".into())
        };
        let strings = match self.resource {
            // Read and write, then their sum.
            r if r.paired() => vec![
                number(values[0]),
                number(values[1]),
                number(
                    values[0]
                        .zip(values[1])
                        .map(|(a, b)| a + b)
                        .or(values[0])
                        .or(values[1]),
                ),
            ],
            Resource::Cpu | Resource::Gpu => vec![values[0]
                .map(|v| self.language.percent(v, 2))
                .unwrap_or_else(|| "—".into())],
            _ => vec![number(values[0])],
        };
        strings.iter().enumerate().map(|(i,s)|format!(r#"<TextBlock Grid.Column="{}" Text="{}" Foreground="{color}" HorizontalAlignment="Right" VerticalAlignment="Center" FontSize="13" Typography.NumeralAlignment="Tabular" TextTrimming="CharacterEllipsis"/>"#,i+self.first_value(),Ui::xml(s))).collect()
    }

    /// Italic note over the value columns; a single value column also lends it the PID column.
    fn missing(&self) -> String {
        let borrow = usize::from(self.widths().len() == 1 && !self.compact);
        format!(
            r#"<TextBlock Grid.Column="{}" Grid.ColumnSpan="{}" Text="{}" FontSize="12" FontStyle="Italic" Foreground="{}" HorizontalAlignment="Right" VerticalAlignment="Center" TextTrimming="CharacterEllipsis"/>"#,
            self.first_value() - borrow,
            self.widths().len() + borrow,
            self.language.text("No data at this moment"),
            self.design.color("text3")
        )
    }

    pub fn header(&self) -> String {
        let d = self.design;
        let titles: &[&str] = match self.resource {
            Resource::Cpu => &["CPU"],
            Resource::Gpu => &["GPU"],
            Resource::Ram => &["Private", "Shared", "Both"],
            Resource::Disk => &["Read", "Write", "Both"],
            Resource::Net => &["Receive", "Send", "Both"],
        };
        let mut markup = format!(
            r#"<Grid xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Height="30" Padding="4,0,12,0" ColumnSpacing="12" BorderBrush="{}" BorderThickness="0,0,0,1">{}<TextBlock Grid.Column="1" Text="{}" FontSize="12" Foreground="{}" VerticalAlignment="Center"/>"#,
            d.color("divider"),
            self.columns(),
            self.language.text("Process"),
            d.color("text3")
        );
        if !self.compact {
            markup.push_str(&format!(r#"<TextBlock Grid.Column="2" Text="PID" FontSize="12" Foreground="{}" VerticalAlignment="Center"/>"#,d.color("text3")));
        }
        let unit = match self.resource {
            Resource::Ram => ", MB",
            r if r.dual() => ", MB/s",
            _ => "",
        };
        let columns = titles
            .iter()
            .map(|title| (*title, unit))
            .chain(self.totals().then_some(("All time", ", MB")));
        for (index, (title, unit)) in columns.enumerate() {
            let sorted = index == self.sort || !self.resource.paired();
            let color = d.color(if sorted { "text" } else { "text3" });
            // The chevron leads: the right-aligned title keeps its place when the
            // sorted column changes, and the left-aligned total's title stands over
            // the indented values with the chevron in the indent.
            let indent = match index {
                3 if sorted => Self::INDENT - Self::CHEVRON,
                3 => Self::INDENT,
                _ => 0,
            };
            let label = format!(
                r#"<StackPanel Orientation="Horizontal" Spacing="4" Margin="{indent},0,0,0">{}<TextBlock Text="{}{unit}" FontSize="12" Foreground="{color}" VerticalAlignment="Center" TextTrimming="CharacterEllipsis"/></StackPanel>"#,
                if sorted {
                    d.icon_sized("chev", color, 12.0, 1.3)
                } else {
                    String::new()
                },
                self.language.text(title),
            );
            let column = index + self.first_value();
            // Two values (read and write): each header sorts the rows by its column.
            // The padding is given back as margin, so the text keeps the column edge:
            // the right one, or the left one over the left-aligned total.
            let (align, margin) = if index == 3 {
                ("Left", "-4,0,0,0")
            } else {
                ("Right", "0,0,-4,0")
            };
            markup.push_str(&if self.resource.paired() {
                format!(
                    r#"<Button x:Name="Sort{index}" Grid.Column="{column}" HorizontalAlignment="{align}" Height="24" MinHeight="0" Padding="4,0" Margin="{margin}" Background="Transparent" BorderThickness="0" AutomationProperties.Name="{}{unit}">{label}</Button>"#,
                    self.language.text(title)
                )
            } else {
                label.replacen(
                    "<StackPanel ",
                    &format!(r#"<StackPanel Grid.Column="{column}" HorizontalAlignment="Right" "#),
                    1,
                )
            });
        }
        markup.push_str("</Grid>");
        markup
    }

    pub fn divider(&self) -> String {
        format!(
            r#"<Border Height="1" Margin="44,4,8,0" Background="{}"/>"#,
            self.design.color("divider")
        )
    }

    /// Quiet line with an icon, e.g. the empty pinned section or «no matches»; as compact
    /// as in the artboards, where the overflowing list shrinks it to its content.
    pub fn note(&self, icon: Option<&str>, text: &str) -> String {
        let color = self.design.color("text3");
        format!(
            r#"<StackPanel Orientation="Horizontal" Spacing="8" Padding="44,0,12,0">{}<TextBlock Text="{}" FontSize="12" Foreground="{color}" VerticalAlignment="Center" TextTrimming="CharacterEllipsis"/></StackPanel>"#,
            icon.map(|i| self.design.icon_sized(i, color, 14.0, 1.3))
                .unwrap_or_default(),
            Ui::xml(text)
        )
    }

    fn swatch(&self, row: &RowContent) -> String {
        match (row.kind, row.color) {
            (RowKind::Pinned, Some(color)) => format!(
                r#"<Rectangle Width="10" Height="10" RadiusX="2.25" RadiusY="2.25" Fill="{}" Stroke="{color}" StrokeThickness="1.5" VerticalAlignment="Center"/>"#,
                Design::alpha(color, 0.35)
            ),
            (RowKind::PinnedMissing, Some(color)) => format!(
                r#"<Rectangle Width="10" Height="10" RadiusX="2.25" RadiusY="2.25" Stroke="{color}" StrokeThickness="1.5" StrokeDashArray="2,1.5" VerticalAlignment="Center"/>"#
            ),
            _ => String::new(),
        }
    }

    pub fn row(&self, row: &RowContent) -> String {
        let d = self.design;
        let pinned = row.kind != RowKind::Regular;
        let label = self.language.text(if pinned { "Unpin" } else { "Pin" });
        let tooltip = if row.locked {
            self.language.text("Up to 6 pinned")
        } else {
            label
        };
        let icon = d.icon_sized(
            if pinned { "pinf" } else { "pin" },
            d.color(if pinned { "text" } else { "text3" }),
            14.0,
            1.3,
        );
        let values = match (row.kind, row.values) {
            (RowKind::PinnedMissing, _) | (_, None) => self.missing(),
            (_, Some(values)) => {
                self.values(values, d.color("text")) + &self.total(row.total, d.color("text"))
            }
        };
        // Memory rows stand for no process.
        let pid = if self.compact || row.pid == MemoryRows::PID {
            String::new()
        } else {
            format!(
                r#"<TextBlock Grid.Column="2" Text="{}" FontSize="12" Foreground="{}" VerticalAlignment="Center" Typography.NumeralAlignment="Tabular"/>"#,
                Self::pid(row),
                d.color("text3")
            )
        };
        format!(
            r#"<Grid x:Name="Row{index}" Background="{background}" Height="32" CornerRadius="4" Padding="4,0,12,0" ColumnSpacing="12">{columns}<Border x:Name="Focus{index}" Grid.ColumnSpan="9" Margin="-4,0,-12,0" CornerRadius="4" BorderBrush="{ink}" BorderThickness="2" Visibility="Collapsed" IsHitTestVisible="False"/><Button x:Name="Pin{index}" Width="28" Height="28" UseSystemFocusVisuals="False" Padding="0" MinHeight="28" BorderThickness="0" Background="Transparent" IsEnabled="{enabled}" ToolTipService.ToolTip="{tooltip}" AutomationProperties.Name="{name}">{icon}</Button><StackPanel Grid.Column="1" Orientation="Horizontal" Spacing="8">{swatch}<TextBlock Text="{title}" MaxWidth="{width:.0}" TextTrimming="CharacterEllipsis" FontSize="13" FontWeight="{weight}" Foreground="{title_color}" VerticalAlignment="Center"/></StackPanel>{pid}{values}</Grid>"#,
            index = row.index,
            ink = d.color("ink"),
            background = if row.hovered {
                d.color("rowHover")
            } else {
                "Transparent"
            },
            columns = self.columns(),
            enabled = if row.locked { "False" } else { "True" },
            name = Ui::xml(&format!("{label} {}", row.name)),
            swatch = self.swatch(row),
            title = Ui::xml(row.name),
            width = self.name_width(),
            weight = if pinned { "SemiBold" } else { "Normal" },
            // Memory outside processes is quieter than the processes around it.
            title_color = d.color(if row.pid == MemoryRows::PID {
                "textMuted"
            } else {
                "text"
            }),
        )
    }

    pub fn footer(
        &self,
        count: usize,
        values: [Option<f64>; 2],
        total: Option<[u64; 2]>,
    ) -> String {
        let color = self.design.color("text2");
        format!(
            r#"<Grid xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Height="32" Padding="4,0,12,0" ColumnSpacing="12">{}<TextBlock Grid.Column="1" Text="{} ({count})" FontSize="13" Foreground="{color}" VerticalAlignment="Center" TextTrimming="CharacterEllipsis"/>{}</Grid>"#,
            self.columns(),
            self.language.text("Other processes"),
            self.values(values, color) + &self.total(total, color)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(resource: Resource, compact: bool) -> TableLayout {
        TableLayout {
            resource,
            compact,
            grouped: false,
            design: Design { dark: false },
            language: Language::English,
            width: 980.0,
            sort: 0,
        }
    }

    #[test]
    fn rows_share_columns_and_mark_pins_without_color_alone() {
        let disk = table(Resource::Disk, false);
        let row = disk.row(&RowContent {
            index: 3,
            name: "Code & co.exe",
            pid: 27564,
            more: 0,
            kind: RowKind::PinnedMissing,
            color: Some("#C25400"),
            values: None,
            total: None,
            locked: false,
            hovered: false,
        });
        assert!(
            row.contains(r#"StrokeDashArray="2,1.5""#) && row.contains(r#"FontWeight="SemiBold""#)
        );
        assert!(
            row.contains("No data at this moment")
                && row.contains(r#"Grid.Column="3" Grid.ColumnSpan="4""#)
        );
        assert!(row.contains("Unpin Code &amp; co.exe") && !row.contains("Tag3"));
        assert!(disk.header().contains("Read, MB/s") && disk.header().contains("Viewbox"));
        let gpu = table(Resource::Gpu, true);
        let values = gpu.values([Some(0.0), None], "#000000");
        assert!(values.contains("0.00") && values.matches("<TextBlock").count() == 1);
        assert!(!gpu.header().contains("PID") && !gpu.header().contains("Engine"));
    }

    #[test]
    fn disk_and_network_show_bytes_since_the_recorder_started() {
        let net = table(Resource::Net, false);
        let header = net.header();
        assert!(
            header.contains(r#"x:Name="Sort3" Grid.Column="6""#) && header.contains("All time, MB")
        );
        assert_eq!(
            (net.sortable(), table(Resource::Net, true).sorted_by(3).sort),
            (4, 2)
        );
        let row = |total| {
            net.row(&RowContent {
                index: 0,
                name: "p.exe",
                pid: 7,
                more: 0,
                kind: RowKind::Regular,
                color: None,
                values: Some([Some(0.5), Some(0.1)]),
                total,
                locked: false,
                hovered: false,
            })
        };
        // Always MB, as in the header: rows compare without reading units.
        let large = row(Some([1_500_000_000, 320_000_000]));
        assert!(
            large.contains(r#"Text="1820""#)
                && large.contains(r#"Text=".00""#)
                && large.contains("(1500.00 + 320.00)")
        );
        // The sum is split at its decimal point, each part in the room of "00000.00".
        let small = row(Some([2_500_000, 300_000]));
        assert!(
            small.contains(r#"Text="2" Foreground"#)
                && small.contains(r#"Text=".80""#)
                && small.contains("(2.50 + 0.30)")
        );
        assert!(small.contains(r#"Text="00000""#) && small.contains(r#"Text=".00""#));
        assert!(row(Some([740_000_000, 0])).contains("(740.00 + 0.00)"));
        // The title keeps its place when the chevron appears in the indent before
        // it; the PID column gives up the room.
        let title = |sort| {
            let header = table(Resource::Net, false).sorted_by(sort).header();
            let at = header.find(r#"x:Name="Sort3""#).unwrap();
            header[at..].contains(&format!(
                r#"Margin="{},0,0,0""#,
                if sort == 3 { 4 } else { 20 }
            ))
        };
        assert!(title(0) && title(3));
        assert!(net
            .columns()
            .contains(r#"Width="52"/><ColumnDefinition Width="104""#));
        assert!(table(Resource::Ram, false)
            .columns()
            .contains(r#"Width="72""#));
        // Disk and network share the grid: switching pages moves no column.
        for compact in [false, true] {
            assert_eq!(
                table(Resource::Disk, compact).columns(),
                table(Resource::Net, compact).columns()
            );
        }
        let zero = row(Some([0, 0]));
        assert!(zero.contains(r#"Text="0" Foreground"#) && !zero.contains("+"));
        assert!(net
            .header()
            .contains(r#"HorizontalAlignment="Left" Height="24""#));
        // A narrow window has no room for it.
        assert!(!table(Resource::Net, true).header().contains("All time"));
        assert!(!table(Resource::Cpu, false).totals());
    }

    #[test]
    fn grouped_rows_show_the_busiest_pid_and_how_many_more() {
        let grouped = TableLayout {
            grouped: true,
            ..table(Resource::Net, false)
        };
        let row = grouped.row(&RowContent {
            index: 0,
            name: "chrome.exe",
            pid: 27564,
            more: 12,
            kind: RowKind::Regular,
            color: None,
            values: Some([Some(0.5), Some(0.1)]),
            total: None,
            locked: false,
            hovered: false,
        });
        assert!(row.contains(r#"Text="27564 (+12)""#));
        // The PID column widens by the room of « (+12)», less the total's indent.
        assert!(grouped.columns().contains(r#"Width="76""#));
        assert!(table(Resource::Cpu, false)
            .columns()
            .contains(r#"Width="72""#));
    }

    #[test]
    fn value_headers_of_disk_and_network_sort_and_mark_their_column() {
        let mut disk = table(Resource::Disk, false);
        disk.sort = 1;
        let header = disk.header();
        assert!(header.contains(r#"x:Name="Sort0""#) && header.contains(r#"x:Name="Sort1""#));
        // The chevron leads "Write", the sorted column.
        let (read, chevron, write) = (
            header.find(r#"Text="Read"#).unwrap(),
            header.find("Viewbox").unwrap(),
            header.find(r#"Text="Write"#).unwrap(),
        );
        assert!(read < chevron && chevron < write && !header[write..].contains("Viewbox"));
        assert!(header.contains(r#"x:Name="Sort2""#) && header.contains("Both, MB/s"));
        assert!(!table(Resource::Cpu, false).header().contains("Sort0"));
        // The sum follows read and write; one missing side still counts the other.
        let values = disk.values([Some(1.25), Some(0.5)], "#000000");
        assert!(values.matches("<TextBlock").count() == 3 && values.contains("1.75"));
        assert!(
            disk.values([None, Some(0.5)], "#000000")
                .matches("0.50")
                .count()
                == 2
        );
    }
}
