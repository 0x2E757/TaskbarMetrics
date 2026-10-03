//! System values beside the charts: the process count, uptime, the page file and
//! the processor's current frequency.

use super::{abi::Raw, pdh::PdhCounter};
use std::time::Duration;

/// Processes running and the time since the system started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemActivity {
    pub processes: u32,
    pub uptime: Duration,
}

impl SystemActivity {
    pub fn read() -> Option<Self> {
        let mut info = PerformanceInformation {
            size: std::mem::size_of::<PerformanceInformation>() as u32,
            ..Default::default()
        };
        // SAFETY: `info` is a PERFORMANCE_INFORMATION of the size it declares.
        if unsafe { K32GetPerformanceInfo(&mut info, info.size) } == 0 {
            return None;
        }
        Some(Self {
            processes: info.processes,
            // SAFETY: no arguments.
            uptime: Duration::from_millis(unsafe { GetTickCount64() }),
        })
    }
}

/// Page files in use and their size, in bytes, summed over all page files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageFile {
    pub used: u64,
    pub size: u64,
}

impl PageFile {
    /// None without a page file or when the system does not answer.
    pub fn read() -> Option<Self> {
        const PAGE_FILES: u32 = 18;
        const LENGTH_MISMATCH: i32 = 0xC000_0004_u32 as i32;
        let mut buffer = vec![0u64; 512];
        loop {
            let bytes = (buffer.len() * 8) as u32;
            let mut needed = 0;
            // SAFETY: the buffer holds `bytes` bytes, 8-aligned for the records.
            let status = unsafe {
                NtQuerySystemInformation(PAGE_FILES, buffer.as_mut_ptr().cast(), bytes, &mut needed)
            };
            match status {
                0 => return Self::parse(&buffer, needed as usize),
                LENGTH_MISMATCH if buffer.len() < 1 << 16 => {
                    buffer.resize(buffer.len() * 2, 0);
                }
                _ => return None,
            }
        }
    }

    /// SYSTEM_PAGEFILE_INFORMATION records (x64): NextEntryOffset +0, TotalSize +4
    /// and TotalInUse +8, both in 4 KiB pages.
    fn parse(buffer: &[u64], length: usize) -> Option<Self> {
        let bytes: Vec<u8> = buffer.iter().flat_map(|w| w.to_le_bytes()).collect();
        let bytes = bytes.get(..length)?;
        let dword = |at: usize| {
            bytes
                .get(at..at + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let (mut used, mut size, mut at) = (0u64, 0u64, 0usize);
        loop {
            size += u64::from(dword(at + 4)?) * 4096;
            used += u64::from(dword(at + 8)?) * 4096;
            match dword(at)? {
                0 => break,
                next => at += next as usize,
            }
        }
        (size > 0).then_some(Self { used, size })
    }
}

/// The processor's current frequency, as Task Manager computes its speed: the base
/// frequency times the share of it the processor runs at, which turbo lifts past 100 %.
pub struct CpuFrequency {
    counter: PdhCounter,
}

impl Default for CpuFrequency {
    fn default() -> Self {
        Self {
            counter: PdhCounter::several(&[
                r"\Processor Information(_Total)\Processor Frequency",
                r"\Processor Information(_Total)\% Processor Performance",
            ]),
        }
    }
}

impl CpuFrequency {
    /// MHz; none until the second sample.
    pub fn sample(&mut self) -> Option<f64> {
        self.counter.collect().ok()?;
        let base = self.counter.collected_value(0)?;
        let performance = self.counter.collected_value(1)?;
        Some(base * performance / 100.0).filter(|mhz| *mhz > 0.0)
    }
}

/// PERFORMANCE_INFORMATION (x64).
#[repr(C)]
#[derive(Default)]
struct PerformanceInformation {
    size: u32,
    commit_total: usize,
    commit_limit: usize,
    commit_peak: usize,
    physical_total: usize,
    physical_available: usize,
    system_cache: usize,
    kernel_total: usize,
    kernel_paged: usize,
    kernel_nonpaged: usize,
    page_size: usize,
    handles: u32,
    processes: u32,
    threads: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn K32GetPerformanceInfo(info: *mut PerformanceInformation, size: u32) -> i32;
    fn GetTickCount64() -> u64;
}

#[link(name = "ntdll")]
extern "system" {
    fn NtQuerySystemInformation(class: u32, buffer: Raw, size: u32, used: *mut u32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_files_are_summed_over_their_records() {
        // Two records: 1 GiB with 256 MiB in use, then 512 MiB with none.
        let mut bytes = [0u8; 64];
        bytes[0..4].copy_from_slice(&32u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&262_144u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&65_536u32.to_le_bytes());
        bytes[36..40].copy_from_slice(&131_072u32.to_le_bytes());
        let words: Vec<u64> = bytes
            .chunks(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let file = PageFile::parse(&words, 64).unwrap();
        assert_eq!((file.used, file.size), (1 << 28, (1 << 30) + (1 << 29)));
        assert_eq!(PageFile::parse(&[0; 4], 0), None);
    }

    #[test]
    fn the_system_answers() {
        let activity = SystemActivity::read().unwrap();
        assert!(activity.processes > 0 && activity.uptime.as_secs() > 0);
    }
}
