use super::super::abi::*;
use std::collections::HashMap;

pub(super) struct Process {
    pub pid: u32,
    pub created: u64,
    pub name: String,
    pub cpu_ticks: u64,
    pub private_working_set: u64,
    pub working_set: u64,
    pub private_bytes: u64,
}
#[derive(Default)]
pub(super) struct SnapshotReader {
    buffer: Vec<u64>,
    /// Processes and threads of the previous snapshot: the next one is sized for them
    /// instead of growing a map of thousands of threads step by step.
    sizes: (usize, usize),
}
impl SnapshotReader {
    // SystemProcessInformation's x64 fixed header is 256 bytes, followed by
    // 80-byte SYSTEM_THREAD_INFORMATION entries. SDK offsets: ImageName 56,
    // UniqueProcessId 80, WorkingSetSize 144, PrivatePageCount 200; the thread's
    // ClientId 40 (UniqueThread 48). The other fields are named where they are read.
    pub fn read(&mut self) -> Result<(Vec<Process>, HashMap<u32, u32>)> {
        self.buffer.resize(self.buffer.len().max(128 * 1024), 0);
        let mut used = 0;
        loop {
            let status = unsafe {
                NtQuerySystemInformation(
                    5,
                    self.buffer.as_mut_ptr().cast(),
                    (self.buffer.len() * 8) as u32,
                    &mut used,
                )
            };
            if status >= 0 {
                break;
            }
            if status != 0xc0000004u32 as i32 || used > 128 * 1024 * 1024 {
                return Err(status);
            }
            self.buffer.resize((used as usize + 65536).div_ceil(8), 0);
        }
        let bytes =
            unsafe { std::slice::from_raw_parts(self.buffer.as_ptr().cast::<u8>(), used as usize) };
        let mut processes = Vec::with_capacity(self.sizes.0);
        let mut threads = HashMap::with_capacity(self.sizes.1);
        let mut offset = 0;
        loop {
            let header = bytes.get(offset..offset + 256).ok_or(E_FAIL)?;
            let u32_at = |i| u32::from_le_bytes(header[i..i + 4].try_into().unwrap());
            let u64_at = |i| u64::from_le_bytes(header[i..i + 8].try_into().unwrap());
            let next = u32_at(0) as usize;
            let count = u32_at(4) as usize;
            let pid = u64_at(80) as u32;
            let name_len = u16::from_le_bytes(header[56..58].try_into().unwrap()) as usize;
            let name_ptr = u64_at(64) as usize;
            let base = bytes.as_ptr() as usize;
            let name = name_ptr
                .checked_sub(base)
                .and_then(|start| bytes.get(start..start.checked_add(name_len)?))
                .map(|b| {
                    String::from_utf16_lossy(
                        &b.chunks_exact(2)
                            .map(|c| u16::from_le_bytes([c[0], c[1]]))
                            .collect::<Vec<_>>(),
                    )
                })
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("PID {pid}"));
            let end = if next == 0 {
                bytes.len()
            } else {
                offset.checked_add(next).ok_or(E_FAIL)?
            };
            if end > bytes.len() || end < offset + 256 || count > (end - offset - 256) / 80 {
                return Err(E_FAIL);
            }
            for i in 0..count {
                let start = offset + 256 + i * 80 + 48;
                let tid = u64::from_le_bytes(bytes[start..start + 8].try_into().unwrap()) as u32;
                threads.insert(tid, pid);
            }
            if pid != 0 {
                processes.push(Process {
                    pid,
                    created: u64_at(32),
                    name,
                    cpu_ticks: u64_at(40).saturating_add(u64_at(48)),
                    private_working_set: u64_at(8),
                    working_set: u64_at(144),
                    private_bytes: u64_at(200),
                });
            }
            if next == 0 {
                break;
            }
            offset = end;
        }
        self.sizes = (processes.len(), threads.len());
        Ok((processes, threads))
    }
}
#[link(name = "ntdll")]
extern "system" {
    fn NtQuerySystemInformation(class: u32, buffer: Raw, size: u32, used: *mut u32) -> i32;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_snapshot_contains_this_process_and_its_threads() {
        let mut reader = SnapshotReader::default();
        let (processes, threads) = reader.read().unwrap();
        let pid = unsafe { GetCurrentProcessId() };
        let process = processes.iter().find(|p| p.pid == pid).unwrap();
        assert!(process.created > 116444736000000000);
        assert!(process.name.ends_with(".exe"));
        assert!(process.working_set > 0);
        assert!(process.private_working_set <= process.working_set);
        assert!(threads.values().any(|&owner| owner == pid));
    }
}
