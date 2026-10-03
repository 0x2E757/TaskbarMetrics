use super::{monitor::Language, *};
use crate::platform::xaml::{TileStyle, WidgetPreview, FRAMEWORK, PANEL};

const RANGE: Guid = Guid::from_u128(0xfa002c1a_494e_46cf_91d4_e14a8d798675);
const CONTROL: Guid = Guid::from_u128(0xa8912263_2951_4f58_a9c5_5a134eaa7f07);

struct InputDefinition {
    name: &'static str,
    label: &'static str,
    min: f64,
    max: f64,
    step: f64,
}
const INPUTS: [InputDefinition; 37] = [
    InputDefinition {
        name: "TileWidth",
        label: "CPU / GPU width, px",
        min: 80.0,
        max: 180.0,
        step: 1.0,
    },
    InputDefinition {
        name: "TileGap",
        label: "Tile spacing, px",
        min: 0.0,
        max: 20.0,
        step: 1.0,
    },
    InputDefinition {
        name: "Radius",
        label: "Corner radius, px",
        min: 0.0,
        max: 18.0,
        step: 1.0,
    },
    InputDefinition {
        name: "LabelSize",
        label: "Header size, px",
        min: 7.0,
        max: 15.0,
        step: 1.0,
    },
    InputDefinition {
        name: "ValueSize",
        label: "Main value size, px",
        min: 9.0,
        max: 20.0,
        step: 1.0,
    },
    InputDefinition {
        name: "SecondarySize",
        label: "Temperature / upload size, px",
        min: 7.0,
        max: 16.0,
        step: 1.0,
    },
    InputDefinition {
        name: "LineWidth",
        label: "Line width, px",
        min: 0.5,
        max: 3.0,
        step: 0.25,
    },
    InputDefinition {
        name: "AreaOpacity",
        label: "Area opacity, %",
        min: 0.0,
        max: 100.0,
        step: 1.0,
    },
    InputDefinition {
        name: "OffsetX",
        label: "Value X offset, px",
        min: -5.0,
        max: 5.0,
        step: 1.0,
    },
    InputDefinition {
        name: "OffsetY",
        label: "Value Y offset, px",
        min: -5.0,
        max: 5.0,
        step: 1.0,
    },
    InputDefinition {
        name: "GraphLeft",
        label: "Chart left inset, px",
        min: 0.0,
        max: 60.0,
        step: 1.0,
    },
    InputDefinition {
        name: "FadeOffset",
        label: "Fade start from text edge, px",
        min: -100.0,
        max: 40.0,
        step: 1.0,
    },
    InputDefinition {
        name: "FadeWidth",
        label: "Fade length, px",
        min: 0.0,
        max: 100.0,
        step: 1.0,
    },
    InputDefinition {
        name: "FadeOpacity",
        label: "Chart visibility on the left, %",
        min: 0.0,
        max: 100.0,
        step: 1.0,
    },
    InputDefinition {
        name: "CpuX",
        label: "CPU: red tint starts at X, °C",
        min: 30.0,
        max: 119.0,
        step: 1.0,
    },
    InputDefinition {
        name: "CpuY",
        label: "CPU: pulse starts at Y, °C",
        min: 31.0,
        max: 120.0,
        step: 1.0,
    },
    InputDefinition {
        name: "GpuX",
        label: "GPU: red tint starts at X, °C",
        min: 30.0,
        max: 119.0,
        step: 1.0,
    },
    InputDefinition {
        name: "GpuY",
        label: "GPU: pulse starts at Y, °C",
        min: 31.0,
        max: 120.0,
        step: 1.0,
    },
    InputDefinition {
        name: "RamX",
        label: "RAM: red tint starts at X, %",
        min: 0.0,
        max: 99.0,
        step: 1.0,
    },
    InputDefinition {
        name: "RamY",
        label: "RAM: pulse starts at Y, %",
        min: 1.0,
        max: 100.0,
        step: 1.0,
    },
    InputDefinition {
        name: "AlertOpacity",
        label: "Pulse intensity, %",
        min: 0.0,
        max: 80.0,
        step: 1.0,
    },
    InputDefinition {
        name: "PulseSeconds",
        label: "Pulse period, s",
        min: 0.5,
        max: 6.0,
        step: 0.1,
    },
    InputDefinition {
        name: "DemoCpu",
        label: "Preview: CPU temperature, °C",
        min: 0.0,
        max: 130.0,
        step: 1.0,
    },
    InputDefinition {
        name: "DemoGpu",
        label: "Preview: GPU temperature, °C",
        min: 0.0,
        max: 130.0,
        step: 1.0,
    },
    InputDefinition {
        name: "DemoRam",
        label: "Preview: RAM usage, %",
        min: 0.0,
        max: 100.0,
        step: 1.0,
    },
    InputDefinition {
        name: "DemoCpuLoad",
        label: "Preview: CPU usage, %",
        min: 0.0,
        max: 100.0,
        step: 1.0,
    },
    InputDefinition {
        name: "DemoGpuLoad",
        label: "Preview: GPU usage, %",
        min: 0.0,
        max: 100.0,
        step: 1.0,
    },
    InputDefinition {
        name: "DemoDiskRead",
        label: "Preview: DISK read, MB/s",
        min: 0.0,
        max: 10000.0,
        step: 0.01,
    },
    InputDefinition {
        name: "DemoDiskWrite",
        label: "Preview: DISK write, MB/s",
        min: 0.0,
        max: 10000.0,
        step: 0.01,
    },
    InputDefinition {
        name: "DemoDownload",
        label: "Preview: NET receive, MB/s",
        min: 0.0,
        max: 1000.0,
        step: 0.01,
    },
    InputDefinition {
        name: "DemoUpload",
        label: "Preview: NET send, MB/s",
        min: 0.0,
        max: 1000.0,
        step: 0.01,
    },
    InputDefinition {
        name: "CpuHotX",
        label: "CPU: number starts turning red at X, %",
        min: 50.0,
        max: 100.0,
        step: 1.0,
    },
    InputDefinition {
        name: "CpuHotY",
        label: "CPU: number is fully red at Y, %",
        min: 51.0,
        max: 101.0,
        step: 1.0,
    },
    InputDefinition {
        name: "GpuHotX",
        label: "GPU: number starts turning red at X, %",
        min: 50.0,
        max: 100.0,
        step: 1.0,
    },
    InputDefinition {
        name: "GpuHotY",
        label: "GPU: number is fully red at Y, %",
        min: 51.0,
        max: 101.0,
        step: 1.0,
    },
    InputDefinition {
        name: "RamHotX",
        label: "RAM: number starts turning red at X, %",
        min: 50.0,
        max: 100.0,
        step: 1.0,
    },
    InputDefinition {
        name: "RamHotY",
        label: "RAM: number is fully red at Y, %",
        min: 51.0,
        max: 101.0,
        step: 1.0,
    },
];

