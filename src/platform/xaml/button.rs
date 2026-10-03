use super::{visual_tree::VisualTree, *};
use crate::{
    metrics::{MetricReading, MetricValue},
    platform::devices::{DeviceId, NetworkAdapter},
    presentation::{
        dash_phase::DashPhase,
        history::{self, ChartPoint, MetricHistory},
        pixel_rows::PixelRows,
    },
};
use std::ptr;

const CONTROL: Guid = Guid::from_u128(0xa8912263_2951_4f58_a9c5_5a134eaa7f07);
const POLYLINE: Guid = Guid::from_u128(0x91dc62f8_42b3_47f3_8476_c55124a7c4c6);
const SHAPE: Guid = Guid::from_u128(0x786f2b75_9aa0_454d_ae06_a2466e37c832);
const POLYGON: Guid = Guid::from_u128(0xe3755c19_2e4d_4bcc_8d34_86871957fa01);
const GRADIENT: Guid = Guid::from_u128(0x8e96d16b_bb84_4c6f_9dbf_9d6c5c6d9c39);

/// Stable native buttons: updates replace point data, never interactive elements.
#[derive(Clone)]
pub(super) struct MetricsButton {
    root: Com,
    tiles: Vec<MetricTile>,
    last_numbers: Option<std::time::Instant>,
    width: f64,
    /// Dragging tiles into a new order; taskbar only.
    reorder: Option<super::reorder::TileReorder>,
}
impl MetricsButton {
    pub(super) fn preview(style: &TileStyle, dark: bool) -> Result<Self> {
        let palette = if dark { style.dark } else { style.light };
        let root = Markup::load(&format!(
            r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Orientation="Horizontal" Spacing="{}" Height="36" Background="{}"/>"#,
            style.gap,
            palette.hex(13)
        ))?;
        let children = XamlVector(root.query(&PANEL)?.object(6)?);
        let mut tiles = Vec::new();
        for id in ["cpu", "gpu", "ram", "net", "disk"] {
            let width = style.width + TileKind::extra_width(id);
            let markup = palette
                .apply(&Markup::styled(id, style), style.fade_opacity, id)
                .replace("@WIDTH@", &width.to_string())
                .replace("@LABEL@", &Markup::label(id))
                .replace("@PREFIX@", Markup::prefix(id))
                .replace(
                    "<Button xmlns=",
                    if dark {
                        "<Button RequestedTheme=\"Dark\" xmlns="
                    } else {
                        "<Button RequestedTheme=\"Light\" xmlns="
                    },
                )
                .replace(
                    "{ThemeResource OmniButtonBackgroundPointerOver}",
                    &palette.hex(11),
                )
                .replace(
                    "{ThemeResource OmniButtonBackgroundPressed}",
                    &palette.hex(12),
                );
            let mut tile = MetricTile::from_root(Markup::load(&markup)?, id, width, *style)?;
            // Manual preview inputs describe a value, not a timed pair of samples.
            tile.average_numbers = false;
            super::rounded_clip::RoundedClip::apply(
                &Markup::find(&tile.root, "Chart")?,
                width as f32,
                36.0,
                style.radius as f32,
            )?;
            let samples = (width / history::STEP as f64) as usize + 1;
            // Only the last sample is shown: the others just fill the histories, which
            // saves drawing every tile some 60 times whenever a setting changes.
            for step in 0..samples - 1 {
                tile.record(&PreviewData::readings(step, samples, style));
            }
            tile.update(&PreviewData::readings(samples - 1, samples, style), true)?;
            children.append(&tile.root.query(&UI_ELEMENT)?)?;
            tiles.push(tile);
        }
        let width = tiles.iter().map(|tile| tile.width).sum::<f64>() + 4.0 * style.gap;
        Ok(Self {
            root,
            tiles,
            width,
            last_numbers: None,
            reorder: None,
        })
    }
    pub(super) fn preview_layout(&mut self) -> Result<()> {
        for tile in &self.tiles {
            tile.update_fades()?;
        }
        Ok(())
    }
    pub(super) fn preview_alert_opacity(&self) -> Result<f64> {
        let background = Markup::find(&self.tiles[0].root, "AlertBackground")?;
        XamlElement(background.query(&UI_ELEMENT)?).number(9)
    }
    /// Validate markup and ABI in a disposable XAML host before touching Explorer.
    pub(crate) fn verify() -> Result<()> {
        use crate::metrics::MetricDescriptor;
        for theme in ["Light", "Dark"] {
            for (id, width) in [
                ("cpu", 104.0),
                ("gpu", 104.0),
                ("ram", 84.0),
                ("disk", 112.0),
                ("net", 112.0),
            ] {
                let markup = Markup::styled(id, &TileStyle::default())
                    .replace("@WIDTH@", &format!("{width}"))
                    .replace("@LABEL@", &Markup::label(id))
                    .replace("@PREFIX@", Markup::prefix(id))
                    .replace(
                        "<Button xmlns=",
                        &format!("<Button RequestedTheme=\"{theme}\" xmlns="),
                    )
                    .replace(
                        "{ThemeResource OmniButtonBackgroundPointerOver}",
                        "#16000000",
                    )
                    .replace("{ThemeResource OmniButtonBackgroundPressed}", "#22000000");
                let root = Markup::load(&markup)?;
                let mut tile = MetricTile::from_root(root, id, width, TileStyle::default())?;
                for value in [
                    MetricValue::WarmingUp,
                    MetricValue::Available(90.0),
                    MetricValue::Unavailable,
                    MetricValue::Available(42.0),
                ] {
                    let main = TileKind::rates(id).map_or(id, |[main, _]| main);
                    let readings: Vec<_> = [
                        main,
                        "net_up",
                        "disk_write",
                        "cpu_temperature",
                        "gpu_temperature",
                    ]
                    .into_iter()
                    .map(|reading| MetricReading {
                        descriptor: MetricDescriptor::new(reading, reading, ""),
                        value: value.clone(),
                    })
                    .collect();
                    tile.update(&readings, true)?;
                }
            }
        }
        Self::verify_number_cadence()?;
        Self::verify_reorder()
    }
    /// Drags the first preview tile to the end: pointer handlers attach, the
    /// strip reorders its children and the new order is saved.
    fn verify_reorder() -> Result<()> {
        let style = TileStyle::default();
        let preview = Self::preview(&style, false)?;
        let saved = std::rc::Rc::new(RefCell::new(Vec::new()));
        let sink = saved.clone();
        let reorder = Self::reorder(
            &preview.root,
            &preview.tiles,
            style.gap,
            std::rc::Rc::new(Cell::new(false)),
            std::rc::Rc::new(move |ids: &[String]| *sink.borrow_mut() = ids.to_vec()),
        )?;
        let dropped = reorder.rehearse()?;
        let expected = ["gpu", "ram", "net", "disk", "cpu"];
        let children = XamlVector(preview.root.query(&PANEL)?.object(6)?);
        let last = unsafe {
            let get: unsafe extern "system" fn(Raw, u32, *mut Raw) -> Hr = children.0.slot(6);
            let mut raw = ptr::null_mut();
            check(get(children.0.raw(), children.size()? - 1, &mut raw))?;
            Com::owned(raw)?
        };
        let first = preview.tiles[0].root.query(&UI_ELEMENT)?;
        if dropped != expected || *saved.borrow() != expected || last.raw() != first.raw() {
            return Err(E_FAIL);
        }
        Ok(())
    }
    fn verify_number_cadence() -> Result<()> {
        use crate::metrics::MetricDescriptor;
        let mut preview = Self::preview(&TileStyle::default(), false)?;
        preview.tiles[0].history = MetricHistory::spanning(100.0);
        preview.tiles[0].average_numbers = true;
        let readings = |value| {
            [MetricReading {
                descriptor: MetricDescriptor::new("cpu", "CPU", "%"),
                value: MetricValue::Available(value),
            }]
        };
        preview.update(&readings(10.0))?;
        let before = preview.tiles[0].history.points(100.0).last().unwrap().y;
        preview.update(&readings(20.0))?;
        if preview.tiles[0].value.query(&TEXT)?.string(26)? != "10%"
            || preview.tiles[0].history.points(100.0).last().unwrap().y == before
        {
            return Err(E_FAIL);
        }
        preview.last_numbers = Some(std::time::Instant::now() - std::time::Duration::from_secs(1));
        preview.update(&readings(30.0))?;
        if preview.tiles[0].value.query(&TEXT)?.string(26)? != "25%" {
            return Err(E_FAIL);
        }
        Ok(())
    }
    /// `save` receives the tile ids after the user drags them into a new order.
    pub(super) fn new(
        panel: &Com,
        ids: &[String],
        style: &TileStyle,
        save: super::reorder::SaveOrder,
    ) -> Result<Self> {
        let root = Markup::load(&format!(
            r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Name="TaskbarMetricsButton" Orientation="Horizontal" Spacing="{}" Height="36"/>"#,
            style.gap
        ))?;
        let children = XamlVector(root.query(&PANEL)?.object(6)?);
        let mut tiles = Vec::new();
        let mut width = 0.0;
        let dragging = std::rc::Rc::new(Cell::new(false));
        for id in ids {
            let tile = MetricTile::new(panel, id, style, dragging.clone())?;
            children.append(&tile.root.query(&UI_ELEMENT)?)?;
            width += tile.width + if tiles.is_empty() { 0.0 } else { style.gap };
            tiles.push(tile);
        }
        let reorder = Self::reorder(&root, &tiles, style.gap, dragging, save)?;
        Ok(Self {
            root,
            tiles,
            width,
            last_numbers: None,
            reorder: Some(reorder),
        })
    }
    fn reorder(
        root: &Com,
        tiles: &[MetricTile],
        gap: f64,
        dragging: std::rc::Rc<Cell<bool>>,
        save: super::reorder::SaveOrder,
    ) -> Result<super::reorder::TileReorder> {
        let slides = tiles
            .iter()
            .map(|tile| {
                super::reorder::Slide::new(
                    &tile.root,
                    &Markup::find(&tile.root, "Shift")?,
                    &Markup::find(&tile.root, "Slide")?,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        super::reorder::TileReorder::new(
            root,
            slides,
            tiles.iter().map(|tile| tile.id.clone()).collect(),
            tiles.iter().map(|tile| tile.width).collect(),
            gap,
            dragging,
            save,
        )
    }
    /// Tile ids in their order on the taskbar.
    pub(super) fn ids(&self) -> Option<Vec<String>> {
        self.reorder.as_ref().map(|reorder| reorder.ids())
    }
    pub(super) fn width(&self) -> f64 {
        self.width
    }
    pub(super) fn restore_history(&mut self, previous: &Self) {
        for tile in &mut self.tiles {
            if let Some(old) = previous.tiles.iter().find(|old| old.id == tile.id) {
                tile.history.replay(&old.history);
                tile.upload_history.replay(&old.upload_history);
                tile.temperature_history.replay(&old.temperature_history);
                // The temperature line keeps its row and its dashes their samples.
                tile.dash_phase = old.dash_phase.clone();
                tile.temperature_rows = old.temperature_rows.clone();
            }
        }
    }
    pub(super) fn element(&self) -> Result<Com> {
        self.root.query(&UI_ELEMENT)
    }
    pub(super) fn framework(&self) -> Result<XamlElement> {
        Ok(XamlElement(self.root.query(&FRAMEWORK)?))
    }
    pub(super) fn update(&mut self, readings: &[MetricReading]) -> Result<()> {
        let now = std::time::Instant::now();
        let numbers = self
            .last_numbers
            .is_none_or(|last| now.duration_since(last).as_millis() >= 950);
        if numbers {
            self.last_numbers = Some(now);
        }
        for tile in &mut self.tiles {
            tile.update(readings, numbers)?;
        }
        Ok(())
    }
    pub(super) fn visible(&self, visible: bool) -> Result<()> {
        let element = self.element()?;
        unsafe {
            let hit: unsafe extern "system" fn(Raw, u8) -> Hr = element.slot(20);
            check(hit(element.raw(), u8::from(visible)))?;
        }
        XamlElement(element).set_number(10, if visible { 1.0 } else { 0.0 })
    }
}

struct Markup;
impl Markup {
    /// `GPU2` (`gpu@1`: numbers count from 1), `DISK3`, `D:`, `WI‑FI` for a device,
    /// `DISK` for the main device of a kind.
    fn label(id: &str) -> String {
        let Some(device) = DeviceId::parse(id) else {
            return id.to_uppercase();
        };
        match (device.ordinal(), &device.tag) {
            (Some(ordinal), _) => format!("{}{ordinal}", device.kind.to_uppercase()),
            // An adapter is named by its link, as in the window's menu.
            (None, Some(tag)) if device.kind == "net" => {
                NetworkAdapter::label(&NetworkAdapter::list(), tag)
                    .unwrap_or_else(|| tag.to_uppercase())
            }
            (None, Some(tag)) => tag.to_uppercase(),
            (None, None) => id.to_uppercase(),
        }
    }
    /// Dashes of the temperature line: 3 px with 2 px gaps.
    const DASH: [f64; 2] = [3.0, 2.0];
    fn temperature_dashes(id: &str, style: &TileStyle) -> String {
        if !(style.dashed_temperature && matches!(id, "cpu" | "gpu")) {
            return String::new();
        }
        // XAML measures dashes in stroke thicknesses.
        let stroke = style.stroke.max(0.1);
        format!(
            r#" StrokeDashArray="{},{}""#,
            Self::DASH[0] / stroke,
            Self::DASH[1] / stroke
        )
    }
    fn styled(id: &str, style: &TileStyle) -> String {
        Palette::apply_defaults(&Self::template(id), style.fade_opacity, id)
            .replace(
                r#"<Polyline x:Name="Upload""#,
                &format!(
                    r#"<Polyline x:Name="Upload"{}"#,
                    Self::temperature_dashes(id, style)
                ),
            )
            .replace(
                "@ALERT_MAX@",
                &(style.alerts.alert_opacity / 100.0).to_string(),
            )
            .replace(
                "@ALERT_MIN@",
                &(style.alerts.alert_opacity / 400.0).to_string(),
            )
            .replace(
                "@ALERT_HALF_PERIOD@",
                &format!("{:.3}", style.alerts.pulse_seconds / 2.0),
            )
            .replace("@PREFIX@", Markup::prefix(id))
            .replace("FontSize=\"9\"", "FontSize=\"@LABEL_SIZE@\"")
            .replace("FontSize=\"13\"", "FontSize=\"@VALUE_SIZE@\"")
            .replace("FontSize=\"10\"", "FontSize=\"@SECONDARY_SIZE@\"")
            .replace("@LABEL_SIZE@", &style.label_size.to_string())
            .replace("@VALUE_SIZE@", &style.value_size.to_string())
            .replace("@SECONDARY_SIZE@", &style.secondary_size.to_string())
            .replace(
                "CornerRadius=\"5\"",
                &format!("CornerRadius=\"{}\"", style.radius),
            )
            // One pixel outside the tile, the hover border keeps its corners parallel.
            .replace(
                "@HOVER_RADIUS@",
                &(if style.radius > 0.0 {
                    style.radius + 1.0
                } else {
                    0.0
                })
                .to_string(),
            )
            .replace(
                "StrokeThickness=\"1.25\"",
                &format!("StrokeThickness=\"{}\"", style.stroke),
            )
            .replace(
                "Opacity=\"0.22\"",
                &format!("Opacity=\"{}\"", style.opacity / 100.0),
            )
            .replace(
                "TranslateTransform X=\"-1\" Y=\"-1\"",
                &format!(
                    "TranslateTransform X=\"{}\" Y=\"{}\"",
                    style.offset_x, style.offset_y
                ),
            )
            .replace(
                "@FADE_ALPHA@",
                &format!("{:02X}", (style.fade_opacity * 2.55).round() as u8),
            )
    }
    fn template(id: &str) -> String {
        include_str!("metric_tile.xaml")
            .replace(
                "@SECONDARY_DARK@",
                if matches!(id, "cpu" | "gpu") {
                    "FFB454"
                } else {
                    "C3A6FF"
                },
            )
            .replace(
                "@SECONDARY_LIGHT@",
                if matches!(id, "cpu" | "gpu") {
                    "A34F00"
                } else {
                    "6B3FD4"
                },
            )
            .replace(
                "@SECONDARY_COLOR@",
                if matches!(id, "cpu" | "gpu") {
                    "MetricTemperature"
                } else {
                    "MetricUpload"
                },
            )
            .replace(
                "@SECONDARY_BRUSH@",
                if matches!(id, "cpu" | "gpu") {
                    "MetricTemperatureText"
                } else {
                    "MetricUp"
                },
            )
    }
    fn prefix(id: &str) -> &'static str {
        // Do not create a collapsed first child: StackPanel spacing must not
        // indent percentage values relative to the header.
        if TileKind::rates(id).is_some() {
            r#"<TextBlock Text="↓" FontSize="13" FontWeight="SemiBold" Foreground="{ThemeResource MetricDown}" Margin="0,0,-2,0"/>"#
        } else {
            ""
        }
    }
    fn load(markup: &str) -> Result<Com> {
        let reader = factory(
            "Windows.UI.Xaml.Markup.XamlReader",
            &Guid::from_u128(0x9891c6bd_534f_4955_b85a_8a8dc0dca602),
        )?;
        let markup = HString::new(markup)?;
        let mut raw = ptr::null_mut();
        unsafe {
            let load: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = reader.slot(6);
            check(load(reader.raw(), markup.0, &mut raw))?;
            Com::owned(raw)
        }
    }
    fn find(root: &Com, name: &str) -> Result<Com> {
        let frame = root.query(&FRAMEWORK)?;
        let name = HString::new(name)?;
        let mut raw = ptr::null_mut();
        unsafe {
            let find: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = frame.slot(51);
            check(find(frame.raw(), name.0, &mut raw))?;
            Com::owned(raw)
        }
    }
}

#[derive(Clone)]
struct MetricTile {
    click: Option<super::events::Subscription>,
    average_numbers: bool,
    root: Com,
    id: String,
    /// Kind and tag of `id`; readings of a tagged device carry its tag.
    device: DeviceId,
    width: f64,
    value: Com,
    hot_value: Com,
    secondary: Com,
    text_group: Com,
    line: Com,
    area: Com,
    upload: Com,
    /// IShape of the upload or temperature line.
    upload_shape: Com,
    fades: Vec<Com>,
    history: MetricHistory,
    upload_history: MetricHistory,
    /// The temperature line of a CPU or GPU tile, on whole pixel rows.
    temperature_history: MetricHistory,
    temperature_rows: PixelRows,
    dash_phase: DashPhase,
    style: TileStyle,
    fade_bounds: [(f32, f32); 3],
    alert: super::alert::AlertVisual,
}
impl MetricTile {
    fn new(
        panel: &Com,
        id: &str,
        style: &TileStyle,
        dragging: std::rc::Rc<Cell<bool>>,
    ) -> Result<Self> {
        // Everything but the title and the readings depends only on the kind.
        let device = DeviceId::parse(id).ok_or(E_FAIL)?;
        let kind = device.kind.as_str();
        let width = style.width + TileKind::extra_width(kind);
        let markup = style
            .dark
            .apply_theme(
                &style.light.apply_theme(
                    &Markup::styled(kind, style),
                    style.fade_opacity,
                    kind,
                    "Light",
                ),
                style.fade_opacity,
                kind,
                "Default",
            )
            .replace("@WIDTH@", &format!("{width}"))
            .replace("@LABEL@", &Markup::label(id))
            .replace("@PREFIX@", Markup::prefix(kind));
        let markup = markup
            .replace("OmniButtonBackgroundPointerOver", "MetricHover")
            .replace("OmniButtonBackgroundPressed", "MetricPressed");
        let root =
            Markup::load(&markup).inspect_err(|hr| log(&format!("Tile markup: {hr:08X}")))?;
        ClockResources::inherit(panel, &root)?;
        let mut tile = Self::from_root(root, id, width, *style)?;
        let resource = id.to_owned();
        tile.click = Some(super::events::Subscription::click(&tile.root, move || {
            // Releasing a dragged tile is a drop, not a click.
            if dragging.get() {
                return Ok(());
            }
            if let Err(error) = crate::platform::window_launcher::WindowLauncher::open(&resource) {
                log(&format!("Open metrics window failed: {error:08X}"));
            }
            Ok(())
        })?);
        Ok(tile)
    }

    fn from_root(root: Com, id: &str, width: f64, style: TileStyle) -> Result<Self> {
        super::rounded_clip::RoundedClip::apply(
            &Markup::find(&root, "Chart")?,
            width as f32,
            36.0,
            style.radius as f32,
        )?;
        let control = root.query(&CONTROL)?;
        unsafe {
            let mut applied = 0u8;
            let apply: unsafe extern "system" fn(Raw, *mut u8) -> Hr = control.slot(45);
            check(apply(control.raw(), &mut applied))?;
        }
        let value = Markup::find(&root, "Value")?;
        let hot_value = Markup::find(&root, "HotValue")?;
        let secondary = Markup::find(&root, "Secondary")?;
        let text_group = Markup::find(&root, "TextGroup")?.query(&FRAMEWORK)?;
        let line = Markup::find(&root, "Line")?.query(&POLYLINE)?.object(8)?;
        let area = Markup::find(&root, "Area")?.query(&POLYGON)?.object(8)?;
        let upload_element = Markup::find(&root, "Upload")?;
        let upload = upload_element.query(&POLYLINE)?.object(8)?;
        let upload_shape = upload_element.query(&SHAPE)?;
        let fades = ["LineFade", "AreaFade", "UploadFade"]
            .iter()
            .map(|name| Markup::find(&root, name)?.query(&GRADIENT))
            .collect::<Result<Vec<_>>>()?;
        let alert = super::alert::AlertVisual::new(
            Markup::find(&root, "AlertBackground")?,
            Markup::find(&root, "AlertPulse")?,
        )?;
        Ok(Self {
            click: None,
            alert,
            average_numbers: true,
            root,
            id: id.into(),
            device: DeviceId::parse(id).unwrap_or_else(|| DeviceId::new(id, None)),
            width,
            value,
            hot_value,
            secondary,
            text_group,
            line,
            area,
            upload,
            upload_shape,
            fades,
            history: MetricHistory::spanning(Self::chart_width(width, &style)),
            upload_history: MetricHistory::spanning(Self::chart_width(width, &style)),
            temperature_history: MetricHistory::spanning(Self::chart_width(width, &style)),
            temperature_rows: PixelRows::default(),
            dash_phase: DashPhase::default(),
            style,
            fade_bounds: [(0.0, width as f32); 3],
        })
    }
    /// The reading `base` of `device`, matched without building its id.
    fn reading<'a>(
        readings: &'a [MetricReading],
        device: &DeviceId,
        base: &str,
    ) -> &'a MetricValue {
        readings
            .iter()
            .find(|r| device.is_reading(&r.descriptor.id, base))
            .map(|r| &r.value)
            .unwrap_or(&MetricValue::Unavailable)
    }
    fn number(value: &MetricValue, rate: bool) -> String {
        match value {
            MetricValue::Available(n) if rate => {
                if *n >= 100.0 {
                    format!("{n:.0}")
                } else if *n >= 10.0 {
                    format!("{n:.1}")
                } else {
                    format!("{n:.2}")
                }
            }
            MetricValue::Available(n) => format!("{n:.0}%"),
            // A reading that failed leaves its place empty.
            _ => String::new(),
        }
    }
    fn display_value(&self, history: &MetricHistory, current: &MetricValue) -> MetricValue {
        if self.average_numbers {
            history.mean_last_two(current)
        } else {
            current.clone()
        }
    }
    /// The tile's main reading and its secondary series: upload or write of a rate
    /// tile, temperature of a CPU or GPU tile.
    fn series<'a>(
        &self,
        readings: &'a [MetricReading],
    ) -> (&'a MetricValue, Option<&'a MetricValue>) {
        let kind = self.device.kind.as_str();
        let (main, secondary) = match (TileKind::rates(kind), kind) {
            (Some([main, second]), _) => (main, Some(second)),
            (None, "cpu") => (kind, Some("cpu_temperature")),
            (None, "gpu") => (kind, Some("gpu_temperature")),
            (None, _) => (kind, None),
        };
        let main = Self::reading(readings, &self.device, main);
        (
            main,
            secondary.map(|base| Self::reading(readings, &self.device, base)),
        )
    }
    /// Adds `readings` to the chart histories without touching the view.
    fn record(&mut self, readings: &[MetricReading]) {
        let (main, secondary) = self.series(readings);
        self.history.push(main);
        if let Some(secondary) = secondary {
            self.upload_history.push(secondary);
            if matches!(self.device.kind.as_str(), "cpu" | "gpu") {
                let row = self
                    .temperature_rows
                    .snap(secondary, std::time::Instant::now());
                self.temperature_history.push(&row);
                self.dash_phase.push();
            }
        }
    }
    fn update(&mut self, readings: &[MetricReading], numbers: bool) -> Result<()> {
        self.record(readings);
        self.render(readings, numbers)
    }
    /// Shows the histories and the latest `readings`.
    fn render(&mut self, readings: &[MetricReading], numbers: bool) -> Result<()> {
        let kind = self.device.kind.clone();
        let rates = TileKind::rates(&kind).is_some();
        let (reading, secondary) = self.series(readings);
        if numbers {
            let display = self.display_value(&self.history, reading);
            let value = Self::number(&display, rates);
            self.value.query(&TEXT)?.set_string(27, &value)?;
            self.hot_value.query(&TEXT)?.set_string(27, &value)?;
            let hot = match display {
                MetricValue::Available(n) => self.style.alerts.hot(&kind, n),
                _ => 0.0,
            };
            // The red copy fades in over the number; the plain one leaves once it is
            // fully covered, so no plain-colored rim stays around the glyphs.
            XamlElement(self.value.query(&UI_ELEMENT)?)
                .set_enum(22, if hot >= 1.0 { 1 } else { 0 })?;
            let red = XamlElement(self.hot_value.query(&UI_ELEMENT)?);
            red.set_enum(22, if hot > 0.0 { 0 } else { 1 })?;
            red.set_number(10, hot)?;
        }
        let minimum_maximum = 1.0;
        let maximum = if rates {
            self.history.peak().max(minimum_maximum)
        } else {
            100.0
        };
        let points = if rates {
            self.history.points_with_hidden_zero(maximum)
        } else {
            self.history.points(maximum)
        };
        let points = self.position_points(points);
        self.fade_bounds[0] = Self::bounds(&points);
        self.fade_bounds[1] = Self::bounds(&points);
        Self::points(&self.line, &points)?;
        let mut area = points.clone();
        if let (Some(first), Some(last)) = (points.first(), points.last()) {
            area.push(ChartPoint { x: last.x, y: 36.0 });
            area.push(ChartPoint {
                x: first.x,
                y: 36.0,
            });
        }
        Self::points(&self.area, &area)?;
        if rates {
            let up = secondary.unwrap_or(&MetricValue::Unavailable);
            if numbers {
                let up = Self::number(&self.display_value(&self.upload_history, up), true);
                let text = if up.is_empty() {
                    up
                } else {
                    format!("↑{up}")
                };
                self.secondary.query(&TEXT)?.set_string(27, &text)?;
            }
            self.secondary_points(
                self.upload_history
                    .points_with_hidden_zero(self.upload_history.peak().max(minimum_maximum)),
            )?;
        }
        if matches!(kind.as_str(), "cpu" | "gpu") && !self.style.temperature(&kind) {
            if numbers {
                self.secondary.query(&TEXT)?.set_string(27, "")?;
            }
            self.secondary_points(Vec::new())?;
        } else if matches!(kind.as_str(), "cpu" | "gpu") {
            let temperature = secondary.unwrap_or(&MetricValue::Unavailable);
            if numbers {
                // A sensor that is missing, as in a virtual machine, leaves the place empty.
                let text = match self.display_value(&self.upload_history, temperature) {
                    MetricValue::Available(value) => format!("{value:.0}°"),
                    _ => String::new(),
                };
                self.secondary.query(&TEXT)?.set_string(27, &text)?;
            }
            // 0–100 °C like the temperature axis of the window: 50 °C sits mid-height.
            let mut points = self.temperature_history.points_range(0.0, 100.0);
            if self.style.dashed_temperature {
                // The dashes scroll with the samples instead of restarting at the
                // left edge; XAML counts the offset in stroke thicknesses. Whole-pixel
                // runs keep every dash on whole pixels.
                let period = Markup::DASH[0] + Markup::DASH[1];
                let offset = self.dash_phase.offset(&points, period);
                XamlElement(self.upload_shape.clone())
                    .set_number(21, offset / self.style.stroke.max(0.1))?;
                points = DashPhase::staircase(&points);
            }
            self.secondary_points(points)?;
        }
        let alert_value = Self::reading(
            readings,
            &self.device,
            match kind.as_str() {
                "cpu" => "cpu_temperature",
                "gpu" => "gpu_temperature",
                kind => kind,
            },
        );
        self.alert.update(&self.style.alerts, &kind, alert_value)?;
        self.update_fades()
    }
    fn update_fades(&self) -> Result<()> {
        let edge = XamlElement(self.text_group.clone()).number(13)? + 6.0;
        let start_px = edge + self.style.fade_offset;
        for (brush, (left, right)) in self.fades.iter().zip(self.fade_bounds) {
            let span = (right - left).max(0.001);
            let start = (start_px as f32 - left) / span;
            unsafe {
                let set_start: unsafe extern "system" fn(Raw, ChartPoint) -> Hr = brush.slot(7);
                let set_end: unsafe extern "system" fn(Raw, ChartPoint) -> Hr = brush.slot(9);
                check(set_start(brush.raw(), ChartPoint { x: start, y: 0.0 }))?;
                check(set_end(
                    brush.raw(),
                    ChartPoint {
                        x: start + self.style.fade_width.max(0.01) as f32 / span,
                        y: 0.0,
                    },
                ))?;
            }
        }
        Ok(())
    }
    /// The chart starts `graph_left` px in and runs to the right edge.
    fn chart_left(width: f64, style: &TileStyle) -> f32 {
        style.graph_left.min(width - 1.0) as f32
    }
    fn chart_width(width: f64, style: &TileStyle) -> f32 {
        width as f32 - Self::chart_left(width, style)
    }
    fn position_points(&self, mut points: Vec<ChartPoint>) -> Vec<ChartPoint> {
        let left = Self::chart_left(self.width, &self.style);
        for point in &mut points {
            point.x += left;
        }
        points
    }
    fn bounds(points: &[ChartPoint]) -> (f32, f32) {
        match (points.first(), points.last()) {
            (Some(first), Some(last)) => (first.x, last.x),
            _ => (0.0, 1.0),
        }
    }
    fn secondary_points(&mut self, points: Vec<ChartPoint>) -> Result<()> {
        let points = self.position_points(points);
        self.fade_bounds[2] = Self::bounds(&points);
        Self::points(&self.upload, &points)
    }
    fn points(vector: &Com, points: &[ChartPoint]) -> Result<()> {
        unsafe {
            let replace: unsafe extern "system" fn(Raw, u32, *const ChartPoint) -> Hr =
                vector.slot(17);
            check(replace(vector.raw(), points.len() as u32, points.as_ptr()))
        }
    }
}

