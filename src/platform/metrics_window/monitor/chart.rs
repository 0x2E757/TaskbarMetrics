use super::{
    design::Design,
    groups::NameGroups,
    legend::Hatch,
    locale::Language,
    model::{Device, ProcessKey, Resource, Timeline},
    pins::PinnedLayer,
    ui::Ui,
};

use crate::platform::process_history::{
    memory::MemoryRows,
    store::{Frame, RETENTION},
};

use std::sync::Arc;

/// Samples on the time axis: always the last 300 s at 0.5 s.
pub const WINDOW: u64 = RETENTION;
/// Live DISK/NET process values wait this many samples for ETW delivery.
pub const ETW_DELAY: u64 = 4;

/// The series a hovered table row stands out with.
#[derive(Clone, Copy)]
pub enum Highlight<'a> {
    Process(ProcessKey),
    /// A grouped row: every process of the name at each moment, but the pinned
    /// ones, which have rows and series of their own.
    Name(&'a str),
}

impl Highlight<'_> {
    pub fn series(
        self,
        frames: &[Arc<Frame>],
        device: &Device,
        pinned: &[PinnedLayer],
    ) -> Vec<[Option<f64>; 2]> {
        match self {
            Self::Process(key) => ChartRenderer::series_of(frames, device, key),
            Self::Name(name) => frames
                .iter()
                .map(|f| {
                    Device::sum(
                        f.processes
                            .iter()
                            .filter(|p| {
                                NameGroups::same(&p.identity.name, name)
                                    && !pinned.iter().any(|layer| layer.key == Timeline::key(p))
                            })
                            .map(|p| device.plotted(f, p)),
                    )
                })
                .collect(),
        }
    }
}

/// Chart canvas size and paddings from `tokens.json → chart.pad`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ChartLayout {
    pub width: f64,
    pub height: f64,
    left: f64,
    right: f64,
    temperature: bool,
    /// Physical pixels per XAML pixel.
    scale: f64,
}

impl ChartLayout {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width: width.max(32.0),
            height: height.max(64.0),
            left: 46.0,
            right: 44.0,
            temperature: true,
            scale: 1.0,
        }
    }

    /// Physical pixels per XAML pixel, for the lines kept on whole pixels.
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale.max(1.0);
        self
    }

    /// `x` moved to the middle of the physical pixel column it falls in.
    pub fn pixel(self, x: f64) -> f64 {
        ((x * self.scale).floor() + 0.5) / self.scale
    }

    /// Canvas height, one for every page: 270 (the mirrored DISK/NET charts of the
    /// artboards), 210 in compact windows.
    pub fn height(compact: bool) -> f64 {
        if compact {
            210.0
        } else {
            270.0
        }
    }

    /// The artboards keep the 44 px right margin even without a temperature axis.
    pub fn for_resource(mut self, resource: Resource) -> Self {
        self.temperature = resource.temperature_axis();
        self
    }

    /// `padCompact` for windows narrower than 1100.
    pub fn compact(mut self, compact: bool) -> Self {
        if compact {
            self.left = 40.0;
            self.right = 36.0;
        }
        self
    }

    pub fn x0(self) -> f64 {
        self.left
    }

    pub fn x1(self) -> f64 {
        (self.width - self.right).max(self.left + 1.0)
    }

    pub fn y0(self) -> f64 {
        28.0
    }

    pub fn y1(self) -> f64 {
        self.height - 22.0
    }

    fn plot_width(self) -> f64 {
        self.x1() - self.x0()
    }

    pub fn fraction(self, x: f64) -> f64 {
        ((x - self.x0()) / self.plot_width()).clamp(0.0, 1.0)
    }

    /// Bucket under the horizontal position `x` for a window ending at `end`.
    pub fn bucket(self, x: f64, end: u64) -> f64 {
        end.saturating_sub(WINDOW - 1) as f64 + self.fraction(x) * WINDOW as f64 - 0.5
    }
}

/// Upper bound of the value axis: fixed 100 % or the 1-2-5 step above the peak.
pub struct ChartScale;

impl ChartScale {
    pub fn maximum<'a>(
        resource: Resource,
        values: impl Iterator<Item = &'a [Option<f64>; 2]>,
    ) -> f64 {
        if resource.percent() {
            return 100.0;
        }
        let peak = values.flatten().flatten().copied().fold(0.0, f64::max);
        if resource == Resource::Ram {
            // Whole 4 GB: each of the four grid lines then falls on a whole gigabyte.
            return ((peak / Self::RAM_STEP).ceil() * Self::RAM_STEP).max(resource.floor());
        }
        Self::step(peak).max(resource.floor())
    }

    const RAM_STEP: f64 = 4096.0;

    fn step(value: f64) -> f64 {
        let magnitude = 10f64.powf(value.max(1.0).log10().floor());
        [1.0, 2.0, 5.0, 10.0]
            .into_iter()
            .map(|v| v * magnitude)
            .find(|v| *v >= value)
            .unwrap_or(magnitude * 10.0)
    }
}

/// Each continuous run closes to the baseline independently; missing samples
/// and gaps in the timeline must never become interpolated activity.
struct SeriesGeometry {
    line: String,
    area: String,
}

impl SeriesGeometry {
    /// A line command takes any number of points, so `L` is written once per run:
    /// the chart markup is parsed on every update and shorter data parses faster.
    fn new(samples: impl IntoIterator<Item = (u64, f64, Option<f64>)>, baseline: f64) -> Self {
        use std::fmt::Write;
        let mut result = Self {
            line: String::new(),
            area: String::new(),
        };
        // Bucket and x of the previous point of the current run, and whether the
        // line already continues with `L`.
        let mut previous: Option<(u64, f64, bool)> = None;
        for (bucket, x, y) in samples {
            if y.is_none() || previous.is_some_and(|(b, _, _)| bucket != b + 1) {
                if let Some((_, last_x, _)) = previous.take() {
                    let _ = write!(result.area, "{last_x:.2},{baseline:.2} Z ");
                }
            }
            if let Some(y) = y {
                match previous {
                    None => {
                        let _ = write!(result.line, "M {x:.2},{y:.2} ");
                        let _ = write!(result.area, "M {x:.2},{baseline:.2} L {x:.2},{y:.2} ");
                    }
                    Some((_, _, continued)) => {
                        if !continued {
                            result.line.push_str("L ");
                        }
                        let _ = write!(result.line, "{x:.2},{y:.2} ");
                        let _ = write!(result.area, "{x:.2},{y:.2} ");
                    }
                }
                previous = Some((bucket, x, previous.is_some()));
            }
        }
        if let Some((_, x, _)) = previous {
            let _ = write!(result.area, "{x:.2},{baseline:.2} Z ");
        }
        result
    }
}

