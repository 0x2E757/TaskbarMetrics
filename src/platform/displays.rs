//! Connected monitors and the choice of those that show the taskbar tiles.
use super::abi::Raw;
use crate::config::NO_MONITORS;
use std::ptr;

/// A connected monitor.
#[derive(Clone, Debug, PartialEq)]
pub struct Display {
    /// Stable across restarts: the monitor's hardware id and connection,
    /// `DEL41A8/5&2f2c1c2&0&UID4353`.
    pub id: String,
    /// Model name from the monitor, empty when it reports none.
    pub name: String,
    /// Built into the device, such as a laptop panel.
    pub internal: bool,
    pub primary: bool,
    pub width: i32,
    pub height: i32,
    left: i32,
    monitor: isize,
}

/// The monitors connected now, the main one first, the others from left to right.
pub struct Displays(Vec<Display>);
impl Displays {
    pub fn current() -> Self {
        let mut monitors: Vec<isize> = Vec::new();
        // SAFETY: the callback only pushes into the vector passed as its context.
        unsafe {
            EnumDisplayMonitors(
                ptr::null_mut(),
                ptr::null(),
                collect,
                &mut monitors as *mut Vec<isize> as isize,
            );
        }
        let targets = DisplayPaths::query();
        let mut displays: Vec<Display> = monitors
            .into_iter()
            .filter_map(|monitor| {
                let mut info = MonitorInfo {
                    size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..Default::default()
                };
                // SAFETY: `info` is a MONITORINFOEXW with its size set.
                if unsafe { GetMonitorInfoW(monitor as Raw, &mut info) } == 0 {
                    return None;
                }
                let device = text(&info.device);
                let target = targets.iter().find(|(source, _)| *source == device);
                Some(Display {
                    id: target
                        .and_then(|(_, target)| Self::id(&text(&target.path)))
                        .unwrap_or_else(|| device.trim_start_matches(r"\\.\").to_string()),
                    name: target
                        .map(|(_, target)| text(&target.name))
                        .unwrap_or_default(),
                    // DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL.
                    internal: target.is_some_and(|(_, target)| target.technology == 0x8000_0000),
                    primary: info.flags & 1 != 0,
                    width: info.monitor.right - info.monitor.left,
                    height: info.monitor.bottom - info.monitor.top,
                    left: info.monitor.left,
                    monitor,
                })
            })
            .collect();
        displays.sort_by_key(|display| (!display.primary, display.left));
        Self(displays)
    }
    /// `\\?\DISPLAY#DEL41A8#5&2f2c1c2&0&UID4353#{e6f07b5f-…}` → `DEL41A8/5&2f2c1c2&0&UID4353`.
    fn id(path: &str) -> Option<String> {
        let mut parts = path.split('#').skip(1);
        let (hardware, instance) = (parts.next()?, parts.next()?);
        (!hardware.is_empty() && !instance.is_empty() && parts.next().is_some())
            .then(|| format!("{hardware}/{instance}"))
    }
    pub fn list(&self) -> &[Display] {
        &self.0
    }
    /// The monitor that holds most of `window`.
    pub fn of_window(&self, window: isize) -> Option<&Display> {
        // SAFETY: a stale window handle only yields the nearest monitor.
        let monitor = unsafe { MonitorFromWindow(window as Raw, 2) } as isize;
        self.0.iter().find(|display| display.monitor == monitor)
    }
    /// Whether `display` shows the tiles when `chosen` lists the monitors for them.
    /// An empty choice means every monitor, [`NO_MONITORS`] none; when none of the
    /// chosen ones is connected, the main monitor shows them, so the tiles do not
    /// vanish by unplugging a monitor.
    pub fn shows(&self, chosen: &[String], display: &Display) -> bool {
        if chosen.is_empty() {
            return true;
        }
        if chosen == [NO_MONITORS] {
            return false;
        }
        if self.0.iter().any(|other| chosen.contains(&other.id)) {
            chosen.contains(&display.id)
        } else {
            display.primary
        }
    }
}

/// Monitor of each active display path, by the GDI name of its source (`\\.\DISPLAY1`).
struct DisplayPaths;
impl DisplayPaths {
    fn query() -> Vec<(String, TargetName)> {
        let mut result = Vec::new();
        // SAFETY: the buffers are sized by GetDisplayConfigBufferSizes, and every
        // device-info request carries its own size in its header.
        unsafe {
            let (mut path_count, mut mode_count) = (0u32, 0u32);
            // QDC_ONLY_ACTIVE_PATHS.
            if GetDisplayConfigBufferSizes(2, &mut path_count, &mut mode_count) != 0 {
                return result;
            }
            let mut paths = vec![PathInfo::default(); path_count as usize];
            let mut modes = vec![[0u8; 64]; mode_count as usize];
            if QueryDisplayConfig(
                2,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                ptr::null_mut(),
            ) != 0
            {
                return result;
            }
            for path in &paths[..path_count as usize] {
                let mut source = SourceName {
                    header: Header::new(
                        1,
                        std::mem::size_of::<SourceName>(),
                        path.source_adapter,
                        path.source_id,
                    ),
                    gdi: [0; 32],
                };
                let mut target = TargetName {
                    header: Header::new(
                        2,
                        std::mem::size_of::<TargetName>(),
                        path.target_adapter,
                        path.target_id,
                    ),
                    ..Default::default()
                };
                if DisplayConfigGetDeviceInfo(&mut source.header) == 0
                    && DisplayConfigGetDeviceInfo(&mut target.header) == 0
                {
                    result.push((text(&source.gdi), target));
                }
            }
        }
        result
    }
}

fn text(units: &[u16]) -> String {
    let end = units
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

unsafe extern "system" fn collect(monitor: Raw, _: Raw, _: *const Rect, context: isize) -> i32 {
    (*(context as *mut Vec<isize>)).push(monitor as isize);
    1
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[repr(C)]
#[derive(Default)]
struct MonitorInfo {
    size: u32,
    monitor: Rect,
    work: Rect,
    flags: u32,
    device: [u16; 32],
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Luid {
    low: u32,
    high: i32,
}
/// DISPLAYCONFIG_PATH_INFO, with only the ids read here named.
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct PathInfo {
    source_adapter: Luid,
    source_id: u32,
    source_rest: [u32; 2],
    target_adapter: Luid,
    target_id: u32,
    target_rest: [u32; 9],
    flags: u32,
}
#[repr(C)]
#[derive(Default)]
struct Header {
    kind: u32,
    size: u32,
    adapter: Luid,
    id: u32,
}
impl Header {
    fn new(kind: u32, size: usize, adapter: Luid, id: u32) -> Self {
        Self {
            kind,
            size: size as u32,
            adapter,
            id,
        }
    }
}
#[repr(C)]
struct SourceName {
    header: Header,
    gdi: [u16; 32],
}
#[repr(C)]
struct TargetName {
    header: Header,
    flags: u32,
    technology: u32,
    edid: [u16; 2],
    connector: u32,
    name: [u16; 64],
    path: [u16; 128],
}
impl Default for TargetName {
    fn default() -> Self {
        Self {
            header: Header::default(),
            flags: 0,
            technology: 0,
            edid: [0; 2],
            connector: 0,
            name: [0; 64],
            path: [0; 128],
        }
    }
}

#[link(name = "user32")]
extern "system" {
    fn EnumDisplayMonitors(
        hdc: Raw,
        clip: *const Rect,
        callback: unsafe extern "system" fn(Raw, Raw, *const Rect, isize) -> i32,
        context: isize,
    ) -> i32;
    fn GetMonitorInfoW(monitor: Raw, info: *mut MonitorInfo) -> i32;
    fn MonitorFromWindow(window: Raw, flags: u32) -> Raw;
    fn GetDisplayConfigBufferSizes(flags: u32, paths: *mut u32, modes: *mut u32) -> i32;
    fn QueryDisplayConfig(
        flags: u32,
        path_count: *mut u32,
        paths: *mut PathInfo,
        mode_count: *mut u32,
        modes: *mut [u8; 64],
        topology: Raw,
    ) -> i32;
    fn DisplayConfigGetDeviceInfo(request: *mut Header) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn display(id: &str, primary: bool) -> Display {
        Display {
            id: id.into(),
            name: String::new(),
            internal: false,
            primary,
            width: 1920,
            height: 1080,
            left: 0,
            monitor: 0,
        }
    }
    #[test]
    fn layouts_match_the_windows_sdk() {
        assert_eq!(std::mem::size_of::<MonitorInfo>(), 104);
        assert_eq!(std::mem::size_of::<PathInfo>(), 72);
        assert_eq!(std::mem::size_of::<SourceName>(), 84);
        assert_eq!(std::mem::size_of::<TargetName>(), 420);
    }
    #[test]
    fn ids_come_from_the_monitor_path() {
        assert_eq!(
            Displays::id(
                r"\\?\DISPLAY#DEL41A8#5&2f2c1c2&0&UID4353#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}"
            )
            .as_deref(),
            Some("DEL41A8/5&2f2c1c2&0&UID4353")
        );
        assert_eq!(Displays::id(""), None);
        assert_eq!(Displays::id(r"\\?\DISPLAY#DEL41A8"), None);
    }
    #[test]
    fn the_main_monitor_stands_in_for_disconnected_choices() {
        let (main, side) = (display("A/1", true), display("B/2", false));
        let displays = Displays(vec![main.clone(), side.clone()]);
        assert!(displays.shows(&[], &main) && displays.shows(&[], &side));
        let only_side = ["B/2".to_string()];
        assert!(!displays.shows(&only_side, &main) && displays.shows(&only_side, &side));
        let gone = ["C/3".to_string()];
        assert!(displays.shows(&gone, &main) && !displays.shows(&gone, &side));
        let both = ["C/3".to_string(), "A/1".to_string()];
        assert!(displays.shows(&both, &main) && !displays.shows(&both, &side));
        let none = [NO_MONITORS.to_string()];
        assert!(!displays.shows(&none, &main) && !displays.shows(&none, &side));
    }
}
