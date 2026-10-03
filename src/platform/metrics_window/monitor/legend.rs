use super::{design::Design, ui::Ui};
use crate::platform::abi::Result;

/// Hatched fill for «no samples» and «awaiting ETW» areas: 1.5 px strokes, 45° apart by `step`.
pub struct Hatch;

impl Hatch {
    pub fn path(x: f64, y: f64, width: f64, height: f64, color: &str, step: f64) -> String {
        let pitch = step * std::f64::consts::SQRT_2;
        let mut data = String::new();
        let mut offset = x - height;
        while offset < x + width {
            data.push_str(&format!(
                "M {:.2},{:.2} L {:.2},{:.2} ",
                offset,
                y + height,
                offset + height,
                y
            ));
            offset += pitch;
        }
        format!(
            r#"<Path Data="{data}" Stroke="{color}" StrokeThickness="1.5"><Path.Clip><RectangleGeometry Rect="{x:.2},{y:.2},{:.2},{:.2}"/></Path.Clip></Path>"#,
            width.max(0.0),
            height.max(0.0)
        )
    }
}

pub enum LegendMark {
    /// Total or temperature line, 16×2.
    Line { color: String, dashed: bool },
    /// Pinned process area, 14×10 with a 2 px top edge.
    Area { color: String },
    /// Pinned process without data at the moment: dashed outline.
    Missing { color: String },
    /// Moment marker, 2×12.
    Moment,
    /// Missing samples or awaiting ETW attribution.
    Hatch,
}

pub struct LegendEntry {
    pub mark: LegendMark,
    pub label: String,
}

/// Wrapping legend row; shape of the marker, not only color, tells series apart.
pub struct ChartLegend {
    pub design: Design,
}

impl ChartLegend {
    fn mark(&self, mark: &LegendMark) -> String {
        let d = self.design;
        match mark {
            LegendMark::Line { color, dashed } => format!(
                r#"<Line X1="0" X2="16" Y1="1" Y2="1" Width="16" Height="2" Stroke="{color}" StrokeThickness="2" VerticalAlignment="Center"{}/>"#,
                if *dashed {
                    r#" StrokeDashArray="3,2""#
                } else {
                    ""
                }
            ),
            LegendMark::Area { color } => Self::area(&Design::alpha(color, 0.3), color),
            LegendMark::Missing { color } => format!(
                r#"<Rectangle Width="14" Height="10" RadiusX="1.25" RadiusY="1.25" Stroke="{color}" StrokeThickness="1.5" StrokeDashArray="2,1.5" VerticalAlignment="Center"/>"#
            ),
            LegendMark::Moment => format!(
                r#"<Border Width="2" Height="12" Margin="6,0" Background="{}" VerticalAlignment="Center"/>"#,
                d.color("ink")
            ),
            LegendMark::Hatch => format!(
                r#"<Grid Width="14" Height="10" VerticalAlignment="Center"><Canvas>{}</Canvas><Border BorderBrush="{}" BorderThickness="1" CornerRadius="2"/></Grid>"#,
                Hatch::path(0.0, 0.0, 14.0, 10.0, d.color("hatch"), 3.5),
                d.color("border")
            ),
        }
    }

    fn area(fill: &str, edge: &str) -> String {
        format!(
            r#"<Grid Width="14" Height="10" VerticalAlignment="Center"><Border CornerRadius="2" Background="{fill}"/><Border Height="2" VerticalAlignment="Top" CornerRadius="2,2,0,0" Background="{edge}"/></Grid>"#
        )
    }

    /// One self-contained legend item; loadable alone so its width can be measured.
    pub fn item(&self, entry: &LegendEntry) -> String {
        format!(
            r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Orientation="Horizontal" Spacing="6" VerticalAlignment="Center">{}<TextBlock Text="{}" FontSize="12" Foreground="{}" VerticalAlignment="Center"/></StackPanel>"#,
            self.mark(&entry.mark),
            Ui::xml(&entry.label),
            self.design.color("text2")
        )
    }

