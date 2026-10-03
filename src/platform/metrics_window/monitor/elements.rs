use super::super::*;
use super::ui::Ui;

/// Named elements the window reads on every frame, found once per loaded markup.
pub struct Elements {
    pub chart: Com,
    pub timeline: Com,
    pub search: Com,
    pub table_card: Com,
    pub rows_scroll: Com,
    /// Hosts redrawn while the pointer moves: the chart tooltip and the overlay.
    pub hover_host: Com,
    pub overlay_host: Com,
}

impl Elements {
    pub fn find(root: &Com) -> Result<Self> {
        Ok(Self {
            chart: Ui::find(root, "Chart")?,
            timeline: Ui::find(root, "Timeline")?,
            search: Ui::find(root, "Search")?,
            table_card: Ui::find(root, "TableCard")?,
            rows_scroll: Ui::find(root, "RowsScroll")?,
            hover_host: Ui::find(root, "HoverHost")?,
            overlay_host: Ui::find(root, "OverlayHost")?,
        })
    }
}