/// A dashed line whose dashes belong to moments, not to the path: a dash covers the
/// same span of time in every redraw, so the dashes move with the samples once the
/// oldest ones leave the chart. `StrokeDashArray` counts from the first point of the
/// path and would restart at the left edge on each redraw of a full window.
struct DashedLine;

impl DashedLine {
    /// 4 px dashes, 3 px gaps, as `StrokeDashArray="3.2,2.4"` with a 1.25 px stroke.
    const DASH: f64 = 4.0;
    const PERIOD: f64 = 7.0;

    /// Path data for `samples` (bucket, x, y); `pixels` is the width of a bucket.
    /// Missing samples and gaps in the buckets break the line.
    fn path(samples: impl IntoIterator<Item = (u64, f64, Option<f64>)>, pixels: f64) -> String {
        use std::fmt::Write;
        let mut path = String::new();
        // Where a dash is open: the points it has so far.
        let mut dash: Vec<(f64, f64)> = Vec::new();
        let mut close = |dash: &mut Vec<(f64, f64)>| {
            if dash.len() > 1 {
                for (index, (x, y)) in dash.iter().enumerate() {
                    let command = match index {
                        0 => "M ",
                        1 => "L ",
                        _ => "",
                    };
                    let _ = write!(path, "{command}{x:.2},{y:.2} ");
                }
            }
            dash.clear();
        };
        // The previous point of the current run: bucket, absolute position, x, y.
        let mut previous: Option<(u64, f64, f64, f64)> = None;
        for (bucket, x, y) in samples {
            let Some(y) = y else {
                close(&mut dash);
                previous = None;
                continue;
            };
            // The position along time that the pattern is anchored to.
            let at = bucket as f64 * pixels;
            match previous {
                Some((last, from, x0, y0)) if last + 1 == bucket => {
                    // Every dash edge between the two samples splits the segment.
                    let mut edge = (from / Self::PERIOD).floor() * Self::PERIOD;
                    loop {
                        for boundary in [edge, edge + Self::DASH] {
                            if boundary > from && boundary < at {
                                let t = (boundary - from) / (at - from);
                                // A dash opens in a gap and closes in a dash.
                                dash.push((x0 + (x - x0) * t, y0 + (y - y0) * t));
                                if dash.len() > 1 {
                                    close(&mut dash);
                                }
                            }
                        }
                        edge += Self::PERIOD;
                        if edge >= at {
                            break;
                        }
                    }
                    if Self::on(at) {
                        dash.push((x, y));
                    } else if !dash.is_empty() {
                        // A dash that ends right on this sample.
                        dash.push((x, y));
                        close(&mut dash);
                    }
                }
                _ => {
                    close(&mut dash);
                    if Self::on(at) {
                        dash.push((x, y));
                    }
                }
            }
            previous = Some((bucket, at, x, y));
        }
        close(&mut dash);
        path
    }

    /// Whether the position falls into a dash.
    fn on(at: f64) -> bool {
        at.rem_euclid(Self::PERIOD) < Self::DASH
    }
}

/// Centered moving average within each continuous run. Sensors report whole degrees,
/// so a steady temperature flips between neighbours every sample; averaging keeps
/// the dashed line readable while real changes still show as ramps.
struct Smoothing;

impl Smoothing {
    /// Samples on each side: 5 samples, 2.5 s in all.
    const RADIUS: usize = 2;

    fn temperatures(frames: &[Arc<Frame>], device: &Device) -> Vec<Option<f64>> {
        Self::apply(
            &frames
                .iter()
                .map(|f| (f.bucket, device.temperature(f)))
                .collect::<Vec<_>>(),
        )
    }

    fn apply(samples: &[(u64, Option<f64>)]) -> Vec<Option<f64>> {
        let joined = |a: usize, b: usize| {
            samples[a].1.is_some() && samples[b].1.is_some() && samples[a].0 + 1 == samples[b].0
        };
        (0..samples.len())
            .map(|i| {
                samples[i].1?;
                let mut first = i;
                while first > 0 && i - first < Self::RADIUS && joined(first - 1, first) {
                    first -= 1;
                }
                let mut last = i;
                while last + 1 < samples.len() && last - i < Self::RADIUS && joined(last, last + 1)
                {
                    last += 1;
                }
                let run = &samples[first..=last];
                Some(run.iter().filter_map(|s| s.1).sum::<f64>() / run.len() as f64)
            })
            .collect()
    }
}

/// Coordinate transform of one rendered frame of the chart.
struct Plot {
    layout: ChartLayout,
    start: u64,
    maximum: f64,
    mirrored: bool,
}

impl Plot {
    fn x(&self, bucket: u64) -> f64 {
        self.layout.x0()
            + (bucket.saturating_sub(self.start) as f64 + 0.5) / WINDOW as f64
                * self.layout.plot_width()
    }

    fn half_sample(&self) -> f64 {
        self.layout.plot_width() / WINDOW as f64 / 2.0
    }

    fn baseline(&self) -> f64 {
        if self.mirrored {
            (self.layout.y0() + self.layout.y1()) / 2.0
        } else {
            self.layout.y1()
        }
    }

    fn y(&self, value: f64, direction: usize) -> f64 {
        let amplitude =
            (self.layout.y1() - self.layout.y0()) / if self.mirrored { 2.0 } else { 1.0 };
        self.baseline()
            + if direction == 1 { 1.0 } else { -1.0 } * value.clamp(0.0, self.maximum)
                / self.maximum
                * amplitude
    }

    fn temperature(&self, value: f64) -> f64 {
        self.layout.y1() - value.clamp(0.0, 100.0) / 100.0 * (self.layout.y1() - self.layout.y0())
    }
}

/// One drawing pass over a series: fills first, lines later.
#[derive(Clone, Copy)]
struct Pass {
    attributed: bool,
    live: bool,
    fill: bool,
}

/// Series style: color, fill alpha, line width and line opacity.
struct Style<'a> {
    color: &'a str,
    fill: f64,
    width: f64,
    opacity: f64,
}

pub struct ChartRenderer {
    pub design: Design,
    pub language: Language,
    /// Width of the total line; pinned and highlighted processes scale with it.
    pub line: f64,
}

