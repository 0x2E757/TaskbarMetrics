use crate::metrics::{MetricReading, MetricValue};

pub mod dash_phase;
pub mod history;
pub mod pixel_rows;

pub trait MetricFormatter {
    fn format(&self, readings: &[MetricReading]) -> String;
}

pub struct CompactFormatter;

impl MetricFormatter for CompactFormatter {
    fn format(&self, readings: &[MetricReading]) -> String {
        readings
            .iter()
            .map(|reading| {
                let value = match reading.value {
                    MetricValue::Available(n) => format!("{n:.0}{}", reading.descriptor.unit),
                    _ => "--".into(),
                };
                format!("{} {value}", reading.descriptor.label)
            })
            .collect::<Vec<_>>()
            .join("   ")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OccupiedRange {
    pub left: f64,
    pub right: f64,
    /// The weather: the tiles may follow it at their own spacing, as one more tile.
    pub snug: bool,
}

/// Chooses the first free interval within the left half of the taskbar.
/// No native window, DPI or XAML knowledge is needed for this policy.
pub struct LeftPlacement {
    width: f64,
    gap: f64,
    /// Space after a snug range: the spacing between tiles.
    snug: f64,
}

impl LeftPlacement {
    pub fn new(width: f64, gap: f64, snug: f64) -> Self {
        Self { width, gap, snug }
    }

    pub fn position(&self, taskbar_width: f64, occupied: &[OccupiedRange]) -> Option<f64> {
        if !taskbar_width.is_finite() || taskbar_width <= 0.0 {
            return None;
        }
        let limit = taskbar_width / 2.0;
        let mut ranges = occupied.to_vec();
        if ranges
            .iter()
            .any(|r| !r.left.is_finite() || !r.right.is_finite() || r.right < r.left)
        {
            return None;
        }
        ranges.sort_by(|a, b| a.left.total_cmp(&b.left));
        let mut left = self.gap;
        for range in ranges {
            if range.right <= left {
                continue;
            }
            if left + self.width + self.gap <= range.left {
                break;
            }
            left = left.max(range.right + if range.snug { self.snug } else { self.gap });
        }
        (left + self.width + self.gap <= limit).then_some(left)
    }
}

/// Tiles side by side; the ones that do not fit give way to a marker after the rest.
pub struct TileStrip {
    /// Tile widths in their order.
    pub widths: Vec<f64>,
    /// Space between tiles, and between the last tile shown and the marker.
    pub spacing: f64,
    pub marker: f64,
    /// Least width of the strip when every tile shows.
    pub minimum: f64,
}

impl TileStrip {
    /// Width of the first `shown` tiles, with the marker after them when some are left out.
    pub fn width(&self, shown: usize) -> f64 {
        let tiles = self.widths[..shown].iter().sum::<f64>()
            + self.spacing * shown.saturating_sub(1) as f64;
        match shown {
            _ if shown == self.widths.len() => tiles.max(self.minimum),
            0 => self.marker,
            _ => tiles + self.spacing + self.marker,
        }
    }

    /// Where the strip starts and how many tiles it shows: all when they fit, else as
    /// many as fit before the marker; `None` when not even the marker fits.
    pub fn place(
        &self,
        taskbar_width: f64,
        occupied: &[OccupiedRange],
        gap: f64,
    ) -> Option<(f64, usize)> {
        (0..=self.widths.len()).rev().find_map(|shown| {
            LeftPlacement::new(self.width(shown), gap, self.spacing)
                .position(taskbar_width, occupied)
                .map(|left| (left, shown))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_preserves_widgets_and_centered_buttons() {
        let layout = LeftPlacement::new(260.0, 12.0, 6.0);
        let ranges = [
            OccupiedRange {
                left: 750.0,
                right: 1200.0,
                snug: false,
            },
            OccupiedRange {
                left: 0.0,
                right: 160.0,
                snug: false,
            },
        ];
        assert_eq!(layout.position(1920.0, &ranges), Some(172.0));
        // The weather is followed at the spacing between tiles.
        let weather = [OccupiedRange {
            left: 6.0,
            right: 158.0,
            snug: true,
        }];
        assert_eq!(layout.position(1920.0, &weather), Some(164.0));
        assert_eq!(layout.position(1920.0, &[]), Some(12.0));
    }

    #[test]
    fn layout_hides_when_no_safe_space_remains_on_left() {
        let layout = LeftPlacement::new(260.0, 12.0, 6.0);
        let occupied = |left, right| {
            [OccupiedRange {
                left,
                right,
                snug: false,
            }]
        };
        assert_eq!(layout.position(800.0, &occupied(0.0, 300.0)), None);
        assert_eq!(layout.position(f64::NAN, &[]), None);
        assert_eq!(layout.position(1920.0, &occupied(f64::NAN, 300.0)), None);
        assert_eq!(layout.position(1920.0, &occupied(0.0, 900.0)), None);
    }

    #[test]
    fn tiles_that_do_not_fit_give_way_to_the_marker() {
        let strip = TileStrip {
            widths: vec![100.0, 100.0, 80.0],
            spacing: 6.0,
            marker: 24.0,
            minimum: 260.0,
        };
        assert_eq!(strip.width(3), 292.0);
        assert_eq!(strip.width(2), 206.0 + 6.0 + 24.0);
        assert_eq!(strip.width(0), 24.0);
        // Buttons from 300: 12 + width + 12 must stay within both 300 and half the bar.
        let buttons = [OccupiedRange {
            left: 300.0,
            right: 900.0,
            snug: false,
        }];
        assert_eq!(strip.place(1920.0, &buttons, 12.0), Some((12.0, 2)));
        assert_eq!(strip.place(1920.0, &[], 12.0), Some((12.0, 3)));
        let crowded = [OccupiedRange {
            left: 40.0,
            right: 940.0,
            snug: false,
        }];
        assert_eq!(strip.place(1920.0, &crowded, 12.0), None);
        let marker_only = [OccupiedRange {
            left: 60.0,
            right: 900.0,
            snug: false,
        }];
        assert_eq!(strip.place(1920.0, &marker_only, 12.0), Some((12.0, 0)));
    }
}
