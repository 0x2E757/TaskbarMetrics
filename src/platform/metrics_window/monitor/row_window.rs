/// Regular table rows worth creating: those in the viewport plus a screen of margin
/// on each side. The others stand in as spacers of the same height, so scrolling
/// keeps its extent while XAML builds a few dozen rows instead of several hundred.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct RowWindow {
    pub start: usize,
    pub end: usize,
    count: usize,
    /// Offset of the first regular row inside the scrolled content.
    top: f64,
}
impl RowWindow {
    pub const ROW: f64 = 32.0;
    /// Rows of `count` around the viewport at `offset`; `top` is where they begin.
    pub fn around(count: usize, top: f64, offset: f64, viewport: f64) -> Self {
        Self::span(count, top, offset, viewport, viewport.max(Self::ROW * 10.0))
    }
    /// Head above the regular rows: list padding, then the pinned rows and their divider.
    pub fn top(pinned: usize) -> f64 {
        let head = if pinned > 0 {
            pinned as f64 * Self::ROW + 5.0
        } else {
            0.0
        };
        4.0 + head
    }
    /// Whether every row visible at `offset` has been created.
    pub fn shows(&self, offset: f64, viewport: f64) -> bool {
        let visible = Self::span(self.count, self.top, offset, viewport, 0.0);
        visible.start >= self.start && visible.end <= self.end
    }
    fn span(count: usize, top: f64, offset: f64, viewport: f64, margin: f64) -> Self {
        let row = |y: f64| ((y - top) / Self::ROW).max(0.0) as usize;
        let start = row(offset - margin).min(count);
        let end = (row(offset + viewport + margin) + 1).clamp(start, count);
        Self {
            start,
            end,
            count,
            top,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_rows_near_the_viewport_are_created() {
        let window = RowWindow::around(300, 100.0, 3200.0, 400.0);
        assert_eq!((window.start, window.end), (84, 122));
        assert!(window.shows(3200.0, 400.0) && window.shows(2900.0, 400.0));
        assert!(!window.shows(4000.0, 400.0));
        let short = RowWindow::around(10, 100.0, 0.0, 400.0);
        assert_eq!((short.start, short.end), (0, 10));
    }
}
