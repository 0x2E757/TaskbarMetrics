//! RAM in use split into processes and the memory outside them, so that the rows of
//! a frame add up to what the RAM tile shows.
use super::store::{Identity, Sample};
use crate::platform::{pdh::PdhCounter, providers::PhysicalMemory};
use std::sync::Arc;

/// Resident memory outside the working sets of processes, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SystemMemory {
    /// Total minus available physical memory, as the RAM tile computes it.
    pub used: u64,
    pub nonpaged_pool: u64,
    pub paged_pool: u64,
    /// Resident driver images and kernel code.
    pub drivers: u64,
    /// Dirty pages taken from working sets that wait to be written to disk.
    pub modified: u64,
    /// The cache manager's working set of file data.
    pub file_cache: u64,
}

/// `SystemMemory` from the memory counters, all read by one PDH query.
pub(super) struct SystemMemoryCounters(PdhCounter);
impl SystemMemoryCounters {
    pub fn new() -> Self {
        Self(PdhCounter::several(&[
            r"\Memory\Pool Nonpaged Bytes",
            r"\Memory\Pool Paged Resident Bytes",
            r"\Memory\System Driver Resident Bytes",
            r"\Memory\System Code Resident Bytes",
            r"\Memory\Modified Page List Bytes",
            r"\Memory\System Cache Resident Bytes",
        ]))
    }
    /// None until the query works: a split with missing parts would not add up.
    pub fn sample(&mut self) -> Option<SystemMemory> {
        self.0.collect().ok()?;
        let mut parts = [0u64; 6];
        for (index, part) in parts.iter_mut().enumerate() {
            *part = self.0.collected_value(index)? as u64;
        }
        let [nonpaged_pool, paged_pool, drivers, code, modified, file_cache] = parts;
        Some(SystemMemory {
            used: PhysicalMemory::used_bytes()?,
            nonpaged_pool,
            paged_pool,
            drivers: drivers + code,
            modified,
            file_cache,
        })
    }
}

/// Rows that stand for the memory outside processes. They use PID 0, which no
/// process has in a snapshot (the idle process is left out), and are shown on the
/// RAM page only.
pub(crate) struct MemoryRows {
    identities: [Arc<Identity>; 6],
}
impl MemoryRows {
    pub const PID: u32 = 0;
    /// Row names, translated by the window. The last row holds what is left of the
    /// memory in use: shared pages of processes not attributed to one of them.
    pub const NAMES: [&'static str; 6] = [
        "Kernel: nonpaged pool",
        "Kernel: paged pool",
        "Drivers and kernel code",
        "Modified pages",
        "System file cache",
        "Shared memory",
    ];
    pub fn new() -> Self {
        Self {
            identities: std::array::from_fn(|index| {
                Arc::new(Identity {
                    pid: Self::PID,
                    created: index as u64 + 1,
                    name: Self::NAMES[index].into(),
                })
            }),
        }
    }
    /// Appends the rows to the processes of a frame. Kernel parts count as private
    /// memory (nothing else maps them), file cache and the rest as shared.
    ///
    /// The shared memory of processes is an estimate: their writable shared pages,
    /// counted in full although a section may be mapped twice or by several
    /// processes. It is scaled down to fit what the memory in use leaves after
    /// private pages, the other parts and `reserved` (the least DLLs and mapped
    /// files take), so the rows always add up to the memory in use.
    pub fn append(&self, samples: &mut Vec<Sample>, memory: &SystemMemory, reserved: u64) {
        let private: u64 = samples.iter().filter_map(Sample::private_working_set).sum();
        let parts = [
            memory.nonpaged_pool,
            memory.paged_pool,
            memory.drivers,
            memory.modified,
            memory.file_cache,
        ];
        let left = memory
            .used
            .saturating_sub(private)
            .saturating_sub(parts.iter().sum());
        let room = left.saturating_sub(reserved);
        let shared: u64 = samples.iter().filter_map(Sample::shared).sum();
        if shared > room {
            for sample in samples.iter_mut() {
                let fitted = sample
                    .shared()
                    .map(|bytes| (bytes as u128 * room as u128 / shared as u128) as u64);
                sample.set_shared(fitted);
            }
        }
        let rest = left - samples.iter().filter_map(Sample::shared).sum::<u64>();
        for (index, bytes) in parts.into_iter().chain([rest]).enumerate() {
            let kernel = index < 4;
            let mut row = Sample::new(
                self.identities[index].clone(),
                None,
                None,
                [kernel.then_some(bytes), None, None],
                None,
            );
            if !kernel {
                row.set_shared(Some(bytes));
            }
            samples.push(row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rows_add_up_to_the_memory_in_use() {
        let mut process = Sample::new(
            Arc::new(Identity {
                pid: 7,
                created: 1,
                name: "p.exe".into(),
            }),
            None,
            None,
            [Some(1000), Some(5000), None],
            None,
        );
        process.set_shared(Some(500));
        let mut samples = vec![process];
        let memory = SystemMemory {
            used: 10_000,
            nonpaged_pool: 100,
            paged_pool: 200,
            drivers: 300,
            modified: 400,
            file_cache: 600,
        };
        let total = |samples: &[Sample]| -> u64 {
            samples
                .iter()
                .map(|s| s.private_working_set().unwrap_or(0) + s.shared().unwrap_or(0))
                .sum()
        };
        let rows = MemoryRows::new();
        let mut fits = samples.clone();
        rows.append(&mut fits, &memory, 1000);
        assert_eq!(total(&fits), 10_000);
        assert_eq!(fits.len(), 7);
        assert_eq!(fits[0].shared(), Some(500));
        assert_eq!(fits[6].shared(), Some(10_000 - 1000 - 1600 - 500));
        assert!(fits[1..].iter().all(|s| s.identity.pid == MemoryRows::PID));
        assert_eq!(fits[5].private_working_set(), None);
        // 7400 left after private pages and the parts, 1400 of it reserved: an
        // estimate of 12 000 shrinks to the 6000 of room.
        samples[0].set_shared(Some(12_000));
        rows.append(&mut samples, &memory, 1400);
        assert_eq!(total(&samples), 10_000);
        assert_eq!(samples[0].shared(), Some(6000));
        assert_eq!(samples[6].shared(), Some(1400));
    }
}
