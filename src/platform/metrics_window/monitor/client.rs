use crate::platform::{
    abi::*,
    process_history::{
        store::{Frame, Identities, IoBytes},
        wire::Packet,
    },
};

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Default)]
pub struct Snapshot {
    pub frames: BTreeMap<u64, Arc<Frame>>,
    pub enabled: bool,
    pub connected: bool,
    pub status: String,
    pub revision: u64,
    /// I/O of each process since the recorder started.
    pub io_totals: Arc<std::collections::HashMap<(u32, u64), IoBytes>>,
}

pub struct HistoryClient {
    pub state: Arc<Mutex<Snapshot>>,
    stop: Arc<AtomicBool>,
}

impl HistoryClient {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(Snapshot::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let output = state.clone();
        let cancelled = stop.clone();
        std::thread::spawn(move || {
            while !cancelled.load(Ordering::Relaxed) {
                let result = (|| -> std::io::Result<()> {
                    let mut pid = 0;
                    unsafe {
                        let window = FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null());
                        GetWindowThreadProcessId(window, &mut pid);
                    }
                    let file = std::fs::File::open(format!(
                        r"\\.\pipe\TaskbarMetrics.History.{pid}.Live"
                    ))?;
                    let mut input = std::io::BufReader::with_capacity(65536, file);
                    let mut identities = Identities::default();
                    {
                        let mut s = output.lock().unwrap_or_else(|e| e.into_inner());
                        s.frames.clear();
                    }
                    while !cancelled.load(Ordering::Relaxed) {
                        let packet = Packet::read_shared(&mut input, &mut identities)?;
                        let mut s = output.lock().unwrap_or_else(|e| e.into_inner());
                        // The recorder repeats its state every 500 ms; an unchanged
                        // packet must not make the window redraw.
                        let mut changed = !packet.frames.is_empty()
                            || !s.connected
                            || s.enabled != packet.enabled
                            || s.status != packet.status;
                        for frame in packet.frames {
                            s.frames.insert(frame.bucket, frame);
                        }
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64
                            / 500;
                        let count = s.frames.len();
                        s.frames
                            .retain(|bucket, _| bucket.saturating_add(super::chart::WINDOW) > now);
                        changed |= s.frames.len() != count;
                        identities.prune();
                        s.io_totals = Arc::new(packet.io_totals.into_iter().collect());
                        s.enabled = packet.enabled;
                        s.status = packet.status;
                        s.connected = true;
                        if changed {
                            s.revision += 1;
                        }
                    }
                    Ok(())
                })();
                if result.is_err() {
                    let mut s = output.lock().unwrap_or_else(|e| e.into_inner());
                    s.connected = false;
                    s.revision += 1;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        });
        Self { state, stop }
    }
}

impl Drop for HistoryClient {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[link(name = "user32")]
extern "system" {
    fn FindWindowW(class: *const u16, title: *const u16) -> Raw;
    fn GetWindowThreadProcessId(window: Raw, pid: *mut u32) -> u32;
}
