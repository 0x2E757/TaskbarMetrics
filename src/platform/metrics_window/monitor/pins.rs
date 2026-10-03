#[cfg(test)]
use super::model::Resource;
use super::model::{Device, ProcessKey, Timeline};
use super::totals::IoTotals;
use crate::platform::process_history::store::{Frame, Identity};
use std::{path::Path, sync::Arc};

/// A pinned process instance and its stable palette index.
struct Pin {
    identity: Arc<Identity>,
    color: usize,
}

#[derive(Default)]
pub struct PinnedProcesses {
    identities: Vec<Pin>,
}

pub struct ProcessRow {
    pub identity: Arc<Identity>,
    pub sample: Option<usize>,
    pub pinned: bool,
}

pub struct PinnedLayer {
    pub key: ProcessKey,
    pub color: String,
    pub name: String,
}

impl PinnedProcesses {
    /// Palette size; a seventh pin is refused.
    pub const LIMIT: usize = 6;

    pub fn layers(
        &self,
        design: super::design::Design,
        language: super::locale::Language,
    ) -> Vec<PinnedLayer> {
        self.identities
            .iter()
            .map(|p| PinnedLayer {
                key: (p.identity.pid, p.identity.created),
                color: design.pin(p.color).into(),
                name: language.process(&p.identity).to_string(),
            })
            .collect()
    }

    pub fn full(&self) -> bool {
        self.identities.len() >= Self::LIMIT
    }

    pub fn process_color(&self, key: ProcessKey, design: super::design::Design) -> Option<String> {
        self.identities
            .iter()
            .find(|p| (p.identity.pid, p.identity.created) == key)
            .map(|p| design.pin(p.color).into())
    }

    pub fn contains(&self, key: ProcessKey) -> bool {
        self.identities
            .iter()
            .any(|p| (p.identity.pid, p.identity.created) == key)
    }

    pub fn toggle(&mut self, identity: Arc<Identity>) {
        let key = (identity.pid, identity.created);
        if self.contains(key) {
            self.identities
                .retain(|p| (p.identity.pid, p.identity.created) != key);
        } else if !self.full() {
            self.push(identity);
        }
    }

    /// The first palette index not used by another pin.
    fn push(&mut self, identity: Arc<Identity>) {
        let color = (0..Self::LIMIT)
            .find(|c| self.identities.iter().all(|p| p.color != *c))
            .unwrap_or(0);
        self.identities.push(Pin { identity, color });
    }

    pub fn rows(
        &self,
        frame: Option<&Frame>,
        device: &Device,
        search: &str,
        all: bool,
        column: usize,
        totals: Option<&IoTotals>,
    ) -> (Vec<ProcessRow>, Vec<usize>) {
        let mut rows: Vec<_> = self
            .identities
            .iter()
            .map(|pin| ProcessRow {
                identity: pin.identity.clone(),
                pinned: true,
                sample: frame.and_then(|f| {
                    f.processes
                        .iter()
                        .position(|p| Timeline::key(p) == (pin.identity.pid, pin.identity.created))
                }),
            })
            .collect();
        let Some(frame) = frame else {
            return (rows, Vec::new());
        };
        // Pinned rows follow the sort too; those without a sample go last.
        let weight = |row: &ProcessRow| {
            row.sample.map_or(f64::NEG_INFINITY, |i| {
                device.weight(frame, &frame.processes[i], column, totals)
            })
        };
        rows.sort_by(|a, b| {
            weight(b)
                .total_cmp(&weight(a))
                .then_with(|| a.identity.pid.cmp(&b.identity.pid))
        });
        let ranked: Vec<_> = device
            .ranked(frame, search, column, totals)
            .into_iter()
            .filter(|i| !self.contains(Timeline::key(&frame.processes[*i])))
            .collect();
        let limit = if all || !search.is_empty() {
            ranked.len()
        } else {
            10.min(ranked.len())
        };
        rows.extend(ranked[..limit].iter().map(|i| ProcessRow {
            identity: frame.processes[*i].identity.clone(),
            sample: Some(*i),
            pinned: false,
        }));
        (rows, ranked[limit..].to_vec())
    }