impl ChartRenderer {
    fn text(
        &self,
        (left, top, width): (f64, f64, f64),
        align: &str,
        text: &str,
        color: &str,
        bold: bool,
    ) -> String {
        format!(
            r#"<TextBlock Canvas.Left="{left:.2}" Canvas.Top="{top:.2}" Width="{:.2}" TextAlignment="{align}" FontSize="11" Foreground="{color}"{} Typography.NumeralAlignment="Tabular" Text="{}"/>"#,
            width.max(0.0),
            if bold {
                r#" FontWeight="SemiBold""#
            } else {
                ""
            },
            Ui::xml(text)
        )
    }

    /// A chip centered on `center` (kept inside `[low, high]`); width is estimated from the text.
    fn chip(
        center: f64,
        top: f64,
        low: f64,
        high: f64,
        text_length: usize,
        extra: f64,
        content: String,
    ) -> String {
        let width = text_length as f64 * 6.3 + extra;
        let center = center.clamp(
            low + width / 2.0,
            (high - width / 2.0).max(low + width / 2.0),
        );
        format!(
            r#"<Grid Canvas.Left="{:.2}" Canvas.Top="{top:.2}" Width="400">{content}</Grid>"#,
            center - 200.0
        )
    }

    fn axis_value(&self, value: f64) -> String {
        let precision = if value.fract().abs() < 1e-9 {
            0
        } else if (value * 10.0).fract().abs() < 1e-9 {
            1
        } else {
            2
        };
        self.language.number(value, precision)
    }

    fn axes(&self, out: &mut String, plot: &Plot, resource: Resource) {
        let d = self.design;
        let l = plot.layout;
        // Gigabytes read better than five-digit megabytes on the memory axis.
        let (unit, divisor) = match resource.unit() {
            "%" => ("\u{A0}%".to_owned(), 1.0),
            _ if resource == Resource::Ram => (" GB".to_owned(), 1024.0),
            unit => (format!(" {unit}"), 1.0),
        };
        for row in 0..=4 {
            let y = l.y0() + (l.y1() - l.y0()) * row as f64 / 4.0;
            let zero = (y - plot.baseline()).abs() < 0.1;
            out.push_str(&format!(
                r#"<Line X1="{:.2}" X2="{:.2}" Y1="{y:.2}" Y2="{y:.2}" Stroke="{}" StrokeThickness="1"/>"#,
                l.x0(),
                l.x1(),
                d.color(if zero { "border" } else { "grid" })
            ));
            let fraction = if plot.mirrored {
                (1.0 - row as f64 / 2.0).abs()
            } else {
                1.0 - row as f64 / 4.0
            };
            let mut label = self.axis_value(fraction * plot.maximum / divisor);
            if row == 0 || (plot.mirrored && row == 4) {
                label.push_str(&unit);
            }
            // Wide labels such as «50 MB/s» may extend into the card padding, as in the artboards.
            out.push_str(&self.text(
                (l.x0() - 108.0, y - 8.0, 100.0),
                "Right",
                &label,
                d.color("text3"),
                false,
            ));
        }
        if l.temperature {
            for value in [0.0, 50.0, 100.0] {
                out.push_str(&self.text(
                    (
                        l.x1() + 8.0,
                        plot.temperature(value) - 8.0,
                        l.width - l.x1(),
                    ),
                    "Left",
                    &if value == 100.0 {
                        "100 °C".to_owned()
                    } else {
                        format!("{value:.0}")
                    },
                    d.color("temp"),
                    false,
                ));
            }
        }
    }

    /// A 1 px vertical line across the plot at `x`. The chart scrolls by 1.4 px a
    /// sample, so a line drawn at a fractional x alternates between one sharp and
    /// two smeared pixel columns; kept in the middle of a pixel column it stays
    /// sharp. Canvas children are not layout-rounded, so the x is snapped here.
    fn rule(x: f64, layout: &ChartLayout, color: &str, dashed: bool) -> String {
        let x = layout.pixel(x);
        format!(
            r#"<Line X1="{x:.3}" X2="{x:.3}" Y1="{:.2}" Y2="{:.2}" Stroke="{color}" StrokeThickness="1"{}/>"#,
            layout.y0(),
            layout.y1(),
            if dashed {
                r#" StrokeDashArray="3,3""#
            } else {
                ""
            }
        )
    }

    fn time_axis(&self, out: &mut String, plot: &Plot, end: u64, live: bool) {
        let d = self.design;
        let l = plot.layout;
        let mut labelled = f64::MIN;
        for bucket in (plot.start..=end).filter(|b| b % 120 == 0) {
            let x = plot.x(bucket) - plot.half_sample();
            out.push_str(&Self::rule(x, &l, d.color("grid"), false));
            // Narrow plots keep every other label so «HH:MM» never overlaps.
            if x - l.x0() >= 16.0 && l.x1() - x >= 56.0 && x - labelled >= 44.0 {
                labelled = x;
                out.push_str(&self.text(
                    (x - 30.0, l.y1() + 3.0, 60.0),
                    "Center",
                    &self.language.minute(bucket),
                    d.color("text3"),
                    false,
                ));
            }
        }
        let label = if live {
            self.language.text("now").to_owned()
        } else {
            self.language.clock(end)
        };
        out.push_str(&self.text(
            (l.x1() - 90.0, l.y1() + 3.0, 90.0),
            "Right",
            &label,
            d.color("text2"),
            true,
        ));
    }

    fn series(
        &self,
        out: &mut String,
        plot: &Plot,
        frames: &[Arc<Frame>],
        values: &[[Option<f64>; 2]],
        style: &Style,
        pass: Pass,
    ) {
        let Pass {
            attributed,
            live,
            fill,
        } = pass;
        let last = frames.last().map_or(0, |f| f.bucket);
        for direction in 0..if plot.mirrored { 2 } else { 1 } {
            let geometry = SeriesGeometry::new(
                frames.iter().zip(values).map(|(frame, value)| {
                    let pending =
                        attributed && live && plot.mirrored && frame.bucket + ETW_DELAY > last;
                    (
                        frame.bucket,
                        plot.x(frame.bucket),
                        if pending {
                            None
                        } else {
                            value[direction].map(|v| plot.y(v, direction))
                        },
                    )
                }),
                plot.baseline(),
            );
            if fill && !geometry.area.is_empty() {
                out.push_str(&format!(
                    r#"<Path Data="{}" Fill="{}" Opacity="{:.3}"/>"#,
                    geometry.area,
                    style.color,
                    style.fill * if direction == 1 { 0.85 } else { 1.0 }
                ));
            }
            if !fill && !geometry.line.is_empty() {
                out.push_str(&format!(
                    r#"<Path Data="{}" Stroke="{}" StrokeThickness="{}" StrokeLineJoin="Round" Opacity="{}"/>"#,
                    geometry.line, style.color, style.width, style.opacity
                ));
            }
        }
    }

