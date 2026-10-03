use super::*;

/// Appearance inputs for the isolated design playground, in XAML logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TileStyle {
    pub alerts: AlertSettings,
    pub demo_cpu: f64,
    pub demo_gpu: f64,
    pub demo_ram: f64,
    pub demo_cpu_load: f64,
    pub demo_gpu_load: f64,
    /// Disk read and write, MB/s.
    pub demo_disk_read: f64,
    pub demo_disk_write: f64,
    pub demo_download: f64,
    pub demo_upload: f64,
    pub width: f64,
    pub gap: f64,
    pub radius: f64,
    pub label_size: f64,
    pub value_size: f64,
    pub secondary_size: f64,
    pub stroke: f64,
    pub opacity: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub graph_left: f64,
    pub fade_offset: f64,
    pub fade_width: f64,
    pub fade_opacity: f64,
    /// Temperature number and line on the CPU and on the GPU tile.
    pub cpu_temperature: bool,
    pub gpu_temperature: bool,
    /// CPU/GPU temperature drawn dashed, like the window charts.
    pub dashed_temperature: bool,
    pub light: Palette,
    pub dark: Palette,
}
impl Default for TileStyle {
    fn default() -> Self {
        Self {
            alerts: AlertSettings::default(),
            demo_cpu: 65.0,
            demo_gpu: 72.0,
            demo_ram: 55.0,
            demo_cpu_load: 34.00,
            demo_gpu_load: 89.00,
            demo_disk_read: 23.40,
            demo_disk_write: 4.12,
            demo_download: 6.80,
            demo_upload: 2.83,
            width: 100.0,
            gap: 6.0,
            radius: 5.0,
            label_size: 10.0,
            value_size: 14.0,
            secondary_size: 11.0,
            stroke: 1.0,
            opacity: 30.0,
            offset_x: -1.0,
            offset_y: 0.0,
            graph_left: 0.0,
            fade_offset: -10.0,
            fade_width: 50.0,
            fade_opacity: 15.0,
            cpu_temperature: true,
            gpu_temperature: true,
            dashed_temperature: false,
            light: Palette::light(),
            dark: Palette::dark(),
        }
    }
}
impl TileStyle {
    /// Whether a tile of `kind` shows its temperature; only CPU and GPU tiles have one.
    pub fn temperature(&self, kind: &str) -> bool {
        match kind {
            "cpu" => self.cpu_temperature,
            "gpu" => self.gpu_temperature,
            _ => false,
        }
    }
}

pub(crate) struct WidgetPreview(button::MetricsButton);
impl WidgetPreview {
    pub fn new(style: &TileStyle, dark: bool) -> Result<Self> {
        Ok(Self(button::MetricsButton::preview(style, dark)?))
    }
    pub fn element(&self) -> Result<Com> {
        self.0.element()
    }
    pub fn refresh_layout(&mut self) -> Result<()> {
        self.0.preview_layout()
    }
    pub fn alert_opacity(&self) -> Result<f64> {
        self.0.preview_alert_opacity()
    }
}
