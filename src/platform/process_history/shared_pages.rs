//! Shared pages in the working sets of processes, split into writable data sections
//! and read-only images and files.
use super::snapshot::Process;
use crate::platform::abi::*;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

/// Shared pages of one process, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SharedBytes {
    /// Writable sections: the guest memory of a virtual machine, buffers shared with
    /// a GPU or another process. Usually mapped by one process, so it counts in full.
    pub data: u64,
    /// Read-only and executable pages: DLLs and mapped files, most of them mapped
    /// by many processes at once.
    pub readonly: u64,
}

/// Windows tells how many processes share a page for DLL pages only; for data
/// sections `QueryWorkingSetEx` reports «seven or more» even when one process maps
/// them. Pages are therefore told apart by the protection `QueryWorkingSet` lists
/// with each of them, and `MemoryRows` fits the estimate to the memory in use.
#[derive(Default)]
pub(super) struct SharedPages {
    /// `QueryWorkingSet` output: the entry count, then one entry per page.
    entries: Vec<u64>,
    scans: HashMap<(u32, u64), Scan>,
    updated: Option<Instant>,
}
/// The last scan of a process.
struct Scan {
    /// None for a process that cannot be opened: protected, or closed to the recorder.
    shared: Option<SharedBytes>,
    at: Instant,
    /// Working set minus private working set when scanned.
    shared_set: u64,
}
impl SharedPages {
    /// Ticks are 500 ms apart: an update every second is every other tick.
    const PERIOD: Duration = Duration::from_millis(900);
    const PAGE: u64 = 4096;
    /// A scan takes time in proportion to the working set. Each second only
    /// processes whose shared set changed are scanned again; the others after 10 to
    /// 19 s, by PID so that rescans spread over the seconds, and later for large sets.
    pub fn update(&mut self, processes: &[Process]) {
        if self.updated.is_some_and(|at| at.elapsed() < Self::PERIOD) {
            return;
        }
        let now = Instant::now();
        self.updated = Some(now);
        let mut scans = HashMap::with_capacity(processes.len());
        for p in processes {
            let key = (p.pid, p.created);
            let shared_set = p.working_set.saturating_sub(p.private_working_set);
            let scan = match self.scans.remove(&key) {
                Some(scan) if !Self::stale(&scan, p.pid, shared_set, now) => scan,
                _ => Scan {
                    shared: self.scan(p.pid, p.working_set),
                    at: now,
                    shared_set,
                },
            };
            scans.insert(key, scan);
        }
        self.scans = scans;
        // A virtual machine's working set needs tens of megabytes of entries, and is
        // rarely scanned: the buffer is not kept for it.
        if self.entries.len() > Self::KEPT {
            self.entries = Vec::new();
        }
    }
    /// Entries kept between updates: 8 MB, a 4 GB working set.
    const KEPT: usize = 1 << 20;
    fn stale(scan: &Scan, pid: u32, shared_set: u64, now: Instant) -> bool {
        // Large sets, such as a virtual machine's guest memory, take long to scan and
        // are scanned again on change anyway: 30 s more per gigabyte.
        let refresh =
            Duration::from_secs(10 + pid as u64 / 4 % 10 + scan.shared_set / (1 << 30) * 30);
        now.duration_since(scan.at) >= refresh
            || shared_set.abs_diff(scan.shared_set) > (4 << 20).max(scan.shared_set / 32)
    }
    /// Shared pages of a process by its last scan; none for a process that cannot be
    /// opened.
    pub fn of(&self, key: &(u32, u64)) -> Option<SharedBytes> {
        self.scans.get(key).and_then(|scan| scan.shared)
    }
    /// The largest read-only shared set of one process: its pages are distinct, so
    /// DLLs and mapped files take at least that much memory.
    pub fn readonly_floor(&self) -> u64 {
        self.scans
            .values()
            .filter_map(|scan| scan.shared)
            .map(|shared| shared.readonly)
            .max()
            .unwrap_or(0)
    }
    /// Frees the buffer, which grows to the largest working set, while no window
    /// shows the history.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    fn scan(&mut self, pid: u32, working_set: u64) -> Option<SharedBytes> {
        // PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, as the query requires.
        let process = Handle::new(unsafe { OpenProcess(0x0410, 0, pid) }).ok()?;
        // Sized for the snapshot's working set plus growth since; a working set
        // that outgrew it reports the entries it needs.
        let mut needed = working_set / Self::PAGE + 4096;
        for _ in 0..3 {
            // Only grown, never cleared: the query writes the entries it reports.
            let length = (needed as usize + 1).min(u32::MAX as usize / 8);
            if self.entries.len() < length {
                self.entries.resize(length, 0);
            }
            if unsafe {
                K32QueryWorkingSet(
                    process.0,
                    self.entries.as_mut_ptr().cast(),
                    (self.entries.len() * 8) as u32,
                )
            } != 0
            {
                return Some(self.classify());
            }
            // ERROR_BAD_LENGTH leaves the entry count in the first element.
            if unsafe { GetLastError() } != 24 {
                return None;
            }
            needed = self.entries[0] + 4096;
        }
        None
    }
    /// Sums the shared pages listed by the last `QueryWorkingSet` by protection.
    fn classify(&self) -> SharedBytes {
        let count = (self.entries[0] as usize).min(self.entries.len() - 1);
        let mut shared = SharedBytes::default();
        for entry in &self.entries[1..=count] {
            // Bit 8 is Shared; bits 0 to 2 the access: 4 read-write, 6 execute and
            // read-write. Image pages are copy-on-write and turn private when written.
            if entry >> 8 & 1 == 0 {
                continue;
            }
            match entry & 7 {
                4 | 6 => shared.data += Self::PAGE,
                _ => shared.readonly += Self::PAGE,
            }
        }
        shared
    }
}
#[link(name = "kernel32")]
extern "system" {
    fn K32QueryWorkingSet(process: Raw, buffer: Raw, size: u32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn writable_sections_count_as_data_and_images_as_readonly() {
        let pid = unsafe { GetCurrentProcessId() };
        let mut pages = SharedPages::default();
        let before = pages.scan(pid, 512 << 20).unwrap();
        // DLLs of this process are shared and read-only.
        assert!(before.readonly > 0);
        let size = 64 * 4096;
        unsafe {
            let section = CreateFileMappingW(
                -1isize as Raw,
                std::ptr::null_mut(),
                0x04,
                0,
                size as u32,
                std::ptr::null(),
            );
            assert!(!section.is_null());
            let view = MapViewOfFile(section, 0x0002 | 0x0004, 0, 0, size);
            assert!(!view.is_null());
            for page in 0..64 {
                std::ptr::write_volatile(view.cast::<u8>().add(page * 4096), 1);
            }
            let after = pages.scan(pid, 512 << 20).unwrap();
            UnmapViewOfFile(view);
            CloseHandle(section);
            assert!(after.data >= before.data + size as u64, "{after:?}");
        }
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateFileMappingW(
            file: Raw,
            attributes: Raw,
            protect: u32,
            high: u32,
            low: u32,
            name: *const u16,
        ) -> Raw;
        fn MapViewOfFile(section: Raw, access: u32, high: u32, low: u32, size: usize) -> Raw;
        fn UnmapViewOfFile(view: Raw) -> i32;
    }
}
