use super::history::ChartPoint;

/// Keeps the dashes of a scrolling line on its samples. A dash pattern is counted
/// along the line from its first point; when the oldest sample leaves the chart the
/// first point changes, and without an offset every dash would jump to another
/// place on the curve.
///
/// The line runs as a staircase (`staircase`) between samples on whole pixel rows:
/// a diagonal step is √5 px long and would put every dash after it between pixels.
#[derive(Clone, Default)]
pub struct DashPhase {
    /// Samples recorded so far.
    pushed: u64,
    /// Number of the sample the line started with at the last `offset`.
    first: u64,
    points: Vec<ChartPoint>,
    offset: f64,
}

impl DashPhase {
    /// Counts a sample recorded into the line's history, gaps included.
    pub fn push(&mut self) {
        self.pushed += 1;
    }

    /// The dash offset, in px, for the staircase through `points`: the newest
    /// samples, ending with the last one pushed. Grows by the length of the line
    /// that left the chart since the last call, modulo the pattern's `period`.
    pub fn offset(&mut self, points: &[ChartPoint], period: f64) -> f64 {
        let first = self.pushed.saturating_sub(points.len() as u64);
        let left = first.wrapping_sub(self.first) as usize;
        if first < self.first || left >= self.points.len() {
            // A new line after a gap: its pattern starts at its first point.
            self.offset = 0.0;
        } else {
            for pair in self.points.windows(2).take(left) {
                self.offset += Self::step(pair[0], pair[1]);
            }
        }
        self.offset %= period.max(f64::EPSILON);
        self.first = first;
        self.points = points.to_vec();
        self.offset
    }

    /// Length of the staircase between two samples.
    fn step(from: ChartPoint, to: ChartPoint) -> f64 {
        f64::from((to.x - from.x).abs() + (to.y - from.y).abs())
    }

    /// The line through `points` as horizontal and vertical runs: half the way
    /// across, the change of row, the rest of the way.
    pub fn staircase(points: &[ChartPoint]) -> Vec<ChartPoint> {
        let mut result = Vec::with_capacity(points.len() * 3);
        for (index, point) in points.iter().enumerate() {
            if let Some(previous) = index.checked_sub(1).map(|i| points[i]) {
                if previous.y != point.y {
                    let middle = (previous.x + point.x) / 2.0;
                    result.push(ChartPoint {
                        x: middle,
                        y: previous.y,
                    });
                    result.push(ChartPoint {
                        x: middle,
                        y: point.y,
                    });
                }
            }
            result.push(*point);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{metrics::MetricValue, presentation::history::MetricHistory};

    /// Pattern position of each sample: its distance along the staircase plus the
    /// offset.
    fn phases(history: &MetricHistory, dash: &mut DashPhase) -> Vec<f64> {
        let points = history.points(33.0);
        let offset = dash.offset(&points, 5.0);
        let mut along = 0.0;
        let mut result = vec![offset % 5.0];
        for pair in points.windows(2) {
            along += DashPhase::step(pair[0], pair[1]);
            result.push((along + offset) % 5.0);
        }
        result
    }

    #[test]
    fn dashes_stay_on_their_samples_as_the_chart_scrolls() {
        // The chart holds six samples: from the seventh on, each pushes the oldest out.
        // Values are whole rows of the 0–33 scale, as the temperature line draws them.
        let mut history = MetricHistory::spanning(10.0);
        let mut dash = DashPhase::default();
        let mut previous: Vec<f64> = Vec::new();
        for value in [10.0, 11.0, 11.0, 14.0, 13.0, 13.0, 20.0, 10.0, 12.0, 12.0] {
            history.push(&MetricValue::Available(value));
            dash.push();
            let current = phases(&history, &mut dash);
            // Samples that stayed keep their place in the pattern, on whole pixels.
            let shift = previous.len() + 1 - current.len();
            for (old, new) in previous.iter().skip(shift).zip(&current) {
                assert!((old - new).abs() < 1e-3, "{previous:?} → {current:?}");
            }
            assert!(current.iter().all(|p| (p - p.round()).abs() < 1e-3));
            previous = current;
        }
    }

    #[test]
    fn steps_change_rows_halfway_with_whole_pixel_runs() {
        let points = [
            ChartPoint { x: 0.0, y: 10.5 },
            ChartPoint { x: 2.0, y: 10.5 },
            ChartPoint { x: 4.0, y: 13.5 },
        ];
        let stairs = DashPhase::staircase(&points);
        let xy: Vec<_> = stairs.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(
            xy,
            [
                (0.0, 10.5),
                (2.0, 10.5),
                (3.0, 10.5),
                (3.0, 13.5),
                (4.0, 13.5)
            ]
        );
    }

    #[test]
    fn a_gap_starts_a_new_pattern() {
        let mut history = MetricHistory::spanning(10.0);
        let mut dash = DashPhase::default();
        for value in [10.0, 80.0, 10.0, 80.0, 10.0, 80.0, 10.0] {
            history.push(&MetricValue::Available(value));
            dash.push();
            dash.offset(&history.points(100.0), 5.0);
        }
        history.push(&MetricValue::Unavailable);
        dash.push();
        assert_eq!(dash.offset(&history.points(100.0), 5.0), 0.0);
        history.push(&MetricValue::Available(50.0));
        dash.push();
        assert_eq!(dash.offset(&history.points(100.0), 5.0), 0.0);
    }
}
