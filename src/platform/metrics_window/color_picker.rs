use super::playground::Playground;
use super::*;
use crate::platform::xaml::{events::Subscription, Palette, FRAMEWORK};
use std::{cell::RefCell, rc::Rc};

const BRUSH: Guid = Guid::from_u128(0x9d850850_66f3_48df_9a8f_824bd5e070af);
const STOP: Guid = Guid::from_u128(0x665f44fe_2e59_4c4a_ab53_076a100ccd81);
const TRANSLATE: Guid = Guid::from_u128(0xc975905c_3c36_4229_817b_178f64c0e113);
const FLYOUT: Guid = Guid::from_u128(0x723eea0b_d12e_430d_a9f0_9bb32bbf9913);
const TEXTBOX: Guid = Guid::from_u128(0xe48f5a8b_1dff_4352_a1f4_e516514ec882);
const TEXT: Guid = Guid::from_u128(0xae2d9271_3b4a_45fc_8468_f7949548f4d5);
/// Saturation/value area, hue and alpha bars (px).
const WIDTH: f64 = 268.0;
const AREA: f64 = 150.0;
const PRESETS: [u32; 8] = [
    0xFFFF4B4B, 0xFFFF967D, 0xFFFFC83D, 0xFF3296E1, 0xFF60CDFF, 0xFF45D0B5, 0xFFB39DFF, 0xFFFFFFFF,
];

fn hex(argb: u32) -> String {
    format!("#{argb:08X}")
}