    fn temperature(&self, out: &mut String, plot: &Plot, frames: &[Arc<Frame>], device: &Device) {
        let d = self.design;
        let color = d.color("temp");
        let line = DashedLine::path(
            frames
                .iter()
                .zip(Smoothing::temperatures(frames, device))
                .map(|(f, v)| (f.bucket, plot.x(f.bucket), v.map(|v| plot.temperature(v)))),
            plot.layout.plot_width() / WINDOW as f64,
        );
        // A lone sample draws no dash, but its ceiling band below still shows.
        if !line.is_empty() {
            out.push_str(&format!(
                r#"<Path Data="{line}" Stroke="{color}" StrokeThickness="1.25" StrokeLineJoin="Round"/>"#
            ));
        }
        // Runs above the 100 °C ceiling: a 3 px band on the top edge and one peak label.
        let mut runs: Vec<(u64, u64, f64)> = Vec::new();
        for frame in frames {
            match (
                device.temperature(frame).filter(|v| *v > 100.0),
                runs.last_mut(),
            ) {
                (Some(v), Some(run)) if run.1 + 1 == frame.bucket => {
                    run.1 = frame.bucket;
                    run.2 = run.2.max(v);
                }
                (Some(v), _) => runs.push((frame.bucket, frame.bucket, v)),
                _ => {}
            }
        }
        let y0 = plot.layout.y0();
        for (first, last, _) in &runs {
            out.push_str(&format!(
                r#"<Line X1="{:.2}" X2="{:.2}" Y1="{y0}" Y2="{y0}" Stroke="{color}" StrokeThickness="3" StrokeStartLineCap="Round" StrokeEndLineCap="Round"/>"#,
                plot.x(*first),
                plot.x(*last)
            ));
        }
        if let Some((first, last, peak)) = runs.iter().max_by(|a, b| a.2.total_cmp(&b.2)) {
            let text = self
                .language
                .text("above scale, peak {v}")
                .replace("{v}", &self.language.celsius(*peak));
            out.push_str(&Self::chip(
                (plot.x(*first) + plot.x(*last)) / 2.0,
                y0 + 6.0,
                plot.layout.x0(),
                plot.layout.x1(),
                text.chars().count(),
                27.0,
                format!(
                    r#"<Border HorizontalAlignment="Center" Background="{}" BorderBrush="{}" BorderThickness="1" CornerRadius="4" Padding="6,1"><StackPanel Orientation="Horizontal" Spacing="4">{}<TextBlock Text="{}" FontSize="11" FontWeight="SemiBold" Foreground="{color}"/></StackPanel></Border>"#,
                    d.color("card"),
                    Design::alpha(color, 0.5),
                    d.icon_sized("arrowup", color, 11.0, 1.6),
                    Ui::xml(&text)
                ),
            ));
        }
    }

    /// Spans without measurements of `device`: buckets without frames, and frames
    /// the recorder took while it was not recording this device.
    fn unmeasured(frames: &[Arc<Frame>], device: &Device) -> Vec<(u64, u64)> {
        let mut spans: Vec<(u64, u64)> = Vec::new();
        let mut push = |first: u64, last: u64| match spans.last_mut() {
            Some(span) if span.1 + 1 >= first => span.1 = span.1.max(last),
            _ => spans.push((first, last)),
        };
        for (index, frame) in frames.iter().enumerate() {
            if let Some(previous) = index.checked_sub(1).map(|i| frames[i].bucket) {
                if frame.bucket > previous + 1 {
                    push(previous + 1, frame.bucket - 1);
                }
            }
            if !device.recorded(frame) {
                push(frame.bucket, frame.bucket);
            }
        }
        spans
    }

    /// RAM: spans of frames with processes but without their shared memory. The
    /// recorder scans for shared pages only while the window is open.
    pub fn unshared(frames: &[Arc<Frame>]) -> Vec<(u64, u64)> {
        let mut spans: Vec<(u64, u64)> = Vec::new();
        for frame in frames {
            let mut processes = frame
                .processes
                .iter()
                .filter(|p| p.identity.pid != MemoryRows::PID)
                .peekable();
            if processes.peek().is_none() || processes.any(|p| p.shared().is_some()) {
                continue;
            }
            match spans.last_mut() {
                Some(span) if span.1 + 1 == frame.bucket => span.1 = frame.bucket,
                _ => spans.push((frame.bucket, frame.bucket)),
            }
        }
        spans
    }

    /// RAM: hatched spans without the shared memory of processes, dashed borders
    /// inside the plot and a chip over the wider spans.
    fn shared_gaps(&self, out: &mut String, plot: &Plot, frames: &[Arc<Frame>]) {
        let d = self.design;
        let l = plot.layout;
        let end = frames.last().map_or(0, |f| f.bucket);
        for (first, last) in Self::unshared(frames) {
            let left = (plot.x(first) - plot.half_sample()).max(l.x0());
            let right = (plot.x(last) + plot.half_sample()).min(l.x1());
            out.push_str(&Hatch::path(
                left,
                l.y0(),
                right - left,
                l.y1() - l.y0(),
                d.color("hatch"),
                5.0,
            ));
            // Borders inside the plot: where the measurement stopped and started again.
            for (x, inside) in [(left, left > l.x0()), (right, last < end)] {
                if inside {
                    out.push_str(&Self::rule(x, &l, d.color("text3"), true));
                }
            }
            let text = self.language.text("shared memory not measured");
            if right - left < text.chars().count() as f64 * 6.3 + 40.0 {
                continue;
            }
            out.push_str(&Self::chip(
                (left + right) / 2.0,
                l.y0() + 6.0,
                left,
                right,
                text.chars().count(),
                14.0,
                format!(
                    r#"<Border HorizontalAlignment="Center" Background="{}" BorderBrush="{}" BorderThickness="1" CornerRadius="4" Padding="6,1"><TextBlock Text="{}" FontSize="11" Foreground="{}"/></Border>"#,
                    d.color("card"),
                    d.color("border"),
                    Ui::xml(text),
                    d.color("text2")
                ),
            ));
        }
    }

