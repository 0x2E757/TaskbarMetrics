use super::color_picker::ColorPicker;
use super::playground::Playground;
use super::*;
use crate::platform::xaml::Palette;

const TEXTBOX: Guid = Guid::from_u128(0xe48f5a8b_1dff_4352_a1f4_e516514ec882);
const TEXT: Guid = Guid::from_u128(0xae2d9271_3b4a_45fc_8468_f7949548f4d5);
const BRUSH: Guid = Guid::from_u128(0x9d850850_66f3_48df_9a8f_824bd5e070af);
const ROLES: [(&str, &str); 14] = [
    ("tile", "Tile background"),
    ("header", "Header"),
    ("value", "Main value"),
    ("hot", "High usage value"),
    ("temperature_text", "Temperature value"),
    ("download_text", "Download arrow"),
    ("upload_text", "Upload arrow and value"),
    ("primary_line", "Primary chart line"),
    ("primary_area", "Chart area"),
    ("temperature_line", "Temperature line"),
    ("upload_line", "Upload line"),
    ("hover", "Hover background"),
    ("pressed", "Pressed background"),
    ("background", "Background behind widgets"),
];

pub(super) struct ColorEditor {
    fields: Vec<Com>,
    pickers: Vec<ColorSelection>,
    picker: ColorPicker,
    status: Com,
    palettes: [Palette; 2],
    invalid: bool,
}
impl ColorEditor {
    /// Element / light theme / dark theme table, grouped like the settings artboards.
    /// Colors are `$token$` placeholders and texts `@text@` ones for `Language::markup`;
    /// `reset` adds the "Reset colors" button.
    pub fn markup(reset: bool) -> String {
        const COLUMNS: &str = r#"<Grid.ColumnDefinitions><ColumnDefinition/><ColumnDefinition Width="190"/><ColumnDefinition Width="190"/></Grid.ColumnDefinitions>"#;
        let mut markup = format!(
            r#"<StackPanel>{}<Grid Height="36" Padding="48,0,16,0" BorderBrush="$divider$" BorderThickness="0,0,0,1">{COLUMNS}<TextBlock Text="@Element@" FontSize="12" Foreground="$text3$" VerticalAlignment="Center"/><TextBlock Text="@Light theme@" Grid.Column="1" FontSize="12" Foreground="$text3$" VerticalAlignment="Center"/><TextBlock Text="@Dark theme@" Grid.Column="2" FontSize="12" Foreground="$text3$" VerticalAlignment="Center"/></Grid>"#,
            ColorPicker::markup()
        );
        for (role, (_, label)) in ROLES.iter().enumerate() {
            if let Some(group) = match role {
                0 => Some("Tile"),
                5 => Some("Charts and directions"),
                11 => Some("States and background"),
                _ => None,
            } {
                markup.push_str(&format!(
                    r#"<TextBlock Text="@{group}@" FontSize="12" FontWeight="SemiBold" Foreground="$text2$" Margin="48,10,16,4"/>"#
                ));
            }
            markup.push_str(&format!(r#"<Grid MinHeight="40" Padding="48,0,16,0">{COLUMNS}<TextBlock Text="@{label}@" VerticalAlignment="Center" FontSize="13" TextWrapping="Wrap"/>"#));
            for (theme, column) in [("Light", 1), ("Dark", 2)] {
                markup.push_str(&ColorSelection::markup(
                    &format!("{theme}Color{role}"),
                    label,
                    theme,
                    column,
                ));
            }
            markup.push_str("</Grid>");
        }
        markup.push_str(&format!(
            r#"<Grid Padding="48,10,16,12" ColumnSpacing="8" BorderBrush="$divider$" BorderThickness="0,1,0,0"><Grid.ColumnDefinitions><ColumnDefinition/><ColumnDefinition Width="Auto"/></Grid.ColumnDefinitions><TextBlock x:Name="ColorStatus" Text="@{}@" FontSize="12" Foreground="$text2$" TextWrapping="Wrap" VerticalAlignment="Center"/>{}</Grid></StackPanel>"#,
            Self::HINT,
            if reset {
                r#"<Button x:Name="ResetColors" Grid.Column="1" Background="Transparent" BorderBrush="Transparent"><TextBlock Text="@Reset colors@" Margin="0,-2,0,0"/></Button>"#
            } else {
                ""
            }
        ));
        markup
    }
    const HINT: &'static str = "Format #RRGGBB or #AARRGGBB, where AA is opacity.";
    /// Replaces both palettes, e.g. when unapplied changes are reverted.
    pub fn load(&mut self, palettes: [Palette; 2]) -> Result<()> {
        self.palettes = palettes;
        for (index, field) in self.fields.iter().enumerate() {
            let color = palettes[index / ROLES.len()].0[index % ROLES.len()];
            field.set_string(7, &palettes[index / ROLES.len()].hex(index % ROLES.len()))?;
            self.pickers[index].set(color)?;
            self.pickers[index].last_text = field.string(6)?;
        }
        Ok(())
    }
    pub fn new(root: &Com) -> Result<Self> {
        let mut fields = Vec::new();
        let mut pickers = Vec::new();
        let mut targets = Vec::new();
        let language = monitor::settings_language();
        for (theme, title) in [("Light", "light theme"), ("Dark", "dark theme")] {
            for (role, (_, label)) in ROLES.iter().enumerate() {
                let name = format!("{theme}Color{role}");
                fields.push(Playground::find(root, &name)?.query(&TEXTBOX)?);
                pickers.push(ColorSelection::new(root, &name)?);
                targets.push((
                    format!("{name}Button"),
                    format!("{} · {}", language.text(label), language.text(title)),
                ));
            }
        }
        let mut editor = Self {
            fields,
            pickers,
            picker: ColorPicker::new(root, targets)?,
            status: Playground::find(root, "ColorStatus")?.query(&TEXT)?,
            palettes: [Palette::light(), Palette::dark()],
            invalid: false,
        };
        if !Self::verifying() {
            if let Some(text) = Self::path().and_then(|path| std::fs::read_to_string(path).ok()) {
                for line in text.lines() {
                    let Some((key, value)) = line.split_once('=') else {
                        continue;
                    };
                    let Some(color) = Palette::parse(value) else {
                        continue;
                    };
                    for (theme, prefix) in ["light", "dark"].iter().enumerate() {
                        for (role, (name, _)) in ROLES.iter().enumerate() {
                            if key == format!("{prefix}.{name}") {
                                editor.palettes[theme].0[role] = color;
                            }
                        }
                    }
                }
            }
        }
        for (index, field) in editor.fields.iter().enumerate() {
            field.set_string(
                7,
                &editor.palettes[index / ROLES.len()].hex(index % ROLES.len()),
            )?;
            editor.pickers[index]
                .set(editor.palettes[index / ROLES.len()].0[index % ROLES.len()])?;
            editor.pickers[index].last_text = field.string(6)?;
        }
        Ok(editor)
    }
    pub fn read(&mut self) -> Result<[Palette; 2]> {
        let current: Vec<u32> = self.pickers.iter().map(|p| p.color).collect();
        if let Some((index, color)) = self.picker.refresh(|i| current[i])? {
            self.fields[index].set_string(7, &format!("#{color:08X}"))?;
        }
        let mut invalid = false;
        for (index, field) in self.fields.iter().enumerate() {
            if let Some(color) = self.pickers[index].read(field)? {
                self.palettes[index / ROLES.len()].0[index % ROLES.len()] = color;
            } else {
                invalid = true;
            }
        }
        if self.invalid != invalid {
            self.status.set_string(
                27,
                monitor::settings_language().text(if invalid {
                    "Invalid or incomplete HEX: the last valid color is used for this field."
                } else {
                    Self::HINT
                }),
            )?;
            self.invalid = invalid;
        }
        Ok(self.palettes)
    }
    /// Verification and demo runs neither read nor write the color draft.
    fn verifying() -> bool {
        std::env::args().any(|arg| {
            matches!(
                arg.as_str(),
                "--verify-settings" | "--verify-monitor" | "--demo"
            )
        })
    }
    fn path() -> Option<std::path::PathBuf> {
        crate::platform::data_directory::DataDirectory::file("taskbar-metrics.colors").ok()
    }
    pub fn save(&self) {
        if Self::verifying() {
            return;
        }
        let mut text = String::new();
        for (theme, palette) in ["light", "dark"].iter().zip(self.palettes) {
            for (role, (key, _)) in ROLES.iter().enumerate() {
                text.push_str(&format!("{theme}.{key}={}\n", palette.hex(role)));
            }
        }
        if let Some(path) = Self::path() {
            let _ = std::fs::write(path, text);
        }
    }
    pub fn verify(&mut self) -> Result<()> {
        for index in [0, ROLES.len()] {
            let (target, color) = self.picker.verify(index, 0x40123456)?.ok_or(E_FAIL)?;
            self.fields[target].set_string(7, &format!("#{color:08X}"))?;
            if target != index
                || self.read()?[index / ROLES.len()].0[index % ROLES.len()] != 0x40123456
            {
                return Err(E_FAIL);
            }
        }
        for field in &self.fields {
            field.set_string(7, "#80AABBCC")?;
        }
        if self
            .read()?
            .iter()
            .any(|palette| palette.0 != [0x80aabbcc; ROLES.len()])
        {
            return Err(E_FAIL);
        }
        self.fields[0].set_string(7, "#invalid")?;
        if self.read()?[0].0[0] != 0x80aabbcc {
            return Err(E_FAIL);
        }
        self.fields[0].set_string(7, "#40123456")?;
        if self.read()?[0].0[0] != 0x40123456 {
            return Err(E_FAIL);
        }
        Ok(())
    }
}

/// Keeps a swatch in step with its editable HEX field.
struct ColorSelection {
    brush: Com,
    color: u32,
    last_text: String,
}
impl ColorSelection {
    fn markup(name: &str, label: &str, theme: &str, column: u32) -> String {
        format!(
            r##"<StackPanel Grid.Column="{column}" Orientation="Horizontal" Spacing="8" VerticalAlignment="Center">
          <Button x:Name="{name}Button" Width="26" Height="26" MinWidth="0" MinHeight="0" Padding="0" BorderThickness="0" CornerRadius="4" Background="Transparent" AutomationProperties.Name="@Choose color@: @{label}@">
            <Grid Width="26" Height="26" Background="#FFFFFFFF"><Grid.ColumnDefinitions><ColumnDefinition/><ColumnDefinition/><ColumnDefinition/></Grid.ColumnDefinitions><Grid.RowDefinitions><RowDefinition/><RowDefinition/><RowDefinition/></Grid.RowDefinitions><Rectangle Fill="#FFCCCCCC"/><Rectangle Grid.Column="2" Fill="#FFCCCCCC"/><Rectangle Grid.Row="1" Grid.Column="1" Fill="#FFCCCCCC"/><Rectangle Grid.Row="2" Fill="#FFCCCCCC"/><Rectangle Grid.Row="2" Grid.Column="2" Fill="#FFCCCCCC"/><Border Grid.RowSpan="3" Grid.ColumnSpan="3" CornerRadius="4" BorderBrush="$ctrlBorder$" BorderThickness="1"><Border.Background><SolidColorBrush x:Name="{name}Swatch" Color="Transparent"/></Border.Background></Border></Grid>
          </Button>
          <TextBox x:Name="{name}" Width="120" MinHeight="30" FontSize="12" Padding="8,6,8,0" MaxLength="9" TextWrapping="Wrap" AutomationProperties.Name="@{label}@, @{theme}@"/>
        </StackPanel>"##
        )
    }
    fn new(root: &Com, name: &str) -> Result<Self> {
        Ok(Self {
            brush: Playground::find(root, &format!("{name}Swatch"))?.query(&BRUSH)?,
            color: 0,
            last_text: String::new(),
        })
    }
    fn set(&mut self, color: u32) -> Result<()> {
        unsafe {
            let set: unsafe extern "system" fn(Raw, [u8; 4]) -> Hr = self.brush.slot(7);
            check(set(self.brush.raw(), color.to_be_bytes()))?;
        }
        self.color = color;
        Ok(())
    }
    fn read(&mut self, field: &Com) -> Result<Option<u32>> {
        let text = field.string(6)?;
        if text != self.last_text {
            self.last_text = text;
            if let Some(color) = Palette::parse(&self.last_text) {
                self.set(color)?;
            }
        }
        Ok(Palette::parse(&self.last_text))
    }
}
