use crate::metrics::MetricValue;
use std::collections::VecDeque;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ChartPoint {
    pub x: f32,
    pub y: f32,
}

/// Pixels between neighbouring samples of a tile chart.
pub const STEP: f32 = 2.0;
/// Height of the 36 px tile chart that values span: 1.5 px stay free at both edges.
pub const SPAN: f64 = 33.0;

/// Samples of a chart `width` px wide, one every `STEP` px with the newest at the
/// right edge. Missing samples break a curve instead of becoming zeroes.
#[derive(Clone)]
pub struct MetricHistory {
    values: VecDeque<Option<f64>>,
    width: f32,
}
impl MetricHistory {
    pub fn spanning(width: f32) -> Self {
        Self {
            values: VecDeque::new(),
            width: width.max(0.0),
        }
    }
    fn capacity(&self) -> usize {
        (self.width / STEP) as usize + 1
    }
    /// Continues `older`, keeping as many of its newest samples as fit.
    pub fn replay(&mut self, older: &Self) {
        for value in &older.values {
            self.push_value(*value);
        }
    }
    pub fn push(&mut self, value: &MetricValue) {
        self.push_value(match value {
            MetricValue::Available(n) if n.is_finite() && *n >= 0.0 => Some(*n),
            _ => None,
        });
    }
    fn push_value(&mut self, value: Option<f64>) {
        if self.values.len() == self.capacity() {
            self.values.pop_front();
        }
        self.values.push_back(value);
    }
    pub fn peak(&self) -> f64 {
        self.values.iter().flatten().copied().fold(0.0, f64::max)
    }
    /// Called after push: missing samples stay missing, never become zeroes.
    pub fn mean_last_two(&self, current: &MetricValue) -> MetricValue {
        if !matches!(current, MetricValue::Available(value) if value.is_finite() && *value >= 0.0) {
            return current.clone();
        }
        let mut recent = self.values.iter().rev();
        match (recent.next(), recent.next()) {
            (Some(Some(latest)), Some(Some(previous))) => {
                MetricValue::Available(latest / 2.0 + previous / 2.0)
            }
            _ => current.clone(),
        }
    }
    pub fn points(&self, maximum: f64) -> Vec<ChartPoint> {
        self.points_range(0.0, maximum)
    }
    pub fn points_range(&self, minimum: f64, maximum: f64) -> Vec<ChartPoint> {
        self.project(minimum, maximum, None)
    }
    pub fn points_with_hidden_zero(&self, maximum: f64) -> Vec<ChartPoint> {
        self.points_with_hidden_idle(maximum, 0.0, -0.1)
    }
    pub fn points_with_hidden_idle(
        &self,
        maximum: f64,
        threshold: f64,
        idle_normalized: f64,
    ) -> Vec<ChartPoint> {
        self.project(0.0, maximum, Some((threshold, idle_normalized)))
    }
    fn project(&self, minimum: f64, maximum: f64, idle: Option<(f64, f64)>) -> Vec<ChartPoint> {
        let start = self
            .values
            .iter()
            .rposition(Option::is_none)
            .map_or(0, |i| i + 1);
        let newest = self.values.len().saturating_sub(1);
        self.values
            .iter()
            .enumerate()
            .skip(start)
            .filter_map(|(i, v)| {
                v.map(|v| ChartPoint {
                    x: self.width - (newest - i) as f32 * STEP,
                    y: 1.5
                        + (1.0
                            - if let Some((_, normalized)) =
                                idle.filter(|(threshold, _)| v == 0.0 || v < *threshold)
                            {
                                normalized
                            } else {
                                ((v - minimum) / (maximum - minimum).max(0.001)).clamp(0.0, 1.0)
                            }) as f32
                            * SPAN as f32,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numbers_average_two_samples_without_averaging_graph_or_missing_values() {
        let mut h = MetricHistory::spanning(100.0);
        h.push(&MetricValue::Available(20.0));
        assert!(matches!(
            h.mean_last_two(&MetricValue::Available(20.0)),
            MetricValue::Available(20.0)
        ));
        h.push(&MetricValue::Available(40.0));
        assert!(matches!(
            h.mean_last_two(&MetricValue::Available(40.0)),
            MetricValue::Available(30.0)
        ));
        assert_eq!(h.peak(), 40.0);
        h.push(&MetricValue::Unavailable);
        assert!(matches!(
            h.mean_last_two(&MetricValue::Unavailable),
            MetricValue::Unavailable
        ));
        h.push(&MetricValue::Available(60.0));
        assert!(matches!(
            h.mean_last_two(&MetricValue::Available(60.0)),
            MetricValue::Available(60.0)
        ));
    }
    #[test]
    fn throughput_scale_has_one_mb_floor_and_tracks_visible_peak() {
        let mut history = MetricHistory::spanning(100.0);
        history.push(&MetricValue::Available(0.2));
        assert_eq!(history.peak().max(1.0), 1.0);
        let y = history.points(history.peak().max(1.0))[0].y;
        assert!((y - 27.9).abs() < 0.001);
        history.push(&MetricValue::Available(5.0));
        assert_eq!(history.peak().max(1.0), 5.0);
        assert_eq!(
            history.points(history.peak().max(1.0)).last().unwrap().y,
            1.5
        );
        for _ in 0..51 {
            history.push(&MetricValue::Available(0.2));
        }
        assert_eq!(history.peak().max(1.0), 1.0);
    }
    #[test]
    fn disk_idle_threshold_maps_only_values_below_half_a_percent_to_zero() {
        let mut history = MetricHistory::spanning(100.0);
        for value in [0.0, 0.38, 0.499, 0.5, 1.0] {
            history.push(&MetricValue::Available(value));
        }
        let points = history.points_with_hidden_idle(100.0, 0.5, 0.0);
        let normal = history.points(100.0);
        for point in &points[..3] {
            assert_eq!(point.y, 34.5);
        }
        for i in 3..5 {
            assert_eq!(points[i].y, normal[i].y);
        }
    }
    #[test]
    fn hidden_zero_does_not_move_nonzero_samples_or_change_history() {
        let mut history = MetricHistory::spanning(100.0);
        for value in [0.0, 0.01, 50.0, 100.0] {
            history.push(&MetricValue::Available(value));
        }
        let regular = history.points(100.0);
        let hidden = history.points_with_hidden_zero(100.0);
        assert!((hidden[0].y - 37.8).abs() < 0.001);
        for i in 1..4 {
            assert_eq!(regular[i].y, hidden[i].y);
        }
        assert_eq!(history.peak(), 100.0);
        history.push(&MetricValue::Unavailable);
        assert!(history.points_with_hidden_zero(100.0).is_empty());
    }
    #[test]
    fn temperature_chart_uses_fixed_range_and_keeps_gaps() {
        let mut history = MetricHistory::spanning(104.0);
        for value in [20.0, 65.0, 110.0] {
            history.push(&MetricValue::Available(value));
        }
        let points = history.points_range(30.0, 100.0);
        assert_eq!(points[0].y, 34.5);
        assert_eq!(points[1].y, 18.0);
        assert_eq!(points[2].y, 1.5);
        history.push(&MetricValue::Unavailable);
        assert!(history.points_range(30.0, 100.0).is_empty());
    }
    #[test]
    fn half_of_the_scale_sits_mid_height() {
        let mut history = MetricHistory::spanning(100.0);
        history.push(&MetricValue::Available(50.0));
        // The 36 px chart keeps 1.5 px at both edges: 1.5..34.5, centre 18.
        assert_eq!(history.points_range(0.0, 100.0)[0].y, 18.0);
        assert_eq!(history.points(100.0)[0].y, 18.0);
    }
    #[test]
    fn samples_sit_two_pixels_apart_from_the_right_edge() {
        let mut history = MetricHistory::spanning(104.0);
        history.push(&MetricValue::Available(50.0));
        let points = history.points(100.0);
        assert_eq!(points[0].x, 104.0);
        assert_eq!(points[0].y, 18.0);
        history.push(&MetricValue::Unavailable);
        assert!(history.points(100.0).is_empty());
        for _ in 0..60 {
            history.push(&MetricValue::Available(100.0));
        }
        let points = history.points(100.0);
        // 104 px hold 53 samples: 0, 2, …, 104.
        assert_eq!(points.len(), 53);
        assert_eq!(points[0].x, 0.0);
        assert_eq!(points[1].x, 2.0);
        assert_eq!(points[52].x, 104.0);
        assert_eq!(points[52].y, 1.5);
    }
    #[test]
    fn a_resized_chart_keeps_the_newest_samples() {
        let mut wide = MetricHistory::spanning(100.0);
        for value in 0..51 {
            wide.push(&MetricValue::Available(value as f64));
        }
        let mut narrow = MetricHistory::spanning(10.0);
        narrow.replay(&wide);
        assert_eq!(narrow.points(100.0).len(), 6);
        assert_eq!(narrow.peak(), 50.0);
        assert_eq!(narrow.points(100.0)[0].x, 0.0);
    }
}
