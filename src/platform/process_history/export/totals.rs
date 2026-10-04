use super::{
    super::store::Frame,
    metric_id::{MetricId, MetricSelector},
    Csv,
};
use std::{
    io::{self, Write},
    sync::Arc,
};

/// The totals of the chosen metrics over a window of frames.
pub(super) struct TotalsTable<'a> {
    frames: &'a [Arc<Frame>],
    /// In the order the recorder keeps them.
    ids: Vec<&'a str>,
}

impl<'a> TotalsTable<'a> {
    /// No `selectors`: every total.
    pub fn new(frames: &'a [Arc<Frame>], selectors: &[MetricSelector]) -> Self {
        let mut ids = Vec::new();
        for (id, _) in frames.iter().flat_map(|f| &f.totals) {
            let chosen = selectors.is_empty()
                || selectors
                    .iter()
                    .any(|selector| selector.matches(&MetricId::parse(id)));
            if chosen && !ids.contains(&id.as_str()) {
                ids.push(id.as_str());
            }
        }
        Self { frames, ids }
    }

    /// One row a metric: the average, maximum and last measured value.
    pub fn summary(&self, out: &mut dyn Write) -> io::Result<()> {
        writeln!(out, "metric,unit,average,maximum,last")?;
        for id in &self.ids {
            let values: Vec<_> = self
                .frames
                .iter()
                .filter_map(|f| Self::value(f, id))
                .collect();
            let unit = MetricId::parse(id).unit();
            let average =
                (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64);
            let maximum = values.iter().copied().reduce(f64::max);
            writeln!(
                out,
                "{},{unit},{},{},{}",
                Csv::field(id),
                Self::format(average, unit),
                Self::format(maximum, unit),
                Self::format(values.last().copied(), unit)
            )?;
        }
        Ok(())
    }

    /// One row a frame, one column a metric.
    pub fn series(&self, out: &mut dyn Write) -> io::Result<()> {
        write!(out, "timestamp_ms")?;
        for id in &self.ids {
            let column = format!("{id} [{}]", MetricId::parse(id).unit());
            write!(out, ",{}", Csv::field(&column))?;
        }
        writeln!(out)?;
        for frame in self.frames {
            write!(out, "{}", frame.bucket * 500)?;
            for id in &self.ids {
                let unit = MetricId::parse(id).unit();
                write!(out, ",{}", Self::format(Self::value(frame, id), unit))?;
            }
            writeln!(out)?;
        }
        Ok(())
    }

    fn value(frame: &Frame, id: &str) -> Option<f64> {
        frame
            .totals
            .iter()
            .find(|(other, _)| other == id)
            .and_then(|(_, value)| *value)
    }

    /// Empty when not measured.
    fn format(value: Option<f64>, unit: &str) -> String {
        match value {
            None => String::new(),
            Some(value) if unit == "bytes" => format!("{value:.0}"),
            Some(value) => format!("{value:.3}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(bucket: u64, totals: &[(&str, Option<f64>)]) -> Arc<Frame> {
        Arc::new(Frame {
            gpu_engines: Vec::new(),
            bucket,
            elapsed_ms: 500.0,
            processes: Vec::new(),
            totals: totals
                .iter()
                .map(|(id, value)| (id.to_string(), *value))
                .collect(),
            etw_active: false,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 1.0,
        })
    }

    fn text(write: impl FnOnce(&mut dyn Write) -> io::Result<()>) -> String {
        let mut out = Vec::new();
        write(&mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn summarizes_the_chosen_metrics_skipping_gaps() {
        let frames = [
            frame(1, &[("cpu", Some(10.0)), ("ram_used", Some(1000.0))]),
            frame(2, &[("cpu", None), ("ram_used", Some(3000.0))]),
            frame(3, &[("cpu", Some(30.0)), ("disk_read@C,1", Some(0.5))]),
        ];
        let table = TotalsTable::new(&frames, &[]);
        assert_eq!(
            text(|out| table.summary(out)),
            "metric,unit,average,maximum,last\n\
             cpu,percent,20.000,30.000,30.000\n\
             ram_used,bytes,2000,3000,3000\n\
             \"disk_read@C,1\",mb_per_s,0.500,0.500,0.500\n"
        );
        let table = TotalsTable::new(&frames, &[MetricSelector::parse("cpu")]);
        assert_eq!(
            text(|out| table.series(out)),
            "timestamp_ms,cpu [percent]\n500,10.000\n1000,\n1500,30.000\n"
        );
    }
}