    /// Interruptions of at least one second: hatched columns and a duration chip.
    fn gaps(&self, out: &mut String, plot: &Plot, frames: &[Arc<Frame>], device: &Device) {
        let d = self.design;
        let l = plot.layout;
        for (first, last) in Self::unmeasured(frames, device) {
            let missing = last - first + 1;
            if missing < 2 {
                continue;
            }
            let left = plot.x(first) - plot.half_sample();
            let right = plot.x(last) + plot.half_sample();
            out.push_str(&Hatch::path(
                left,
                l.y0(),
                right - left,
                l.y1() - l.y0(),
                d.color("hatch"),
                6.0,
            ));
            if missing >= 8 {
                let text = self
                    .language
                    .text("no samples · {d}")
                    .replace("{d}", &self.language.span(missing / 2));
                out.push_str(&Self::chip(
                    (left + right) / 2.0,
                    l.y1() - 26.0,
                    l.x0(),
                    l.x1(),
                    text.chars().count(),
                    14.0,
                    format!(
                        r#"<Border HorizontalAlignment="Center" Background="{}" BorderBrush="{}" BorderThickness="1" CornerRadius="4" Padding="6,1"><TextBlock Text="{}" FontSize="11" Foreground="{}"/></Border>"#,
                        d.color("card"),
                        d.color("border"),
                        Ui::xml(&text),
                        d.color("text2")
                    ),
                ));
            }
        }
    }

    /// Live DISK/NET: the newest samples still wait for ETW attribution.
    fn etw(&self, out: &mut String, plot: &Plot, last: u64) {
        let d = self.design;
        let l = plot.layout;
        let left =
            (plot.x(last.saturating_sub(ETW_DELAY - 1)) - plot.half_sample()).min(l.x1() - 4.0);
        out.push_str(&Hatch::path(
            left,
            l.y0(),
            l.x1() - left,
            l.y1() - l.y0(),
            d.color("hatch"),
            5.0,
        ));
        out.push_str(&Self::rule(left, &l, d.color("text3"), true));
        out.push_str(&format!(
            r#"<Grid Canvas.Left="{:.2}" Canvas.Top="{:.2}" Width="400"><Border HorizontalAlignment="Right" Background="{}" BorderBrush="{}" BorderThickness="1" CornerRadius="4" Padding="7,2"><StackPanel Orientation="Horizontal" Spacing="5">{}<TextBlock Text="{}" FontSize="11" Margin="0,-1,0,1" Foreground="{}"/></StackPanel></Border></Grid>"#,
            l.x1() - 10.0 - 400.0,
            l.y0() + 6.0,
            d.color("card"),
            d.color("border"),
            d.icon_sized("clock", d.color("text2"), 12.0, 1.3),
            Ui::xml(self.language.text("per-process · ETW ~2 s")),
            d.color("text2")
        ));
    }

    /// Less than 5 minutes collected: a dashed start line and a centered note.
    fn empty(&self, out: &mut String, plot: &Plot, first: u64, last: u64) {
        let d = self.design;
        let l = plot.layout;
        let x = plot.x(first) - plot.half_sample();
        out.push_str(&Self::rule(x, &l, d.color("text3"), true));
        if x - l.x0() < 200.0 {
            return;
        }
        let collected = self.language.elapsed((last - first).div_ceil(2));
        out.push_str(&format!(
            r#"<Grid Canvas.Left="{:.2}" Canvas.Top="{:.2}" Width="{:.2}" Height="{:.2}"><StackPanel HorizontalAlignment="Center" VerticalAlignment="Center" Spacing="4" Padding="12,0">{}<TextBlock Text="{}" FontSize="13" FontWeight="SemiBold" HorizontalAlignment="Center" TextAlignment="Center"/><TextBlock Text="{}" FontSize="12" Foreground="{}" TextAlignment="Center" TextWrapping="Wrap"/></StackPanel></Grid>"#,
            l.x0(),
            l.y0(),
            x - l.x0(),
            l.y1() - l.y0(),
            d.icon_sized("history", d.color("text3"), 20.0, 1.3).replace("VerticalAlignment=\"Center\"", "HorizontalAlignment=\"Center\""),
            self.language.text("Collecting history"),
            Ui::xml(
                &self
                    .language
                    .text("Collected {a} of 5:00. Left of the data is not zero but missing samples.")
                    .replace("{a}", &collected)
            ),
            d.color("text2")
        ));
    }

    /// «↓ Read» above and «↑ Write» below the mirrored axis.
    fn directions(&self, out: &mut String, plot: &Plot, resource: Resource) {
        let d = self.design;
        let l = plot.layout;
        let names = if resource == Resource::Disk {
            ["Read", "Write"]
        } else {
            ["Receive", "Send"]
        };
        for (name, icon, top) in [
            (names[0], "arrowdown", l.y0() + 4.0),
            (names[1], "arrowup", l.y1() - 22.0),
        ] {
            out.push_str(&format!(
                r#"<Border Canvas.Left="{:.2}" Canvas.Top="{top:.2}" Background="{}" CornerRadius="4" Padding="6,1"><StackPanel Orientation="Horizontal" Spacing="4">{}<TextBlock Text="{}" FontSize="11" FontWeight="SemiBold" Foreground="{}"/></StackPanel></Border>"#,
                l.x0() + 8.0,
                Design::alpha(d.color("card"), 0.9),
                d.icon_sized(icon, d.color("text2"), 12.0, 1.6),
                self.language.text(name),
                d.color("text2")
            ));
        }
    }

    fn moment(&self, out: &mut String, plot: &Plot, bucket: u64) {
        let d = self.design;
        let l = plot.layout;
        let x = plot.x(bucket);
        let time = self.language.time(bucket);
        out.push_str(&format!(
            r#"<Line X1="{x:.2}" X2="{x:.2}" Y1="{:.2}" Y2="{:.2}" Stroke="{}" StrokeThickness="1.25"/>"#,
            l.y0() - 4.0,
            l.y1(),
            d.color("ink")
        ));
        out.push_str(&Self::chip(
            x,
            l.y0() - 26.0,
            l.x0(),
            l.x1(),
            time.chars().count(),
            30.0,
            format!(
                r#"<Border HorizontalAlignment="Center" Height="20" Background="{}" CornerRadius="4" Padding="7,0"><StackPanel Orientation="Horizontal" Spacing="5">{}<TextBlock Text="{time}" FontSize="11" FontWeight="SemiBold" Foreground="{}" VerticalAlignment="Center" Typography.NumeralAlignment="Tabular"/></StackPanel></Border>"#,
                d.color("ink"),
                d.icon_sized("clock", d.color("card"), 11.0, 1.6),
                d.color("card")
            ),
        ));
    }

    fn dot(&self, out: &mut String, x: f64, y: f64, color: &str) {
        out.push_str(&format!(
            r#"<Ellipse Canvas.Left="{:.2}" Canvas.Top="{:.2}" Width="8.5" Height="8.5" Fill="{}" Stroke="{color}" StrokeThickness="2"/>"#,
            x - 4.25,
            y - 4.25,
            self.design.color("card")
        ));
    }