/// What a tile kind shows: two rates in MB/s, network down/up or disk read/write,
/// or one percentage, with the temperature on CPU and GPU tiles.
struct TileKind;
impl TileKind {
    /// Readings of the two rates, the main one first.
    fn rates(kind: &str) -> Option<[&'static str; 2]> {
        match kind {
            "net" => Some(["net_down", "net_up"]),
            "disk" => Some(["disk_read", "disk_write"]),
            _ => None,
        }
    }
    /// Width beyond the style's: two rates need room, RAM shows one short number.
    fn extra_width(kind: &str) -> f64 {
        match kind {
            _ if Self::rates(kind).is_some() => 8.0,
            "cpu" | "gpu" => 0.0,
            _ => -20.0,
        }
    }
}

struct PreviewData;
impl PreviewData {
    fn readings(step: usize, samples: usize, style: &TileStyle) -> Vec<MetricReading> {
        use crate::metrics::MetricDescriptor;
        let wave = [
            18.0, 22.0, 25.0, 20.0, 31.0, 28.0, 40.0, 35.0, 44.0, 30.0, 33.0, 38.0, 29.0, 26.0,
            35.0, 41.0, 37.0, 30.0, 32.0, 34.0,
        ];
        // Upload has a shape of its own, short bursts: a scaled copy of the download
        // looks the same once each line fits its own scale.
        let upload = [
            4.0, 3.0, 5.0, 12.0, 6.0, 4.0, 3.0, 9.0, 15.0, 7.0, 4.0, 5.0, 3.0, 11.0, 6.0, 4.0, 8.0,
            5.0, 3.0, 6.0,
        ];
        // The disk runs the same shapes from elsewhere, so it does not mirror the network.
        let (disk_read, disk_write) = (
            wave[(step + 9) % wave.len()],
            upload[(step + 5) % upload.len()],
        );
        let upload = upload[step % upload.len()];
        let wave = wave[step % wave.len()];
        [
            ("cpu", wave, style.demo_cpu_load),
            ("gpu", wave + 55.0, style.demo_gpu_load),
            ("ram", 48.0 + wave / 5.0, style.demo_ram),
            ("disk_read", disk_read / 2.0, style.demo_disk_read),
            ("disk_write", disk_write / 2.0, style.demo_disk_write),
            ("net_down", wave / 5.0, style.demo_download),
            ("net_up", upload / 5.0, style.demo_upload),
            ("cpu_temperature", 52.0 + wave / 3.0, style.demo_cpu),
            ("gpu_temperature", 55.0 + wave / 2.0, style.demo_gpu),
        ]
        .into_iter()
        .map(|(id, historical, current)| MetricReading {
            descriptor: MetricDescriptor::new(id, id, ""),
            value: MetricValue::Available(if step + 1 == samples {
                current
            } else {
                historical
            }),
        })
        .collect()
    }
}

struct ClockResources;
impl ClockResources {
    fn inherit(panel: &Com, button: &Com) -> Result<()> {
        let tree = VisualTree::new()?;
        let clock = ClockCatalog::find(panel)?;
        let resources = button.query(&FRAMEWORK)?.object(7)?;
        let merged = XamlVector(resources.object(8)?);
        for ancestor in tree.ancestors(&clock)? {
            if let Ok(framework) = ancestor.query(&FRAMEWORK) {
                let copy = super::resources::ClockResourceCopier::copy(&framework.object(7)?)?;
                merged.append(&copy)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiles_title_devices_by_tag_and_main_devices_by_kind() {
        assert_eq!(Markup::label("disk"), "DISK");
        assert_eq!(Markup::label("disk@D:"), "D:");
        // An adapter that is not up keeps its name; one that is gets its link.
        assert_eq!(Markup::label("net@Absent adapter 7"), "ABSENT ADAPTER 7");
        assert_eq!(Markup::label("gpu@1"), "GPU2");
        assert_eq!(Markup::label("disk@2"), "DISK3");
    }
}