    /// Rows of `items` as laid out by `flow`.
    pub fn markup(items: &[String], flow: &LegendFlow) -> String {
        let mut markup = format!(
            r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Spacing="{}">"#,
            LegendFlow::ROW_GAP
        );
        for row in &flow.rows {
            markup.push_str(&format!(
                r#"<StackPanel Orientation="Horizontal" Spacing="{}" Height="{}">"#,
                LegendFlow::GAP,
                LegendFlow::LINE
            ));
            for item in row {
                markup.push_str(&items[*item]);
            }
            markup.push_str("</StackPanel>");
        }
        markup.push_str("</StackPanel>");
        markup
    }
}

/// Measured widths of legend items by their markup: the same item lays out the same.
#[derive(Default)]
pub struct LegendWidths(std::cell::RefCell<std::collections::HashMap<String, f64>>);

impl LegendWidths {
    /// Hovered process names add items; the oldest measurements are dropped together.
    const LIMIT: usize = 256;

    pub fn width(&self, item: &str, measure: impl FnOnce() -> Result<f64>) -> Result<f64> {
        if let Some(width) = self.0.borrow().get(item) {
            return Ok(*width);
        }
        let width = measure()?;
        let mut widths = self.0.borrow_mut();
        if widths.len() >= Self::LIMIT {
            widths.clear();
        }
        widths.insert(item.to_owned(), width);
        Ok(width)
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }
}

/// CSS `flex-wrap` of the legend row: items flow with 16 px gaps over the whole
/// width of the chart card and wrap to the next row when they do not fit.
#[derive(Debug, PartialEq)]
pub struct LegendFlow {
    pub rows: Vec<Vec<usize>>,
}

impl LegendFlow {
    pub const GAP: f64 = 16.0;
    pub const ROW_GAP: f64 = 6.0;
    /// Row height of 12 px text.
    pub const LINE: f64 = 16.0;

    pub fn new(widths: &[f64], available: f64) -> Self {
        let mut rows: Vec<Vec<usize>> = Vec::new();
        let mut used = 0.0;
        for (index, width) in widths.iter().enumerate() {
            match rows.last_mut() {
                Some(row) if used + Self::GAP + width <= available => {
                    row.push(index);
                    used += Self::GAP + width;
                }
                _ => {
                    rows.push(vec![index]);
                    used = *width;
                }
            }
        }
        Self { rows }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hatch_is_clipped_and_legend_marks_differ_by_shape() {
        let hatch = Hatch::path(10.0, 20.0, 30.0, 40.0, "#000000", 6.0);
        assert!(hatch.contains(r#"Rect="10.00,20.00,30.00,40.00""#));
        let legend = ChartLegend {
            design: Design { dark: true },
        };
        let items = [
            legend.item(&LegendEntry {
                mark: LegendMark::Line {
                    color: "#4CC2FF".into(),
                    dashed: true,
                },
                label: "A & B".into(),
            }),
            legend.item(&LegendEntry {
                mark: LegendMark::Missing {
                    color: "#FF9E50".into(),
                },
                label: "x".into(),
            }),
        ];
        let markup = ChartLegend::markup(&items, &LegendFlow::new(&[100.0, 50.0], 400.0));
        assert!(markup.contains("StrokeDashArray") && markup.contains("A &amp; B"));
        assert_eq!(markup.matches(r#"Height="16""#).count(), 1);
    }

    #[test]
    fn items_wrap_only_when_the_row_is_full() {
        let flow = LegendFlow::new(&[300.0, 300.0, 300.0], 950.0);
        assert_eq!(flow.rows, vec![vec![0, 1, 2]]);
        let flow = LegendFlow::new(&[300.0, 300.0, 300.0], 700.0);
        assert_eq!(flow.rows, vec![vec![0, 1], vec![2]]);
    }
}