    pub fn series_of(
        frames: &[Arc<Frame>],
        device: &Device,
        key: ProcessKey,
    ) -> Vec<[Option<f64>; 2]> {
        // Samples keep the recorder's process order, so the position in the previous
        // frame usually holds the process again; a key is unique within a frame.
        let mut hint = 0;
        frames
            .iter()
            .map(|f| {
                let index = f
                    .processes
                    .get(hint)
                    .filter(|p| Timeline::key(p) == key)
                    .map(|_| hint)
                    .or_else(|| f.processes.iter().position(|p| Timeline::key(p) == key));
                if let Some(index) = index {
                    hint = index;
                }
                index
                    .map(|i| device.plotted(f, &f.processes[i]))
                    .unwrap_or([None, None])
            })
            .collect()
    }

    /// Axis maximum shared by the chart and the «Scale» statistic.
    pub fn scale(frames: &[Arc<Frame>], device: &Device, pinned: &[PinnedLayer]) -> f64 {
        let totals: Vec<_> = frames.iter().map(|f| device.total(f)).collect();
        let layers: Vec<_> = pinned
            .iter()
            .map(|p| Self::series_of(frames, device, p.key))
            .collect();
        let capacity = [device.capacity(), None];
        ChartScale::maximum(
            device.resource,
            totals
                .iter()
                .chain(layers.iter().flatten())
                .chain(std::iter::once(&capacity)),
        )
    }

    /// Shared coordinate transform of the base chart and its overlay.
    fn plot(
        frames: &[Arc<Frame>],
        device: &Device,
        values: &[[Option<f64>; 2]],
        layers: &[(&PinnedLayer, Vec<[Option<f64>; 2]>)],
        layout: ChartLayout,
    ) -> Plot {
        let end = frames.last().map_or(0, |f| f.bucket);
        Plot {
            layout,
            start: end.saturating_sub(WINDOW - 1),
            maximum: ChartScale::maximum(
                device.resource,
                values
                    .iter()
                    .chain(layers.iter().flat_map(|(_, s)| s.iter()))
                    .chain(std::iter::once(&[device.capacity(), None])),
            ),
            mirrored: device.resource.dual(),
        }
    }

