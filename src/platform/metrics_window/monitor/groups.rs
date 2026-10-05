use crate::platform::process_history::store::Frame;
use std::collections::{HashMap, HashSet};

/// Processes of one executable name, as the table groups them: a browser runs as
/// many processes but reads as one program. Windows compares file names
/// regardless of case.
pub struct NameGroups;

impl NameGroups {
    pub fn same(a: &str, b: &str) -> bool {
        a.to_lowercase() == b.to_lowercase()
    }

    /// `ranked` processes of `frame` as rows: merged by name when `grouped`, one
    /// process per row otherwise.
    pub fn rows(
        frame: &Frame,
        ranked: Vec<usize>,
        grouped: bool,
        weight: impl Fn(usize) -> f64,
    ) -> Vec<Vec<usize>> {
        if grouped {
            Self::merge(frame, &ranked, weight)
        } else {
            ranked.into_iter().map(|index| vec![index]).collect()
        }
    }

    /// `ranked` processes of `frame`, busiest first, merged by name: each group
    /// leads with its busiest process, and the groups rank by the sum of `weight`.
    pub fn merge(
        frame: &Frame,
        ranked: &[usize],
        weight: impl Fn(usize) -> f64,
    ) -> Vec<Vec<usize>> {
        let mut groups: Vec<Vec<usize>> = Vec::new();
        let mut names: HashMap<String, usize> = HashMap::new();
        for &index in ranked {
            let name = frame.processes[index].identity.name.to_lowercase();
            match names.get(&name) {
                Some(&group) => groups[group].push(index),
                None => {
                    names.insert(name, groups.len());
                    groups.push(vec![index]);
                }
            }
        }
        let sums: Vec<f64> = groups
            .iter()
            .map(|group| group.iter().map(|&index| weight(index)).sum())
            .collect();
        let mut order: Vec<usize> = (0..groups.len()).collect();
        // Stable: equal sums keep the order of their leads.
        order.sort_by(|a, b| sums[*b].total_cmp(&sums[*a]));
        order
            .into_iter()
            .map(|group| std::mem::take(&mut groups[group]))
            .collect()
    }

    /// Number of distinct names among `names`.
    pub fn count<'a>(names: impl IntoIterator<Item = &'a str>) -> usize {
        names
            .into_iter()
            .map(str::to_lowercase)
            .collect::<HashSet<_>>()
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::process_history::store::{Identity, Sample};
    use std::sync::Arc;

    fn frame(names: &[&str]) -> Frame {
        Frame {
            gpu_engines: Vec::new(),
            bucket: 1,
            elapsed_ms: 500.0,
            processes: names
                .iter()
                .enumerate()
                .map(|(pid, name)| {
                    Sample::new(
                        Arc::new(Identity {
                            pid: pid as u32,
                            created: 1,
                            name: (*name).into(),
                        }),
                        None,
                        None,
                        [None; 3],
                        None,
                    )
                })
                .collect(),
            totals: vec![],
            etw_active: false,
            lost_events: 0,
            undecoded: 0,
            sample_ms: 0.0,
        }
    }

    #[test]
    fn groups_lead_with_their_busiest_process_and_rank_by_their_sum() {
        let frame = frame(&["chrome.exe", "code.exe", "Chrome.exe", "chrome.exe"]);
        let weights = [5.0, 7.0, 2.0, 1.0];
        // Ranked busiest first: code 7, chrome 5, Chrome 2, chrome 1.
        let groups = NameGroups::merge(&frame, &[1, 0, 2, 3], |i| weights[i]);
        assert_eq!(groups, [vec![0, 2, 3], vec![1]]);
        assert_eq!(NameGroups::count(["a.exe", "A.EXE", "b.exe"]), 2);
        assert!(NameGroups::same("Chrome.exe", "chrome.EXE"));
    }
}
