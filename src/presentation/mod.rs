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
}