/// Indices of the preview values: CPU/GPU temperature, RAM, CPU/GPU load, DISK ↓/↑, NET ↓/↑.
const PREVIEW_VALUES: std::ops::Range<usize> = 22..31;
/// Thresholds of tile numbers, X and Y for CPU, GPU and RAM.
const NUMBER_THRESHOLDS: std::ops::RangeFrom<usize> = 31..;
/// On/off tile settings: check box name and label, in the order of `TileStyle`.
const SWITCHES: [(&str, &str); 3] = [
    ("CpuTemperature", "CPU temperature on the tile"),
    ("GpuTemperature", "GPU temperature on the tile"),
    ("DashedTemperature", "Dashed temperature line"),
];
/// Test values of the settings artboards: GPU between its thresholds, RAM above them.
const ARTBOARD_VALUES: [f64; 9] = [56.0, 76.0, 93.0, 34.0, 89.0, 23.4, 4.12, 217.91, 53.94];

/// Appearance drafts are isolated; the monitoring switch changes live collection.
pub(super) struct Playground {
    monitoring: super::monitoring::MonitoringSetting,
    colors: super::colors::ColorEditor,
    controls: Vec<Com>,
    /// "Dashed temperature line".
    /// Check boxes of `SWITCHES`.
    switches: Vec<Com>,
    /// Text fields of the eight preview values, locked while live values are shown.
    preview_fields: Vec<Com>,
    hosts: [Com; 2],
    previews: Vec<WidgetPreview>,
    last: Option<TileStyle>,
    layout_pending: bool,
}
impl Playground {
    pub fn apply_to_taskbar(&mut self) -> std::io::Result<()> {
        let mut style = self
            .read()
            .map_err(|hr| std::io::Error::other(format!("{hr:08X}")))?;
        [style.light, style.dark] = self
            .colors
            .read()
            .map_err(|hr| std::io::Error::other(format!("{hr:08X}")))?;
        crate::platform::xaml::appearance::Appearance::save(
            &self.monitoring.appearance_path(),
            &style,
        )
    }
    /// The standalone editor, in English.
    pub fn markup() -> String {
        Language::English.markup(&Self::markup_with(include_str!("settings.xaml")))
    }
    pub(super) fn markup_with(template: &str) -> String {
        let controls = INPUTS.iter().map(|input| format!(r##"
          <Border BorderBrush="#20808080" BorderThickness="0,0,0,1" Padding="0,6,0,12">
            <Grid><Grid.ColumnDefinitions><ColumnDefinition Width="260"/><ColumnDefinition/><ColumnDefinition Width="80"/></Grid.ColumnDefinitions>
              <TextBlock Text="{}" FontSize="13" VerticalAlignment="Center"/>
              <Slider x:Name="{}" Grid.Column="1" Minimum="{}" Maximum="{}" StepFrequency="{}" Margin="12,0,18,0" VerticalAlignment="Center" AutomationProperties.Name="{}"/>
              <TextBox x:Name="{}Text" Grid.Column="2" Text="{{Binding Value, ElementName={}, Mode=TwoWay, UpdateSourceTrigger=PropertyChanged}}" VerticalAlignment="Center" AutomationProperties.Name="{}"/>
            </Grid>
          </Border>"##, Self::label(input), input.name, input.min, input.max, input.step, Self::label(input), input.name, input.name, Self::label(input))).collect::<Vec<_>>();
        template
            .replace(
                "@CONTROLS@",
                &(controls[..14].join("")
                    + &SWITCHES
                        .iter()
                        .map(|(name, label)| {
                            format!(
                                r#"<CheckBox x:Name="{name}" Margin="0,8" Content="@{label}@"/>"#
                            )
                        })
                        .collect::<String>()),
            )
            .replace(
                "@ALERTS@",
                &(controls[14..22].join("") + &controls[NUMBER_THRESHOLDS].join("")),
            )
            .replace("@DEMO@", &controls[PREVIEW_VALUES].join(""))
            .replace(
                "@COLORS@",
                &Self::neutral(&super::colors::ColorEditor::markup(false)),
            )
    }
    /// The design lab has no theme tokens: shared markup gets neutral colors.
    fn neutral(markup: &str) -> String {
        [
            ("$text2$", "#CC808080"),
            ("$text3$", "#99808080"),
            ("$divider$", "#20808080"),
            ("$ctrlBorder$", "#40808080"),
            (
                "$ctrl$",
                "{ThemeResource SystemControlBackgroundAltHighBrush}",
            ),
            (
                "$text$",
                "{ThemeResource SystemControlForegroundBaseHighBrush}",
            ),
            (
                "$card$",
                "{ThemeResource SystemControlBackgroundAltHighBrush}",
            ),
            (
                "$border$",
                "{ThemeResource SystemControlForegroundBaseLowBrush}",
            ),
            (
                "$accent$",
                "{ThemeResource SystemControlBackgroundAccentBrush}",
            ),
            (
                "$onAccent$",
                "{ThemeResource SystemControlForegroundChromeWhiteBrush}",
            ),
        ]
        .iter()
        .fold(markup.to_owned(), |markup, (token, color)| {
            markup.replace(token, color)
        })
    }
    /// Restores the default palettes of both themes.
    pub fn reset_colors(&mut self) -> Result<()> {
        self.colors.load([
            crate::platform::xaml::Palette::light(),
            crate::platform::xaml::Palette::dark(),
        ])
    }
    /// "CPU / GPU width, px" → ("CPU / GPU width", "px").
    fn split(label: &str) -> (&str, &str) {
        label.rsplit_once(", ").unwrap_or((label, ""))
    }
    /// The label with `@name@` for `Language::markup` and the unit, translated if it reads
    /// differently in another language.
    fn label(input: &InputDefinition) -> String {
        match Self::split(input.label) {
            (name, "") => format!("@{name}@"),
            (name, unit) => format!("@{name}@, {}", Language::marked(unit)),
        }
    }
    /// Settings-card rows of the main window: label, 220 px slider, 64 px field and unit.
    pub(super) fn rows(indices: &[usize]) -> String {
        let rows: String = indices.iter().enumerate().map(|(position, index)| {
            let input = &INPUTS[*index];
            let (label, unit) = Self::split(input.label);
            let (label, unit) = (format!("@{label}@"), Language::marked(unit));
            format!(r#"<Grid MinHeight="48" Padding="48,0,16,0" ColumnSpacing="16" BorderBrush="$divider$" BorderThickness="0,{},0,0"><Grid.ColumnDefinitions><ColumnDefinition/><ColumnDefinition Width="220"/><ColumnDefinition Width="Auto"/></Grid.ColumnDefinitions><TextBlock Text="{label}" VerticalAlignment="Center" TextWrapping="Wrap"/><Slider x:Name="{name}" Grid.Column="1" Minimum="{}" Maximum="{}" StepFrequency="{}" VerticalAlignment="Center" AutomationProperties.Name="{label}"/><StackPanel Grid.Column="2" Orientation="Horizontal" Spacing="8"><TextBox x:Name="{name}Text" Width="64" TextAlignment="Right" Text="{{Binding Value, ElementName={name}, Mode=TwoWay, UpdateSourceTrigger=PropertyChanged}}" AutomationProperties.Name="{label}"/><TextBlock Text="{unit}" Width="24" FontSize="12" Foreground="$text3$" VerticalAlignment="Center"/></StackPanel></Grid>"#,
                u8::from(position > 0), input.min, input.max, input.step, name = input.name)
        }).collect();
        format!("<StackPanel>{rows}</StackPanel>")
    }
    /// Settings-card rows of the temperature switches, below slider rows.
    pub(super) fn switch_rows() -> String {
        SWITCHES
            .iter()
            .map(|(name, label)| format!(r#"<Grid MinHeight="48" Padding="48,0,16,0" BorderBrush="$divider$" BorderThickness="0,1,0,0"><CheckBox x:Name="{name}" VerticalAlignment="Center" AutomationProperties.Name="@{label}@"><TextBlock Text="@{label}@"/></CheckBox></Grid>"#))
            .collect()
    }
    /// Preview values: a column per tile in the tiles' order, its name on top and
    /// below it the load and temperature, or the two directions of a rate.
    pub(super) fn preview_values() -> String {
        const COLUMNS: [(&str, &[(usize, &str)]); 5] = [
            ("CPU", &[(25, ""), (22, "")]),
            ("GPU", &[(26, ""), (23, "")]),
            ("RAM", &[(24, "")]),
            ("NET", &[(29, "↓"), (30, "↑")]),
            ("DISK", &[(27, "↓"), (28, "↑")]),
        ];
        let mut markup = String::from(
            r#"<Grid ColumnSpacing="16" RowSpacing="6"><Grid.ColumnDefinitions><ColumnDefinition/><ColumnDefinition/><ColumnDefinition/><ColumnDefinition/><ColumnDefinition/></Grid.ColumnDefinitions><Grid.RowDefinitions><RowDefinition/><RowDefinition/><RowDefinition/></Grid.RowDefinitions>"#,
        );
        for (column, (title, cells)) in COLUMNS.iter().enumerate() {
            markup.push_str(&format!(
                r#"<TextBlock Grid.Column="{column}" Text="{title}" FontSize="12" Foreground="$text2$"/>"#
            ));
            for (row, (index, direction)) in cells.iter().enumerate() {
                let input = &INPUTS[*index];
                let unit = Language::marked(Self::split(input.label).1);
                let direction = match *direction {
                    "" => String::new(),
                    arrow => format!(
                        r#"<TextBlock Text="{arrow}" Width="10" FontSize="12" Foreground="$text2$" VerticalAlignment="Center"/>"#
                    ),
                };
                markup.push_str(&format!(r#"<StackPanel Grid.Row="{}" Grid.Column="{column}" Orientation="Horizontal" Spacing="6">{direction}<TextBox x:Name="{name}Text" Width="70" TextAlignment="Right" Text="{{Binding Value, ElementName={name}, Mode=TwoWay, UpdateSourceTrigger=PropertyChanged}}" AutomationProperties.Name="{}"/><TextBlock Text="{unit}" Width="30" FontSize="11" Foreground="$text3$" VerticalAlignment="Center"/><Slider x:Name="{name}" Visibility="Collapsed" Minimum="{}" Maximum="{}" StepFrequency="{}"/></StackPanel>"#,
                    row + 1, Self::label(input), input.min, input.max, input.step, name = input.name));
            }
        }
        markup.push_str("</Grid>");
        markup
    }
    /// Sets the preview values that are known, in `PREVIEW_VALUES` order.
    pub fn preview(&self, values: &[Option<f64>; 9]) -> Result<()> {
        for (control, value) in self.controls[PREVIEW_VALUES].iter().zip(values) {
            if let Some(value) = value.filter(|v| v.is_finite()) {
                Self::set(control, value)?;
            }
        }
        Ok(())
    }
    /// Lower and upper thresholds of the rule whose sliders are `{name}X` and `{name}Y`.
    pub fn set_alert(&self, name: &str, x: f64, y: f64) -> Result<()> {
        for (suffix, value) in [("X", x), ("Y", y)] {
            let slider = format!("{name}{suffix}");
            let index = INPUTS
                .iter()
                .position(|input| input.name == slider)
                .ok_or(E_FAIL)?;
            Self::set(&self.controls[index], value)?;
        }
        Ok(())
    }
    /// Live values are read-only in the fields.
    pub fn lock_preview(&self, locked: bool) -> Result<()> {
        for field in &self.preview_fields {
            let control = field.query(&CONTROL)?;
            unsafe {
                // Control.IsEnabled
                let set: unsafe extern "system" fn(Raw, u8) -> Hr = control.slot(23);
                check(set(control.raw(), u8::from(!locked)))?;
            }
        }
        Ok(())
    }
    /// The last style shown in the previews, colors included.
    pub fn style(&self) -> Option<TileStyle> {
        self.last
    }
    /// Loads saved appearance into the controls; preview values stay as they are.
    pub fn load(&mut self, style: &TileStyle) -> Result<()> {
        let alerts = style.alerts;
        for (control, value) in self.controls.iter().zip([
            style.width,
            style.gap,
            style.radius,
            style.label_size,
            style.value_size,
            style.secondary_size,
            style.stroke,
            style.opacity,
            style.offset_x,
            style.offset_y,
            style.graph_left,
            style.fade_offset,
            style.fade_width,
            style.fade_opacity,
            alerts.cpu_x,
            alerts.cpu_y,
            alerts.gpu_x,
            alerts.gpu_y,
            alerts.ram_x,
            alerts.ram_y,
            alerts.alert_opacity,
            alerts.pulse_seconds,
        ]) {
            Self::set(control, value)?;
        }
        for (control, value) in self.controls[NUMBER_THRESHOLDS].iter().zip([
            alerts.cpu_hot_x,
            alerts.cpu_hot_y,
            alerts.gpu_hot_x,
            alerts.gpu_hot_y,
            alerts.ram_hot_x,
            alerts.ram_hot_y,
        ]) {
            Self::set(control, value)?;
        }
        for (check, on) in self.switches.iter().zip([
            style.cpu_temperature,
            style.gpu_temperature,
            style.dashed_temperature,
        ]) {
            super::monitor::ui::Ui::checked(check, on)?;
        }
        self.colors.load([style.light, style.dark])
    }
    pub fn new(root: &Com) -> Result<Self> {
        let controls = INPUTS
            .iter()
            .map(|input| Self::find(root, input.name)?.query(&RANGE))
            .collect::<Result<Vec<_>>>()?;
        let hosts = [
            Self::find(root, "LightPreview")?.query(&PANEL)?.object(6)?,
            Self::find(root, "DarkPreview")?.query(&PANEL)?.object(6)?,
        ];
        let preview_fields = INPUTS[PREVIEW_VALUES]
            .iter()
            .map(|input| Self::find(root, &format!("{}Text", input.name)))
            .collect::<Result<Vec<_>>>()?;
        let mut result = Self {
            monitoring: super::monitoring::MonitoringSetting::new(root)?,
            colors: super::colors::ColorEditor::new(root)?,
            controls,
            switches: SWITCHES
                .iter()
                .map(|(name, _)| Self::find(root, name))
                .collect::<Result<_>>()?,
            preview_fields,
            hosts,
            previews: Vec::new(),
            last: None,
            layout_pending: false,
        };
        let defaults = TileStyle::default();
        for (control, value) in result.controls.iter().zip([
            defaults.width,
            defaults.gap,
            defaults.radius,
            defaults.label_size,
            defaults.value_size,
            defaults.secondary_size,
            defaults.stroke,
            defaults.opacity,
            defaults.offset_x,
            defaults.offset_y,
            defaults.graph_left,
            defaults.fade_offset,
            defaults.fade_width,
            defaults.fade_opacity,
            defaults.alerts.cpu_x,
            defaults.alerts.cpu_y,
            defaults.alerts.gpu_x,
            defaults.alerts.gpu_y,
            defaults.alerts.ram_x,
            defaults.alerts.ram_y,
            defaults.alerts.alert_opacity,
            defaults.alerts.pulse_seconds,
            defaults.demo_cpu,
            defaults.demo_gpu,
            defaults.demo_ram,
            defaults.demo_cpu_load,
            defaults.demo_gpu_load,
            defaults.demo_disk_read,
            defaults.demo_disk_write,
            defaults.demo_download,
            defaults.demo_upload,
            defaults.alerts.cpu_hot_x,
            defaults.alerts.cpu_hot_y,
            defaults.alerts.gpu_hot_x,
            defaults.alerts.gpu_hot_y,
            defaults.alerts.ram_hot_x,
            defaults.alerts.ram_hot_y,
        ]) {
            Self::set(control, value)?;
        }
        for (check, on) in result
            .switches
            .iter()
            .zip([defaults.cpu_temperature, defaults.gpu_temperature])
        {
            super::monitor::ui::Ui::checked(check, on)?;
        }
        result.preview(&ARTBOARD_VALUES.map(Some))?;
        result.restore_draft()?;
        result.refresh()?;
        Ok(result)
    }
    fn restore_draft(&self) -> Result<()> {
        if std::env::args().any(|arg| {
            matches!(
                arg.as_str(),
                "--verify-settings" | "--verify-monitor" | "--demo"
            )
        }) {
            return Ok(());
        }
        let Some(text) =
            crate::platform::data_directory::DataDirectory::file("taskbar-metrics.editor")
                .ok()
                .and_then(|path| std::fs::read_to_string(path).ok())
        else {
            return Ok(());
        };
        let keys = [
            "width",
            "gap",
            "radius",
            "label_size",
            "value_size",
            "secondary_size",
            "stroke",
            "opacity",
            "offset_x",
            "offset_y",
            "graph_left",
            "fade_offset",
            "fade_width",
            "fade_opacity",
            "cpu_x",
            "cpu_y",
            "gpu_x",
            "gpu_y",
            "ram_x",
            "ram_y",
            "alert_opacity",
            "pulse_seconds",
            "demo_cpu",
            "demo_gpu",
            "demo_ram",
            "demo_cpu_load",
            "demo_gpu_load",
            "demo_disk_read",
            "demo_disk_write",
            "demo_download",
            "demo_upload",
            "cpu_hot_x",
            "cpu_hot_y",
            "gpu_hot_x",
            "gpu_hot_y",
            "ram_hot_x",
            "ram_hot_y",
        ];
        for line in text.lines() {
            let Some((key, raw)) = line.trim().split_once(':') else {
                continue;
            };
            // The temperature switches come back; the dashed line starts unchecked.
            let switch = ["cpu_temperature", "gpu_temperature"]
                .iter()
                .position(|candidate| *candidate == key);
            if let Some(switch) = switch {
                if let Ok(on) = raw.trim().trim_end_matches(',').parse::<bool>() {
                    super::monitor::ui::Ui::checked(&self.switches[switch], on)?;
                }
                continue;
            }
            let Some(index) = keys.iter().position(|candidate| *candidate == key) else {
                continue;
            };
            let Ok(value) = raw.trim().trim_end_matches(',').parse::<f64>() else {
                continue;
            };
            let input = &INPUTS[index];
            if value.is_finite() && (input.min..=input.max).contains(&value) {
                Self::set(&self.controls[index], value)?;
            }
        }
        Ok(())
    }
    pub(super) fn find(root: &Com, name: &str) -> Result<Com> {
        let frame = root.query(&FRAMEWORK)?;
        let name = HString::new(name)?;
        unsafe {
            let find: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = frame.slot(51);
            let mut object = ptr::null_mut();
            check(find(frame.raw(), name.0, &mut object))?;
            Com::owned(object)
        }
    }
    fn set(control: &Com, value: f64) -> Result<()> {
        unsafe {
            let set: unsafe extern "system" fn(Raw, f64) -> Hr = control.slot(15);
            check(set(control.raw(), value))
        }
    }
    fn read(&self) -> Result<TileStyle> {
        let mut values = [0.0; INPUTS.len()];
        for ((control, value), input) in self.controls.iter().zip(&mut values).zip(&INPUTS) {
            unsafe {
                let get: unsafe extern "system" fn(Raw, *mut f64) -> Hr = control.slot(14);
                check(get(control.raw(), value))?;
            }
            if !value.is_finite() {
                return Err(E_FAIL);
            }
            *value = value.clamp(input.min, input.max);
        }
        for (x, y) in [(14, 15), (16, 17), (18, 19), (31, 32), (33, 34), (35, 36)] {
            if values[y] <= values[x] {
                values[y] = values[x] + 1.0;
                Self::set(&self.controls[y], values[y])?;
            }
        }
        let [width, gap, radius, label_size, value_size, secondary_size, stroke, opacity, offset_x, offset_y, graph_left, fade_offset, fade_width, fade_opacity, cpu_x, cpu_y, gpu_x, gpu_y, ram_x, ram_y, alert_opacity, pulse_seconds, demo_cpu, demo_gpu, demo_ram, demo_cpu_load, demo_gpu_load, demo_disk_read, demo_disk_write, demo_download, demo_upload, cpu_hot_x, cpu_hot_y, gpu_hot_x, gpu_hot_y, ram_hot_x, ram_hot_y] =
            values;
        Ok(TileStyle {
            alerts: crate::platform::xaml::AlertSettings {
                cpu_x,
                cpu_y,
                gpu_x,
                gpu_y,
                ram_x,
                ram_y,
                alert_opacity,
                pulse_seconds,
                cpu_hot_x,
                cpu_hot_y,
                gpu_hot_x,
                gpu_hot_y,
                ram_hot_x,
                ram_hot_y,
            },
            demo_cpu,
            demo_gpu,
            demo_ram,
            demo_cpu_load,
            demo_gpu_load,
            demo_disk_read,
            demo_disk_write,
            demo_download,
            demo_upload,
            width,
            gap,
            radius,
            label_size,
            value_size,
            secondary_size,
            stroke,
            opacity,
            offset_x,
            offset_y,
            graph_left,
            fade_offset,
            fade_width,
            fade_opacity,
            cpu_temperature: super::monitor::ui::Ui::is_checked(&self.switches[0])?,
            gpu_temperature: super::monitor::ui::Ui::is_checked(&self.switches[1])?,
            dashed_temperature: super::monitor::ui::Ui::is_checked(&self.switches[2])?,
            light: crate::platform::xaml::Palette::light(),
            dark: crate::platform::xaml::Palette::dark(),
        })
    }
    pub fn refresh(&mut self) -> Result<()> {
        self.monitoring.refresh()?;
        let mut style = self.read()?;
        [style.light, style.dark] = self.colors.read()?;
        if self.last != Some(style) {
            let previews = [false, true]
                .into_iter()
                .map(|dark| WidgetPreview::new(&style, dark))
                .collect::<Result<Vec<_>>>()?;
            for (host, preview) in self.hosts.iter().zip(&previews) {
                let element = preview.element()?;
                unsafe {
                    let clear: unsafe extern "system" fn(Raw) -> Hr = host.slot(15);
                    let append: unsafe extern "system" fn(Raw, Raw) -> Hr = host.slot(13);
                    check(clear(host.raw()))?;
                    check(append(host.raw(), element.raw()))?;
                }
            }
            self.previews = previews;
            self.last = Some(style);
            self.colors.save();
            if !std::env::args().any(|arg| {
                matches!(
                    arg.as_str(),
                    "--verify-settings" | "--verify-monitor" | "--demo"
                )
            }) {
                if let Ok(path) =
                    crate::platform::data_directory::DataDirectory::file("taskbar-metrics.editor")
                {
                    let _ = std::fs::write(path, format!("{style:#?}\n"));
                }
            }
            self.layout_pending = true;
        } else if self.layout_pending {
            // The next UI tick has measured text widths. Update fades once;
            // unchanged controls do not recreate tiles or write visual properties.
            for preview in &mut self.previews {
                preview.refresh_layout()?;
            }
            self.layout_pending = false;
        }
        Ok(())
    }
    pub fn verify(&mut self, root: &Com) -> Result<()> {
        self.colors.verify()?;
        for maximum in [false, true] {
            for (control, input) in self.controls.iter().zip(&INPUTS) {
                Self::set(control, if maximum { input.max } else { input.min })?;
            }
            NativeWindow::drain();
            self.refresh()?;
        }
        Self::set(&self.controls[21], 0.5)?;
        self.refresh()?;
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for _ in 0..16 {
            NativeWindow::drain();
            let opacity = self.previews[0].alert_opacity()?;
            low = low.min(opacity);
            high = high.max(opacity);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if high - low < 0.1 {
            return Err(E_FAIL);
        }
        Self::set(&self.controls[22], 0.0)?;
        self.refresh()?;
        NativeWindow::drain();
        if self.previews[0].alert_opacity()? != 0.0 {
            return Err(E_FAIL);
        }
        let text = Self::find(root, "TileWidthText")?
            .query(&Guid::from_u128(0xe48f5a8b_1dff_4352_a1f4_e516514ec882))?;
        text.set_string(7, "120")?;
        NativeWindow::drain();
        self.refresh()?;
        if self.read()?.width != 120.0 {
            return Err(E_FAIL);
        }
        text.set_string(7, "invalid")?;
        NativeWindow::drain();
        self.refresh()?;
        if self.read()?.width != 120.0 {
            return Err(E_FAIL);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn design_lab_markup_has_no_unexpanded_tokens() {
        let markup = Playground::markup();
        let token = markup
            .find('$')
            .map(|i| &markup[i..(i + 40).min(markup.len())]);
        assert_eq!(token, None);
        // Texts are English; only the theme colors are left for `SystemTheme`.
        let texts = markup
            .replace("@BACKGROUND@", "")
            .replace("@FOREGROUND@", "");
        assert!(!texts.contains('@'));
    }
}
