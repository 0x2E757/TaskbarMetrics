use super::metric_id::MetricSelector;
use std::path::PathBuf;

/// What `--dump` writes, from its arguments.
pub struct DumpQuery {
    /// The last seconds of the history, 1–300.
    pub(super) seconds: u64,
    /// No items: every total.
    pub(super) metrics: Vec<MetricSelector>,
    /// Every 500 ms frame instead of the average, maximum and last value.
    pub(super) series: bool,
    pub(super) processes: Option<ProcessQuery>,
    /// The recorder's whole export as it is.
    pub(super) raw: bool,
    pub(super) output: Option<PathBuf>,
}

pub(super) struct ProcessQuery {
    pub count: usize,
    pub by: Ranking,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Ranking {
    Cpu,
    Gpu,
    Ram,
    Disk,
    Net,
}

impl Ranking {
    const NAMES: [(&'static str, Self); 5] = [
        ("cpu", Self::Cpu),
        ("gpu", Self::Gpu),
        ("ram", Self::Ram),
        ("disk", Self::Disk),
        ("net", Self::Net),
    ];

    pub fn name(self) -> &'static str {
        Self::NAMES
            .iter()
            .find(|(_, ranking)| *ranking == self)
            .map_or("", |(name, _)| name)
    }
}

impl DumpQuery {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut query = Self {
            seconds: 60,
            metrics: Vec::new(),
            series: false,
            processes: None,
            raw: false,
            output: None,
        };
        let mut by = None;
        let mut filtered = false;
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            let mut value = || {
                args.next()
                    .ok_or_else(|| Self::error(&format!("{arg} needs a value")))
            };
            match arg.as_str() {
                "--last" => query.seconds = Self::number(value()?, arg, 1, 300)?,
                "--metrics" => {
                    query.metrics = value()?
                        .split(',')
                        .map(str::trim)
                        .filter(|item| !item.is_empty())
                        .map(MetricSelector::parse)
                        .collect()
                }
                "--series" => query.series = true,
                "--processes" => {
                    query.processes = Some(ProcessQuery {
                        count: Self::number(value()?, arg, 1, 1000)? as usize,
                        by: Ranking::Cpu,
                    })
                }
                "--by" => by = Some(Self::ranking(value()?)?),
                "--raw" => query.raw = true,
                "--output" => query.output = Some(PathBuf::from(value()?)),
                _ => return Err(Self::error(&format!("unknown option {arg}"))),
            }
            filtered |= !matches!(arg.as_str(), "--raw" | "--output");
        }
        if query.raw && filtered {
            return Err(Self::error("--raw takes only --output"));
        }
        if let Some(by) = by {
            query
                .processes
                .as_mut()
                .ok_or_else(|| Self::error("--by needs --processes"))?
                .by = by;
        }
        Ok(query)
    }

    fn number(text: &str, option: &str, low: u64, high: u64) -> Result<u64, String> {
        text.parse()
            .ok()
            .filter(|n| (low..=high).contains(n))
            .ok_or_else(|| Self::error(&format!("{option} takes a number from {low} to {high}")))
    }

    fn ranking(text: &str) -> Result<Ranking, String> {
        Ranking::NAMES
            .iter()
            .find(|(name, _)| *name == text)
            .map(|(_, ranking)| *ranking)
            .ok_or_else(|| Self::error("--by takes cpu, gpu, ram, disk or net"))
    }

    fn error(problem: &str) -> String {
        format!("--dump: {problem}. TaskbarMetrics.exe --help lists the options.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<DumpQuery, String> {
        DumpQuery::parse(&args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn defaults_to_a_summary_of_every_total_over_a_minute() {
        let query = parse(&[]).unwrap();
        assert_eq!(query.seconds, 60);
        assert!(query.metrics.is_empty() && !query.series && !query.raw);
        assert!(query.processes.is_none() && query.output.is_none());
    }

    #[test]
    fn reads_every_option() {
        let query = parse(&[
            "--last",
            "300",
            "--metrics",
            "cpu, disk@C:,",
            "--series",
            "--by",
            "disk",
            "--processes",
            "5",
            "--output",
            "out.csv",
        ])
        .unwrap();
        assert_eq!(query.seconds, 300);
        assert_eq!(query.metrics.len(), 2);
        assert!(query.series);
        let processes = query.processes.unwrap();
        assert_eq!((processes.count, processes.by), (5, Ranking::Disk));
        assert_eq!(query.output, Some(PathBuf::from("out.csv")));
    }

    #[test]
    fn refuses_what_it_cannot_honor() {
        for args in [
            &["--last", "0"][..],
            &["--last", "301"],
            &["--last"],
            &["--processes", "many"],
            &["--by", "cpu"],
            &["--processes", "3", "--by", "io"],
            &["--raw", "--series"],
            &["--sample"],
        ] {
            let error = parse(args).err().unwrap();
            assert!(error.ends_with("TaskbarMetrics.exe --help lists the options."));
        }
        assert!(parse(&["--raw", "--output", "all.csv"]).unwrap().raw);
    }
}
