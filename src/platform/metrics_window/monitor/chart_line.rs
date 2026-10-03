use super::{ui::Ui, *};
use std::path::PathBuf;

/// Width of the total line in the window's chart, kept in the data directory.
/// Pinned and highlighted processes keep their proportion to it.
pub(super) struct ChartLine;

impl ChartLine {
    pub const DEFAULT: f64 = 1.0;
    const RANGE: (f64, f64) = (1.0, 4.0);
    const STEP: f64 = 0.25;

    fn path() -> Option<PathBuf> {
        crate::platform::data_directory::DataDirectory::file("taskbar-metrics.chart").ok()
    }

    /// Checks and demos draw the default line.
    pub fn load() -> f64 {
        persistent()
            .then(Self::path)
            .flatten()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| Self::parse(&text))
            .unwrap_or(Self::DEFAULT)
    }

    pub fn save(width: f64) -> std::io::Result<()> {
        if !persistent() {
            return Ok(());
        }
        let path = Self::path().ok_or(std::io::ErrorKind::NotFound)?;
        std::fs::write(path, format!("line={width}\n"))
    }

    fn parse(text: &str) -> Option<f64> {
        text.lines()
            .find_map(|line| line.trim().strip_prefix("line="))
            .and_then(|value| value.trim().parse::<f64>().ok())
            .filter(|width| width.is_finite())
            .map(Self::snap)
    }

    /// The nearest step inside the range.
    fn snap(width: f64) -> f64 {
        ((width / Self::STEP).round() * Self::STEP).clamp(Self::RANGE.0, Self::RANGE.1)
    }

    /// The settings row: icon, title, caption, slider and the width it shows.
    pub fn markup() -> String {
        format!(
            r#"<Grid MinHeight="64" Padding="16,10" ColumnSpacing="16" BorderBrush="$divider$" BorderThickness="0,1,0,0"><Grid.ColumnDefinitions><ColumnDefinition Width="Auto"/><ColumnDefinition/><ColumnDefinition Width="200"/><ColumnDefinition Width="48"/></Grid.ColumnDefinitions>$icon:chart:16:1.3:text2$<StackPanel Grid.Column="1" Spacing="1" VerticalAlignment="Center"><TextBlock Text="@Chart line width@"/><TextBlock Text="@Total load; pinned processes in proportion@" FontSize="12" Foreground="$text2$" TextWrapping="Wrap"/></StackPanel><Slider x:Name="ChartLine" Grid.Column="2" Minimum="{}" Maximum="{}" StepFrequency="{}" VerticalAlignment="Center" AutomationProperties.Name="@Chart line width@"/><TextBlock x:Name="ChartLineValue" Grid.Column="3" FontSize="12" Foreground="$text2$" HorizontalAlignment="Right" VerticalAlignment="Center" Typography.NumeralAlignment="Tabular"/></Grid>"#,
            Self::RANGE.0,
            Self::RANGE.1,
            Self::STEP
        )
    }
}

/// The slider of `ChartLine::markup`, saving each new width.
pub(super) struct ChartLineSlider {
    root: Com,
    slider: Com,
    shown: f64,
}

impl ChartLineSlider {
    pub fn new(root: &Com) -> Result<Self> {
        let slider = Self {
            root: root.clone(),
            slider: Ui::find(root, "ChartLine")?,
            shown: ChartLine::load(),
        };
        Ui::range(&slider.slider, Some(slider.shown))?;
        slider.caption()?;
        Ok(slider)
    }

    fn caption(&self) -> Result<()> {
        Ui::text(&self.root, "ChartLineValue", &format!("{} px", self.shown))
    }

    /// The new width once the slider moved; a failed write keeps it for this run.
    pub fn refresh(&mut self) -> Result<Option<f64>> {
        let width = ChartLine::snap(Ui::range(&self.slider, None)?);
        if width == self.shown {
            return Ok(None);
        }
        self.shown = width;
        let _ = ChartLine::save(width);
        self.caption()?;
        Ok(Some(width))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_widths_snap_into_the_range() {
        assert_eq!(ChartLine::parse("line=2.5\n"), Some(2.5));
        assert_eq!(ChartLine::parse("line=2.4"), Some(2.5));
        assert_eq!(ChartLine::parse("line=0.2"), Some(1.0));
        assert_eq!(ChartLine::parse("line=9"), Some(4.0));
        assert_eq!(ChartLine::parse("line=NaN"), None);
        assert_eq!(ChartLine::parse("width=2"), None);
    }
}