    /// Axes, total and pinned series, temperature, gaps and ETW marks. The series sit
    /// in the `Series` canvas, dimmed while a process is highlighted by `overlay`.
    pub fn markup(
        &self,
        frames: &[Arc<Frame>],
        device: &Device,
        selected: Option<u64>,
        pinned: &[PinnedLayer],
        layout: ChartLayout,
    ) -> String {
        let d = self.design;
        let mut out = format!(
            r#"<Canvas xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Width="{}" Height="{}" IsHitTestVisible="False">"#,
            layout.width, layout.height
        );
        let resource = device.resource;
        let live = selected.is_none();
        let end = frames.last().map_or(0, |f| f.bucket);
        let values: Vec<_> = frames.iter().map(|f| device.total(f)).collect();
        let layers: Vec<_> = pinned
            .iter()
            .map(|p| (p, Self::series_of(frames, device, p.key)))
            .collect();
        let plot = Self::plot(frames, device, &values, &layers, layout);
        self.axes(&mut out, &plot, resource);
        self.time_axis(&mut out, &plot, end, live);
        let Some(first) = frames.first() else {
            out.push_str("</Canvas>");
            return out;
        };
        let total = Style {
            color: d.color("total"),
            fill: d.fill(0),
            width: self.line,
            opacity: 1.0,
        };
        out.push_str(r#"<Canvas x:Name="Series">"#);
        for fill in [true, false] {
            self.series(
                &mut out,
                &plot,
                frames,
                &values,
                &total,
                Pass {
                    attributed: false,
                    live,
                    fill,
                },
            );
            for (layer, points) in &layers {
                let style = Style {
                    color: &layer.color,
                    fill: d.fill(1),
                    width: self.line * 1.25 / 1.5,
                    opacity: 1.0,
                };
                self.series(
                    &mut out,
                    &plot,
                    frames,
                    points,
                    &style,
                    Pass {
                        attributed: true,
                        live,
                        fill,
                    },
                );
            }
        }
        if resource.temperature_axis() {
            self.temperature(&mut out, &plot, frames, device);
        }
        out.push_str("</Canvas>");
        self.gaps(&mut out, &plot, frames, device);
        if resource == Resource::Ram {
            self.shared_gaps(&mut out, &plot, frames);
        }
        if first.bucket > plot.start + ETW_DELAY {
            self.empty(&mut out, &plot, first.bucket, end);
        }
        if resource.dual() {
            if live {
                self.etw(&mut out, &plot, end);
            }
            self.directions(&mut out, &plot, resource);
        }
        out.push_str("</Canvas>");
        out
    }

    /// Highlighted process and the selected moment with its dots: small enough to be
    /// redrawn on every hover and drag without touching the base chart.
    pub fn overlay(
        &self,
        frames: &[Arc<Frame>],
        device: &Device,
        selected: Option<u64>,
        hover: Option<Highlight>,
        pinned: &[PinnedLayer],
        layout: ChartLayout,
    ) -> String {
        let d = self.design;
        let mut out = format!(
            r#"<Canvas xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Width="{}" Height="{}" IsHitTestVisible="False">"#,
            layout.width, layout.height
        );
        let values: Vec<_> = frames.iter().map(|f| device.total(f)).collect();
        let layers: Vec<_> = pinned
            .iter()
            .map(|p| (p, Self::series_of(frames, device, p.key)))
            .collect();
        let plot = Self::plot(frames, device, &values, &layers, layout);
        let process = hover.map(|hover| hover.series(frames, device, pinned));
        if let Some(points) = &process {
            let hover = Style {
                color: d.color("ink"),
                fill: d.fill(2),
                width: self.line * 1.75 / 1.5,
                opacity: 1.0,
            };
            for fill in [true, false] {
                self.series(
                    &mut out,
                    &plot,
                    frames,
                    points,
                    &hover,
                    Pass {
                        attributed: true,
                        live: selected.is_none(),
                        fill,
                    },
                );
            }
        }
        if let Some(index) = selected.and_then(|b| frames.iter().position(|f| f.bucket == b)) {
            let bucket = frames[index].bucket;
            self.moment(&mut out, &plot, bucket);
            let x = plot.x(bucket);
            for direction in 0..if plot.mirrored { 2 } else { 1 } {
                if let Some(v) = values[index][direction] {
                    self.dot(&mut out, x, plot.y(v, direction), d.color("total"));
                }
                for (layer, points) in &layers {
                    if let Some(v) = points[index][direction] {
                        self.dot(&mut out, x, plot.y(v, direction), &layer.color);
                    }
                }
                if let Some(v) = process.as_ref().and_then(|p| p[index][direction]) {
                    self.dot(&mut out, x, plot.y(v, direction), d.color("ink"));
                }
            }
            // On the drawn line; the tooltip keeps the measured value.
            if let Some(v) = Smoothing::temperatures(frames, device)[index] {
                self.dot(&mut out, x, plot.temperature(v), d.color("temp"));
            }
        }
        out.push_str("</Canvas>");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::devices::DeviceId;

    #[test]
    fn dashes_keep_their_moments_when_the_window_scrolls() {
        // 1.4 px a bucket, a line that wanders, x counted from the window start.
        let line = |first: u64, start: u64| {
            DashedLine::path(
                (first..140).map(|b| {
                    let x = (b as f64 - start as f64) * 1.4;
                    (b, x, Some(50.0 + (b % 7) as f64 * 3.0))
                }),
                1.4,
            )
        };
        // Five buckets later the oldest left: the dashes after the first one, which
        // the left edge may cut, are the same as before at the same places.
        let full = line(100, 105);
        let scrolled = line(105, 105);
        let rest = &scrolled[scrolled[1..].find('M').unwrap() + 1..];
        assert!(full.ends_with(rest), "{full}\n{scrolled}");
        // On a level line each dash is 4 px long and the next starts 7 px later.
        let level = DashedLine::path((0..20).map(|b| (b, b as f64 * 1.4, Some(10.0))), 1.4);
        assert!(level.starts_with(
            "M 0.00,10.00 L 1.40,10.00 2.80,10.00 4.00,10.00 M 7.00,10.00 L 8.40,10.00 9.80,10.00 11.00,10.00 "
        ));
        // Missing samples break the line.
        let gap = DashedLine::path(
            [(0, 0.0, Some(10.0)), (1, 1.4, None), (2, 2.8, Some(10.0))],
            1.4,
        );
        assert!(!gap.contains('L'));
    }

    fn frame(bucket: u64, totals: Vec<(&str, Option<f64>)>) -> Arc<Frame> {
        Arc::new(Frame {
            bucket,
            elapsed_ms: 500.0,
            processes: vec![],
            gpu_engines: Vec::new(),
            totals: totals.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            etw_active: true,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 0.0,
        })
    }

    #[test]
    fn process_series_follow_reordered_and_missing_samples() {
        use crate::platform::process_history::store::{Identity, Sample};
        let sample = |pid: u32, cpu: f64| {
            Sample::new(
                Arc::new(Identity {
                    pid,
                    created: 7,
                    name: "p".into(),
                }),
                Some(cpu),
                None,
                [None, None, None],
                None,
            )
        };
        let orders: [&[u32]; 5] = [&[1, 2, 3], &[1, 2, 3], &[3, 1, 2], &[1, 3], &[2, 3, 1]];
        let frames: Vec<_> = orders
            .iter()
            .enumerate()
            .map(|(bucket, pids)| {
                let mut f = (*frame(bucket as u64, vec![])).clone();
                f.processes = pids
                    .iter()
                    .map(|pid| sample(*pid, (*pid * 10) as f64 + bucket as f64))
                    .collect();
                Arc::new(f)
            })
            .collect();
        let device = Device::of(Resource::Cpu);
        let cpu: Vec<_> = ChartRenderer::series_of(&frames, &device, (2, 7))
            .iter()
            .map(|v| v[0])
            .collect();
        assert_eq!(cpu, [Some(20.0), Some(21.0), Some(22.0), None, Some(24.0)]);
    }

    fn renderer() -> ChartRenderer {
        ChartRenderer {
            design: Design { dark: false },
            language: Language::English,
            line: super::super::chart_line::ChartLine::DEFAULT,
        }
    }

    #[test]
    fn mirrored_directions_share_scale_and_temperature_is_clipped() {
        let frames = [frame(
            1200,
            vec![
                ("disk_read", Some(10.0)),
                ("disk_write", Some(5.0)),
                ("cpu_temperature", Some(105.0)),
            ],
        )];
        let layout = ChartLayout::new(400.0, 300.0);
        let disk = renderer().markup(
            &frames,
            &Device::of(Resource::Disk),
            Some(1200),
            &[],
            layout.for_resource(Resource::Disk),
        );
        assert!(disk.contains(r#"Text="10 MB/s""#));
        assert!(disk.contains(",28.00") && disk.contains(",215.50"));
        let cpu = renderer().markup(&frames, &Device::of(Resource::Cpu), None, &[], layout);
        assert!(cpu.contains("above scale, peak 105 °C"));
        assert!(cpu.contains("Collecting history") && cpu.contains(r#"StrokeDashArray="3,3""#));
        assert!(
            cpu.contains("100\u{A0}%") && cpu.contains("100 °C") && cpu.contains(r#"Text="now""#)
        );
    }

    #[test]
    fn moment_is_drawn_by_the_overlay_over_the_named_series() {
        let frames: Vec<_> = (0..3)
            .map(|b| frame(100 + b, vec![("cpu", Some(20.0))]))
            .collect();
        let layout = ChartLayout::new(600.0, 250.0);
        let base = renderer().markup(&frames, &Device::of(Resource::Cpu), Some(101), &[], layout);
        assert!(base.contains(r#"x:Name="Series""#) && !base.contains("<Ellipse"));
        let overlay = renderer().overlay(
            &frames,
            &Device::of(Resource::Cpu),
            Some(101),
            None,
            &[],
            layout,
        );
        assert!(overlay.contains("<Ellipse") && !overlay.contains(r#"x:Name="Series""#));
    }

    #[test]
    fn unmeasured_canvas_never_produces_negative_sizes() {
        let frames = [frame(
            10,
            vec![("cpu", Some(5.0)), ("cpu_temperature", Some(50.0))],
        )];
        let markup = renderer().markup(
            &frames,
            &Device::of(Resource::Cpu),
            Some(10),
            &[],
            ChartLayout::new(0.0, 0.0),
        );
        assert!(!markup.contains("Width=\"-") && !markup.contains("Height=\"-"));
    }

    #[test]
    fn scale_grows_by_steps_above_resource_minimum() {
        let values = [[Some(12.0), Some(3.0)]];
        assert_eq!(ChartScale::maximum(Resource::Disk, values.iter()), 20.0);
        assert_eq!(
            ChartScale::maximum(Resource::Disk, [[Some(3.0), None]].iter()),
            10.0
        );
        assert_eq!(
            ChartScale::maximum(Resource::Net, [[Some(0.35), None]].iter()),
            1.0
        );
        assert_eq!(
            ChartScale::maximum(Resource::Ram, [[Some(3431.0), None]].iter()),
            4096.0
        );
        assert_eq!(
            ChartScale::maximum(Resource::Ram, [[Some(25_331.0), None]].iter()),
            28_672.0
        );
        // The memory a 30.9 GB machine reports lifts a 14.5 GB line to 32 GB.
        assert_eq!(
            ChartScale::maximum(
                Resource::Ram,
                [[Some(14_848.0), None], [Some(31_641.0), None]].iter()
            ),
            32_768.0
        );
        assert_eq!(
            ChartScale::maximum(Resource::Cpu, [[Some(300.0), None]].iter()),
            100.0
        );
    }

    #[test]
    fn gaps_are_hatched_and_live_io_waits_for_etw() {
        let frames: Vec<_> = (0..10)
            .chain(30..40)
            .map(|b| {
                frame(
                    2000 + b,
                    vec![("net_down", Some(0.5)), ("net_up", Some(0.1))],
                )
            })
            .collect();
        let layout = ChartLayout::new(900.0, 260.0).for_resource(Resource::Net);
        let markup = renderer().markup(&frames, &Device::of(Resource::Net), None, &[], layout);
        assert!(markup.contains("no samples · 10 s"));
        assert!(markup.contains("per-process · ETW ~2 s"));
        assert!(markup.contains("Receive") && markup.contains("Send"));
    }

    #[test]
    fn frames_without_the_device_are_hatched_like_missing_samples() {
        let frames: Vec<_> = (0..30)
            .map(|b| {
                let totals = if (10..20).contains(&b) {
                    vec![("cpu", Some(5.0))]
                } else {
                    vec![("cpu", Some(5.0)), ("disk_read@D:", Some(1.0))]
                };
                frame(3000 + b, totals)
            })
            .collect();
        let disk = Device::new(DeviceId::parse("disk@D:").unwrap()).unwrap();
        assert_eq!(ChartRenderer::unmeasured(&frames, &disk), [(3010, 3019)]);
        let layout = ChartLayout::new(900.0, 260.0).for_resource(Resource::Disk);
        let markup = renderer().markup(&frames, &disk, None, &[], layout);
        assert!(markup.contains("no samples · 5 s"));
    }

    #[test]
    fn ram_hatches_frames_without_shared_memory() {
        use crate::platform::process_history::store::{Identity, Sample};
        let process = |pid: u32, shared: Option<u64>| {
            let mut sample = Sample::new(
                Arc::new(Identity {
                    pid,
                    created: 1,
                    name: "p".into(),
                }),
                None,
                None,
                [Some(1 << 20), None, None],
                None,
            );
            sample.set_shared(shared);
            sample
        };
        let frames: Vec<_> = (0..300)
            .map(|b| {
                let mut frame = frame(4000 + b, vec![("ram", Some(1000.0))]);
                // The window opens at 4200; the memory rows always carry shared bytes.
                let shared = (b >= 200).then_some(1 << 20);
                let processes = vec![process(4, shared), process(MemoryRows::PID, Some(5))];
                Arc::make_mut(&mut frame).processes = if b < 20 { vec![] } else { processes };
                frame
            })
            .collect();
        assert_eq!(ChartRenderer::unshared(&frames), [(4020, 4199)]);
        let layout = ChartLayout::new(900.0, 260.0).for_resource(Resource::Ram);
        let markup = renderer().markup(&frames, &Device::of(Resource::Ram), None, &[], layout);
        assert!(markup.contains("shared memory not measured"));
    }

    #[test]
    fn vertical_rules_stay_in_the_middle_of_a_pixel_column() {
        let plain = ChartLayout::new(900.0, 260.0);
        assert_eq!([plain.pixel(100.2), plain.pixel(100.9)], [100.5, 100.5]);
        // 125 %: columns are 0.8 XAML pixels wide.
        let scaled = plain.scale(1.25);
        assert!((scaled.pixel(100.2) - 100.4).abs() < 1e-9);
    }

    #[test]
    fn resized_chart_preserves_hit_testing_and_stroke_sizes() {
        for width in [400.0, 900.0, 1500.0] {
            let layout = ChartLayout::new(width, 300.0);
            assert_eq!(layout.fraction(46.0 + (width - 90.0) / 2.0), 0.5);
            assert_eq!(layout.fraction(width - 44.0), 1.0);
            let markup = renderer().markup(&[], &Device::of(Resource::Cpu), None, &[], layout);
            assert!(markup.contains(&format!("X2=\"{:.2}\"", width - 44.0)));
            assert!(markup.contains("StrokeThickness=\"1\""));
            assert!(!markup.contains("Viewbox Width=\"400\""));
        }
    }

    #[test]
    fn temperature_jitter_is_averaged_within_runs_only() {
        let samples = [
            (1, Some(40.0)),
            (2, Some(41.0)),
            (3, Some(40.0)),
            (4, Some(41.0)),
            (5, Some(40.0)),
            (6, None),
            (7, Some(60.0)),
            (9, Some(70.0)),
        ];
        let smoothed = Smoothing::apply(&samples);
        // A whole-degree flip settles between the two readings.
        assert_eq!(smoothed[2], Some(40.4));
        assert_eq!(smoothed[0], Some(121.0 / 3.0));
        // Gaps and missing samples are never averaged across.
        assert_eq!(smoothed[5], None);
        assert_eq!(smoothed[6], Some(60.0));
        assert_eq!(smoothed[7], Some(70.0));
    }

    #[test]
    fn filled_areas_close_each_run_without_bridging_missing_samples() {
        let shape = SeriesGeometry::new(
            [
                (1, 8.0, Some(100.0)),
                (2, 9.0, Some(80.0)),
                (3, 10.0, None),
                (4, 11.0, Some(120.0)),
                (6, 13.0, Some(90.0)),
            ],
            236.0,
        );
        assert_eq!(shape.area,"M 8.00,236.00 L 8.00,100.00 9.00,80.00 9.00,236.00 Z M 11.00,236.00 L 11.00,120.00 11.00,236.00 Z M 13.00,236.00 L 13.00,90.00 13.00,236.00 Z ");
        assert_eq!(shape.line.matches('M').count(), 3);
    }
}