    pub fn load(path: &Path) -> Self {
        let mut result = Self::default();
        if let Ok(text) = std::fs::read_to_string(path) {
            for line in text.lines().take(1024) {
                let mut fields = line.splitn(3, '\t');
                let Some(pid) = fields.next().and_then(|s| s.parse().ok()) else {
                    continue;
                };
                let Some(created) = fields.next().and_then(|s| s.parse().ok()) else {
                    continue;
                };
                let Some(name) = fields.next() else {
                    continue;
                };
                if !result.contains((pid, created)) && !result.full() {
                    result.push(Arc::new(Identity {
                        pid,
                        created,
                        name: name.into(),
                    }));
                }
            }
        }
        result
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let text = self
            .identities
            .iter()
            .map(|Pin { identity: p, .. }| {
                format!(
                    "{}\t{}\t{}\n",
                    p.pid,
                    p.created,
                    p.name.replace(['\r', '\n', '\t'], " ")
                )
            })
            .collect::<String>();
        let temporary = path.with_extension("pins.tmp");
        std::fs::write(&temporary, text)?;
        std::fs::rename(temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::process_history::store::Sample;

    #[test]
    fn pins_cross_tabs_ignore_search_and_do_not_follow_reused_pid() {
        let identity = Arc::new(Identity {
            pid: 7,
            created: 10,
            name: "pinned.exe".into(),
        });
        let mut pins = PinnedProcesses::default();
        pins.toggle(identity.clone());
        let frame = Frame {
            gpu_engines: Vec::new(),
            bucket: 1,
            elapsed_ms: 500.0,
            processes: vec![Sample::new(
                Arc::new(Identity {
                    pid: 7,
                    created: 20,
                    name: "new.exe".into(),
                }),
                Some(80.0),
                None,
                [None, None, None],
                None,
            )],
            totals: vec![],
            etw_active: false,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 0.0,
        };
        for resource in Resource::ALL {
            let (rows, _) = pins.rows(
                Some(&frame),
                &Device::of(resource),
                "unmatched",
                false,
                0,
                None,
            );
            assert_eq!(rows.len(), 1);
            assert!(rows[0].pinned);
            assert_eq!(rows[0].sample, None);
            assert_eq!(rows[0].identity.created, 10);
        }
        let (rows, rest) = pins.rows(Some(&frame), &Device::of(Resource::Cpu), "", false, 0, None);
        assert_eq!(rows.len(), 2);
        assert!(rest.is_empty());
        pins.toggle(identity);
        assert!(!pins.contains((7, 10)));
    }

    #[test]
    fn pinned_rows_follow_the_sort_and_missing_ones_go_last() {
        let identity = |pid| {
            Arc::new(Identity {
                pid,
                created: 1,
                name: format!("p{pid}").into(),
            })
        };
        let mut pins = PinnedProcesses::default();
        for pid in [3, 1, 2] {
            pins.toggle(identity(pid));
        }
        let sample = |pid, cpu| Sample::new(identity(pid), Some(cpu), None, [None; 3], None);
        let frame = Frame {
            gpu_engines: Vec::new(),
            bucket: 1,
            elapsed_ms: 500.0,
            processes: vec![sample(1, 10.0), sample(2, 80.0)],
            totals: vec![],
            etw_active: false,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 0.0,
        };
        let (rows, _) = pins.rows(Some(&frame), &Device::of(Resource::Cpu), "", false, 0, None);
        let order: Vec<_> = rows.iter().map(|r| r.identity.pid).collect();
        assert_eq!(order, [2, 1, 3]);
    }

    #[test]
    fn colors_stay_with_their_pin_and_seventh_pin_is_refused() {
        let design = super::super::design::Design { dark: false };
        let identity = |pid| {
            Arc::new(Identity {
                pid,
                created: 1,
                name: format!("p{pid}").into(),
            })
        };
        let mut pins = PinnedProcesses::default();
        for pid in 1..=7 {
            pins.toggle(identity(pid));
        }
        assert!(pins.full() && !pins.contains((7, 1)));
        pins.toggle(identity(2));
        assert_eq!(
            pins.process_color((3, 1), design),
            Some(design.pin(2).into())
        );
        pins.toggle(identity(8));
        assert_eq!(
            pins.process_color((8, 1), design),
            Some(design.pin(1).into())
        );
    }
}
