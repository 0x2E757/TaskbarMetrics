use super::history::SPAN;
use crate::metrics::MetricValue;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

/// Values of a 0–100 line snapped to whole pixel rows of the tile chart, so a 1 px
/// line stays sharp instead of spreading over two rows. A value that wavers by one
/// row around a row boundary would make the line step back and forth: while the
/// last 3 s stay within two neighbouring rows, the line keeps its row if it is one
/// of them and takes the row of their mean otherwise. The mean alone still steps
/// when the samples split unevenly between the rows.
#[derive(Clone, Default)]
pub struct PixelRows {
    recent: VecDeque<(Instant, f64)>,
    /// Row of the last value drawn.
    shown: Option<f64>,
}
impl PixelRows {
    const WINDOW: Duration = Duration::from_secs(3);
    /// Row of `value` from the bottom: 0 to `SPAN`.
    fn row(value: f64) -> f64 {
        (value.clamp(0.0, 100.0) / 100.0 * SPAN).round()
    }
    /// The value to draw for a sample taken at `now`; gaps stay gaps.
    pub fn snap(&mut self, value: &MetricValue, now: Instant) -> MetricValue {
        let value = match *value {
            MetricValue::Available(value) if value.is_finite() => value,
            ref missing => {
                self.recent.clear();
                self.shown = None;
                return match missing {
                    MetricValue::Available(_) => MetricValue::Unavailable,
                    other => other.clone(),
                };
            }
        };
        while self
            .recent
            .front()
            .is_some_and(|(at, _)| now.duration_since(*at) >= Self::WINDOW)
        {
            self.recent.pop_front();
        }
        self.recent.push_back((now, value));
        let rows = self.recent.iter().map(|(_, v)| Self::row(*v));
        let (low, high) = rows.fold((f64::MAX, f64::MIN), |(l, h), r| (l.min(r), h.max(r)));
        let row = match self.shown {
            _ if high - low > 1.0 => Self::row(value),
            Some(shown) if (low..=high).contains(&shown) => shown,
            _ => Self::row(
                self.recent.iter().map(|(_, v)| v).sum::<f64>() / self.recent.len() as f64,
            ),
        };
        self.shown = Some(row);
        MetricValue::Available(row / SPAN * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::history::MetricHistory;
    /// Draws `values` taken 500 ms apart, from `clock` on.
    fn drawn(rows: &mut PixelRows, clock: &mut Instant, values: &[f64]) -> Vec<f32> {
        let mut history = MetricHistory::spanning(100.0);
        for value in values {
            *clock += Duration::from_millis(500);
            let at = *clock;
            history.push(&rows.snap(&MetricValue::Available(*value), at));
        }
        history
            .points_range(0.0, 100.0)
            .iter()
            .map(|p| p.y)
            .collect()
    }
    #[test]
    fn lines_sit_on_pixel_centres() {
        let ys = drawn(
            &mut PixelRows::default(),
            &mut Instant::now(),
            &[0.0, 12.3, 49.9, 77.7, 100.0, 140.0],
        );
        for y in ys {
            assert!((y - 0.5).fract().abs() < 1e-4, "{y}");
        }
    }
    #[test]
    fn wavering_by_a_row_averages_and_a_real_change_shows_at_once() {
        // A row is 100/33 ≈ 3.03 °C: 61.5 °C and 62.5 °C fall into rows 20 and 21.
        let mut rows = PixelRows::default();
        let mut clock = Instant::now();
        let wavering = [61.5, 62.5, 62.5, 61.5, 62.5, 61.5, 61.5, 62.5, 62.5, 62.5];
        let ys = drawn(&mut rows, &mut clock, &wavering);
        assert!(ys.iter().all(|y| *y == ys[0]), "{ys:?}");
        // Staying in the next row for 3 s moves the line there.
        let ys = drawn(&mut rows, &mut clock, &[62.5; 7]);
        assert_eq!(ys[6], 34.5 - 21.0);
        // A jump of several rows is drawn at once.
        let ys = drawn(&mut rows, &mut clock, &[61.5, 61.5, 75.0]);
        assert_eq!(ys[2], 34.5 - 25.0);
    }
    #[test]
    fn gaps_restart_the_window() {
        let mut rows = PixelRows::default();
        let now = Instant::now();
        rows.snap(&MetricValue::Available(90.0), now);
        assert_eq!(
            rows.snap(&MetricValue::Unavailable, now),
            MetricValue::Unavailable
        );
        assert_eq!(
            rows.snap(&MetricValue::Available(61.0), now),
            MetricValue::Available(20.0 / SPAN * 100.0)
        );
    }
}