/// Hue in degrees, saturation and value in 0..=1.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Hsv {
    pub h: f64,
    pub s: f64,
    pub v: f64,
}
impl Hsv {
    pub fn of(rgb: u32) -> Self {
        let [r, g, b] = [16, 8, 0].map(|shift| ((rgb >> shift) & 0xFF) as f64 / 255.0);
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        let delta = max - min;
        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / delta).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        Self {
            h,
            s: if max == 0.0 { 0.0 } else { delta / max },
            v: max,
        }
    }
    /// `0x00RRGGBB`.
    pub fn rgb(self) -> u32 {
        let c = self.v * self.s;
        let x = c * (1.0 - ((self.h / 60.0).rem_euclid(2.0) - 1.0).abs());
        let (r, g, b) = match (self.h.rem_euclid(360.0) / 60.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = self.v - c;
        [r, g, b]
            .map(|channel| ((channel + m) * 255.0).round() as u32)
            .iter()
            .fold(0, |rgb, channel| (rgb << 8) | channel)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Area,
    Hue,
    Alpha,
}
enum PickerEvent {
    Open(usize),
    Drag(Part, f64, f64),
    Preset(u32),
    Done,
    Cancel,
}
/// Target field and the color being edited.
struct Editing {
    target: usize,
    hsv: Hsv,
    alpha: u8,
    hex: String,
    percent: String,
}
impl Editing {
    fn argb(&self) -> u32 {
        (u32::from(self.alpha) << 24) | self.hsv.rgb()
    }
}

/// One HSV picker flyout shared by every swatch of the colors table (settings artboard):
/// saturation/value area, hue and alpha bars, HEX and alpha fields, presets.
pub(super) struct ColorPicker {
    root: Com,
    flyout: Com,
    targets: Vec<(Com, String)>,
    events: Rc<RefCell<Vec<PickerEvent>>>,
    _subscriptions: Vec<Subscription>,
    editing: Option<Editing>,
}
impl ColorPicker {
    fn swatch(size: u32, brush: &str, color: &str) -> String {
        format!(
            r##"<Grid Width="{size}" Height="{size}"><Grid.Background><SolidColorBrush Color="#FFFFFFFF"/></Grid.Background><Path Data="{}" Fill="#FFC8C8C8"/><Border CornerRadius="4" BorderBrush="$ctrlBorder$" BorderThickness="1"><Border.Background><SolidColorBrush x:Name="{brush}" Color="{color}"/></Border.Background></Border></Grid>"##,
            Self::checker(size, size)
        )
    }
    /// 4 px checkerboard behind translucent colors.
    fn checker(width: u32, height: u32) -> String {
        let mut data = String::new();
        for row in 0..height.div_ceil(4) {
            for column in 0..width.div_ceil(4) {
                if (row + column) % 2 == 0 {
                    data.push_str(&format!("M{},{}h4v4h-4z", column * 4, row * 4));
                }
            }
        }
        data
    }
    fn thumb(name: &str) -> String {
        format!(
            r##"<Canvas IsHitTestVisible="False"><Grid Width="20" Height="20" Canvas.Top="-4"><Grid.RenderTransform><TranslateTransform x:Name="{name}Thumb"/></Grid.RenderTransform><Ellipse Width="22" Height="22" Margin="-1" Stroke="#59000000" StrokeThickness="1"/><Ellipse Stroke="#FFFFFFFF" StrokeThickness="2"><Ellipse.Fill><SolidColorBrush x:Name="{name}Fill"/></Ellipse.Fill></Ellipse></Grid></Canvas>"##
        )
    }
    /// Attached flyout for the colors table. Uses `$token$` colors.
    pub fn markup() -> String {
        let presets: String = PRESETS
            .iter()
            .enumerate()
            .map(|(index, color)| {
                format!(
                    r#"<Button x:Name="PickerPreset{index}" Width="22" Height="22" MinWidth="0" MinHeight="0" Padding="0" BorderThickness="0" Background="Transparent" AutomationProperties.Name="{}">{}</Button>"#,
                    hex(*color),
                    Self::swatch(22, &format!("PickerPresetFill{index}"), &hex(*color))
                )
            })
            .collect();
        format!(
            r##"<FlyoutBase.AttachedFlyout><Flyout x:Name="ColorFlyout" Placement="Bottom"><Flyout.FlyoutPresenterStyle><Style TargetType="FlyoutPresenter"><Setter Property="Background" Value="$card$"/><Setter Property="BorderBrush" Value="$border$"/><Setter Property="BorderThickness" Value="1"/><Setter Property="Padding" Value="16"/><Setter Property="CornerRadius" Value="8"/></Style></Flyout.FlyoutPresenterStyle>
<StackPanel Width="{WIDTH}" Spacing="12">
 <TextBlock x:Name="PickerTitle" FontSize="13" FontWeight="SemiBold"/>
 <Grid x:Name="PickerArea" Width="{WIDTH}" Height="{AREA}" Background="Transparent">
  <Border CornerRadius="6"><Border.Background><SolidColorBrush x:Name="PickerHue" Color="#FFFF0000"/></Border.Background></Border>
  <Border CornerRadius="6"><Border.Background><LinearGradientBrush StartPoint="0,0" EndPoint="1,0"><GradientStop Color="#FFFFFFFF" Offset="0"/><GradientStop Color="#00FFFFFF" Offset="1"/></LinearGradientBrush></Border.Background></Border>
  <Border CornerRadius="6"><Border.Background><LinearGradientBrush StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#00000000" Offset="0"/><GradientStop Color="#FF000000" Offset="1"/></LinearGradientBrush></Border.Background></Border>
  <Canvas IsHitTestVisible="False"><Grid Width="16" Height="16"><Grid.RenderTransform><TranslateTransform x:Name="PickerMarker"/></Grid.RenderTransform><Ellipse Width="18" Height="18" Margin="-1" Stroke="#66000000" StrokeThickness="1"/><Ellipse Stroke="#FFFFFFFF" StrokeThickness="2"/></Grid></Canvas>
 </Grid>
 <Grid x:Name="PickerHueBar" Height="12" Background="Transparent">
  <Border CornerRadius="6"><Border.Background><LinearGradientBrush StartPoint="0,0" EndPoint="1,0"><GradientStop Color="#FFFF0000" Offset="0"/><GradientStop Color="#FFFFFF00" Offset="0.1667"/><GradientStop Color="#FF00FF00" Offset="0.3333"/><GradientStop Color="#FF00FFFF" Offset="0.5"/><GradientStop Color="#FF0000FF" Offset="0.6667"/><GradientStop Color="#FFFF00FF" Offset="0.8333"/><GradientStop Color="#FFFF0000" Offset="1"/></LinearGradientBrush></Border.Background></Border>
  {hue}
 </Grid>
 <Grid x:Name="PickerAlphaBar" Height="12" Background="Transparent">
  <Border CornerRadius="6" Background="#FFFFFFFF"/>
  <Path Data="{checker}" Fill="#FFC8C8C8" Margin="4,0"/>
  <Border CornerRadius="6"><Border.Background><LinearGradientBrush StartPoint="0,0" EndPoint="1,0"><GradientStop x:Name="PickerAlpha0" Offset="0"/><GradientStop x:Name="PickerAlpha1" Offset="1"/></LinearGradientBrush></Border.Background></Border>
  {alpha}
 </Grid>
 <Grid ColumnSpacing="8" Margin="0,0,-28,0"><Grid.ColumnDefinitions><ColumnDefinition Width="Auto"/><ColumnDefinition Width="Auto"/><ColumnDefinition Width="Auto"/></Grid.ColumnDefinitions>
  <StackPanel Spacing="4" VerticalAlignment="Bottom"><TextBlock Text="HEX (AARRGGBB)" FontSize="11" Foreground="$text2$"/><TextBox x:Name="PickerHex" Width="130" {field} MaxLength="9" AutomationProperties.Name="HEX"/></StackPanel>
  <StackPanel Grid.Column="1" Spacing="4" VerticalAlignment="Bottom"><TextBlock Text="@Alpha@" FontSize="11" Foreground="$text2$"/><StackPanel Orientation="Horizontal" Spacing="6"><TextBox x:Name="PickerAlpha" Width="58" {field} MaxLength="3" TextAlignment="Right" AutomationProperties.Name="@Alpha@"/><TextBlock Text="%" Width="24" FontSize="12" Foreground="$text3$" VerticalAlignment="Center"/></StackPanel></StackPanel>
  <StackPanel Grid.Column="2" Spacing="4" VerticalAlignment="Bottom"><TextBlock Text="@Before / after@" Width="60" FontSize="11" Foreground="$text2$" TextWrapping="Wrap"/><StackPanel Orientation="Horizontal">{was}{now}</StackPanel></StackPanel>
 </Grid>
 <StackPanel Orientation="Horizontal" Spacing="6">{presets}</StackPanel>
 <Grid ColumnSpacing="8"><Grid.ColumnDefinitions><ColumnDefinition/><ColumnDefinition/></Grid.ColumnDefinitions>
  <Button x:Name="PickerDone" HorizontalAlignment="Stretch" {button} Background="$accent$" Foreground="$onAccent$" BorderBrush="$accent$" Content="@Done@"/>
  <Button x:Name="PickerCancel" Grid.Column="1" HorizontalAlignment="Stretch" {button} Background="$ctrl$" Foreground="$text$" BorderBrush="$ctrlBorder$" Content="@Cancel@"/>
 </Grid>
</StackPanel></Flyout></FlyoutBase.AttachedFlyout>"##,
            // Flyouts live outside the page tree, so the page's implicit styles do not reach them.
            // Wrapping single-line fields hides the clear button, as in the artboard.
            field = r#"Height="30" MinHeight="30" Padding="8,5,8,0" FontSize="13" BorderThickness="1" CornerRadius="4" TextWrapping="Wrap""#,
            button = r#"Height="32" CornerRadius="4" BorderThickness="1" FontSize="13""#,
            hue = Self::thumb("PickerHue"),
            alpha = Self::thumb("PickerAlpha"),
            checker = Self::checker(WIDTH as u32 - 8, 12),
            was = Self::swatch(30, "PickerWas", "#00000000"),
            now = Self::swatch(30, "PickerNow", "#00000000"),
        )
    }
    /// `targets`: swatch button names with the title shown for each ("Temperature line · dark theme").
    pub fn new(root: &Com, targets: Vec<(String, String)>) -> Result<Self> {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut subscriptions = Vec::new();
        let click = |name: &str, event: fn() -> PickerEvent| -> Result<Subscription> {
            let queue = events.clone();
            Subscription::click(&Playground::find(root, name)?, move || {
                queue.borrow_mut().push(event());
                Ok(())
            })
        };
        subscriptions.push(click("PickerDone", || PickerEvent::Done)?);
        subscriptions.push(click("PickerCancel", || PickerEvent::Cancel)?);
        for (index, color) in PRESETS.iter().enumerate() {
            let (queue, color) = (events.clone(), *color);
            subscriptions.push(Subscription::click(
                &Playground::find(root, &format!("PickerPreset{index}"))?,
                move || {
                    queue.borrow_mut().push(PickerEvent::Preset(color));
                    Ok(())
                },
            )?);
        }
        let mut buttons = Vec::new();
        for (index, (name, title)) in targets.into_iter().enumerate() {
            let button = Playground::find(root, &name)?;
            let queue = events.clone();
            subscriptions.push(Subscription::click(&button, move || {
                queue.borrow_mut().push(PickerEvent::Open(index));
                Ok(())
            })?);
            buttons.push((button, title));
        }
        for (name, part) in [
            ("PickerArea", Part::Area),
            ("PickerHueBar", Part::Hue),
            ("PickerAlphaBar", Part::Alpha),
        ] {
            let element = Playground::find(root, name)?;
            let dragging = Rc::new(std::cell::Cell::new(false));
            // Pressed (57) starts a captured drag, moves (59) follow it, released (61) ends it.
            for slot in [57, 59, 61] {
                let (queue, dragging) = (events.clone(), dragging.clone());
                subscriptions.push(Subscription::pointer(
                    &element,
                    slot,
                    move |sender, args| {
                        match slot {
                            57 => {
                                dragging.set(true);
                                Subscription::capture(sender, args)?;
                            }
                            61 => dragging.set(false),
                            _ if !dragging.get() => return Ok(()),
                            _ => {}
                        }
                        let (x, y) = Subscription::position(sender, args)?;
                        queue
                            .borrow_mut()
                            .push(PickerEvent::Drag(part, x as f64, y as f64));
                        Ok(())
                    },
                )?);
            }
        }
        Ok(Self {
            flyout: Playground::find(root, "ColorFlyout")?.query(&FLYOUT)?,
            root: root.clone(),
            targets: buttons,
            events,
            _subscriptions: subscriptions,
            editing: None,
        })
    }
    fn find(&self, name: &str) -> Result<Com> {
        Playground::find(&self.root, name)
    }
    fn color(object: &Com, argb: u32) -> Result<()> {
        unsafe {
            let set: unsafe extern "system" fn(Raw, [u8; 4]) -> Hr = object.slot(7);
            check(set(object.raw(), argb.to_be_bytes()))
        }
    }
    fn translate(&self, name: &str, x: f64, y: f64) -> Result<()> {
        let transform = self.find(name)?.query(&TRANSLATE)?;
        unsafe {
            let set: unsafe extern "system" fn(Raw, f64) -> Hr = transform.slot(7);
            check(set(transform.raw(), x))?;
            let set: unsafe extern "system" fn(Raw, f64) -> Hr = transform.slot(9);
            check(set(transform.raw(), y))
        }
    }
    fn text(&self, name: &str) -> Result<String> {
        self.find(name)?.query(&TEXTBOX)?.string(6)
    }
    /// Moves markers, recolors bars and rewrites the fields for the edited color.
    fn render(&mut self) -> Result<()> {
        let Some(editing) = &self.editing else {
            return Ok(());
        };
        let (hsv, alpha, argb) = (editing.hsv, editing.alpha, editing.argb());
        let hue = 0xFF00_0000
            | Hsv {
                s: 1.0,
                v: 1.0,
                ..hsv
            }
            .rgb();
        Self::color(&self.find("PickerHue")?.query(&BRUSH)?, hue)?;
        Self::color(&self.find("PickerHueFill")?.query(&BRUSH)?, hue)?;
        Self::color(&self.find("PickerAlphaFill")?.query(&BRUSH)?, argb)?;
        Self::color(&self.find("PickerNow")?.query(&BRUSH)?, argb)?;
        Self::color(&self.find("PickerAlpha0")?.query(&STOP)?, hsv.rgb())?;
        Self::color(
            &self.find("PickerAlpha1")?.query(&STOP)?,
            0xFF00_0000 | hsv.rgb(),
        )?;
        self.translate(
            "PickerMarker",
            hsv.s * WIDTH - 8.0,
            (1.0 - hsv.v) * AREA - 8.0,
        )?;
        self.translate("PickerHueThumb", hsv.h / 360.0 * (WIDTH - 20.0), 0.0)?;
        self.translate(
            "PickerAlphaThumb",
            f64::from(alpha) / 255.0 * (WIDTH - 20.0),
            0.0,
        )?;
        let text = hex(argb);
        let percent = (f64::from(alpha) / 2.55).round().to_string();
        let (hex_field, alpha_field) = (self.find("PickerHex")?, self.find("PickerAlpha")?);
        let editing = self.editing.as_mut().ok_or(E_FAIL)?;
        if editing.hex != text {
            hex_field.query(&TEXTBOX)?.set_string(7, &text)?;
            editing.hex = text;
        }
        if editing.percent != percent {
            alpha_field.query(&TEXTBOX)?.set_string(7, &percent)?;
            editing.percent = percent;
        }
        Ok(())
    }
    fn open(&mut self, target: usize, color: u32) -> Result<()> {
        let (button, title) = self.targets[target].clone();
        self.find("PickerTitle")?
            .query(&TEXT)?
            .set_string(27, &title)?;
        Self::color(&self.find("PickerWas")?.query(&BRUSH)?, color)?;
        self.editing = Some(Editing {
            target,
            hsv: Hsv::of(color),
            alpha: (color >> 24) as u8,
            hex: String::new(),
            percent: String::new(),
        });
        self.render()?;
        let button = button.query(&FRAMEWORK)?;
        unsafe {
            let show: unsafe extern "system" fn(Raw, Raw) -> Hr = self.flyout.slot(14);
            check(show(self.flyout.raw(), button.raw()))
        }
    }
    fn hide(&mut self) -> Result<()> {
        self.editing = None;
        unsafe {
            let hide: unsafe extern "system" fn(Raw) -> Hr = self.flyout.slot(15);
            check(hide(self.flyout.raw()))
        }
    }
    /// Handles picker input. `current(i)` is the color of target `i`; returns the
    /// target and color confirmed with "Done".
    pub fn refresh(&mut self, current: impl Fn(usize) -> u32) -> Result<Option<(usize, u32)>> {
        let mut confirmed = None;
        let events = std::mem::take(&mut *self.events.borrow_mut());
        for event in events {
            if let PickerEvent::Open(target) = event {
                self.open(target, current(target))?;
                continue;
            }
            let Some(editing) = self.editing.as_mut() else {
                continue;
            };
            match event {
                PickerEvent::Drag(part, x, y) => {
                    let fraction = |value: f64, size: f64| (value / size).clamp(0.0, 1.0);
                    match part {
                        Part::Area => {
                            editing.hsv.s = fraction(x, WIDTH);
                            editing.hsv.v = 1.0 - fraction(y, AREA);
                        }
                        Part::Hue => editing.hsv.h = fraction(x, WIDTH) * 359.9,
                        Part::Alpha => editing.alpha = (fraction(x, WIDTH) * 255.0).round() as u8,
                    }
                    self.render()?;
                }
                PickerEvent::Preset(color) => {
                    editing.hsv = Hsv::of(color);
                    editing.alpha = (color >> 24) as u8;
                    self.render()?;
                }
                PickerEvent::Done => {
                    confirmed = Some((editing.target, editing.argb()));
                    self.hide()?;
                }
                PickerEvent::Cancel => self.hide()?,
                PickerEvent::Open(_) => {}
            }
        }
        // Typed HEX or alpha percent take over once they parse.
        if self.editing.is_some() {
            let (hex, percent) = (self.text("PickerHex")?, self.text("PickerAlpha")?);
            let editing = self.editing.as_mut().ok_or(E_FAIL)?;
            if hex != editing.hex {
                editing.hex = hex.clone();
                if let Some(color) = Palette::parse(&hex) {
                    editing.hsv = Hsv::of(color);
                    editing.alpha = (color >> 24) as u8;
                    self.render()?;
                }
            } else if percent != editing.percent {
                editing.percent = percent.clone();
                if let Some(value) = percent
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|v| (0.0..=100.0).contains(v))
                {
                    editing.alpha = (value * 2.55).round() as u8;
                    self.render()?;
                }
            }
        }
        Ok(confirmed)
    }
    /// Opens the picker for `target`, types a HEX value and confirms it.
    pub fn verify(&mut self, target: usize, color: u32) -> Result<Option<(usize, u32)>> {
        self.events.borrow_mut().push(PickerEvent::Open(target));
        self.refresh(|_| 0xFF000000)?;
        for _ in 0..20 {
            NativeWindow::drain();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        self.find("PickerHex")?
            .query(&TEXTBOX)?
            .set_string(7, &hex(color))?;
        self.refresh(|_| 0)?;
        self.events.borrow_mut().push(PickerEvent::Done);
        self.refresh(|_| 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hsv_round_trips_colors_of_the_artboard() {
        for rgb in [0xFF4B4B, 0x3296E1, 0x000000, 0xFFFFFF, 0x45D0B5, 0x964B32] {
            assert_eq!(Hsv::of(rgb).rgb(), rgb);
        }
        let red = Hsv::of(0xFF0000);
        assert_eq!((red.h, red.s, red.v), (0.0, 1.0, 1.0));
        assert!(ColorPicker::markup().contains(r#"x:Name="PickerPreset7""#));
    }
}
