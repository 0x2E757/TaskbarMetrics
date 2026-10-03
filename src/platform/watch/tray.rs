//! The watcher's notification icon: a hidden window that owns it, a timer, and a
//! menu with two items.
use crate::platform::abi::*;
use std::{cell::RefCell, ptr};

/// What the icon's window reports to its owner.
pub trait TrayEvents {
    /// The timer fired or the taskbar came back.
    fn tick(&mut self);
    /// "Restart all services" was chosen.
    fn restart(&mut self);
    /// "Close Taskbar Metrics" was chosen; the icon goes away after it.
    fn close_all(&mut self);
}

/// The texts of the menu's items, in the window's language.
pub struct MenuLabels {
    pub restart: String,
    pub close: String,
}

/// Window class of the icon's window: `--stop` and updates close it by this name.
pub const CLASS: &str = "TaskbarMetrics.Watch";
const CALLBACK: u32 = 0x8001; // WM_APP + 1
const TIMER: usize = 1;
const CLOSE: usize = 1;
const RESTART: usize = 2;

struct State {
    events: Box<dyn TrayEvents>,
    icon: NotifyIcon,
    /// Item ids and texts, top to bottom.
    menu: Vec<(usize, Vec<u16>)>,
    /// "TaskbarCreated": a new Explorer drew the taskbar.
    taskbar_created: u32,
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

pub struct TrayIcon;
impl TrayIcon {
    /// Shows the icon with `tooltip` and runs the window until it closes, calling
    /// `events` every `period_ms`.
    pub fn run(
        events: Box<dyn TrayEvents>,
        tooltip: &str,
        menu: MenuLabels,
        period_ms: u32,
    ) -> Result<()> {
        unsafe {
            let instance = GetModuleHandleW(ptr::null());
            let class = wide(CLASS);
            let window_class = WindowClass {
                size: std::mem::size_of::<WindowClass>() as u32,
                style: 0,
                procedure,
                class_extra: 0,
                window_extra: 0,
                instance,
                icon: ptr::null_mut(),
                cursor: ptr::null_mut(),
                background: ptr::null_mut(),
                menu: ptr::null(),
                name: class.as_ptr(),
                small_icon: ptr::null_mut(),
            };
            if RegisterClassExW(&window_class) == 0 {
                return Err(last_error());
            }
            // A top-level window that is never shown, so FindWindowW finds it.
            let window = CreateWindowExW(
                0,
                class.as_ptr(),
                wide("Taskbar Metrics").as_ptr(),
                0,
                0,
                0,
                0,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                ptr::null_mut(),
            );
            if window.is_null() {
                return Err(last_error());
            }
            let icon = NotifyIcon::new(window, instance, tooltip);
            icon.add();
            STATE.with(|state| {
                *state.borrow_mut() = Some(State {
                    events,
                    icon,
                    menu: vec![(RESTART, wide(&menu.restart)), (CLOSE, wide(&menu.close))],
                    taskbar_created: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
                })
            });
            SetTimer(window, TIMER, period_ms, ptr::null_mut());
            let mut message = Message::default();
            while GetMessageW(&mut message, ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            STATE.with(|state| state.borrow_mut().take());
        }
        Ok(())
    }
    /// Removes the icon and ends the loop. WM_DESTROY arrives while the state is
    /// out, so it is done here.
    fn quit(window: Raw, icon: &NotifyIcon) {
        icon.remove();
        unsafe {
            DestroyWindow(window);
            PostQuitMessage(0);
        }
    }
    /// The menu at the pointer; the id of the item chosen, 0 for none.
    fn menu(window: Raw, items: &[(usize, Vec<u16>)]) -> usize {
        unsafe {
            let menu = CreatePopupMenu();
            if menu.is_null() {
                return 0;
            }
            for (id, text) in items {
                AppendMenuW(menu, 0, *id, text.as_ptr());
            }
            let mut point = Point::default();
            GetCursorPos(&mut point);
            // Without it the menu would not close on a click elsewhere.
            SetForegroundWindow(window);
            // TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN
            let chosen = TrackPopupMenu(
                menu,
                0x100 | 0x2 | 0x20,
                point.x,
                point.y,
                0,
                window,
                ptr::null(),
            );
            PostMessageW(window, 0, 0, 0);
            DestroyMenu(menu);
            chosen as usize
        }
    }
}

unsafe extern "system" fn procedure(
    window: Raw,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    const WM_DESTROY: u32 = 0x0002;
    const WM_CLOSE: u32 = 0x0010;
    const WM_TIMER: u32 = 0x0113;
    const WM_LBUTTONUP: isize = 0x0202;
    const WM_RBUTTONUP: isize = 0x0205;
    // The state is taken out while its owner runs, so a message that arrives meanwhile
    // (the menu runs a loop of its own) finds none and is left to Windows.
    let state = STATE.with(|state| state.borrow_mut().take());
    let Some(mut state) = state else {
        return DefWindowProcW(window, message, wparam, lparam);
    };
    let result = match message {
        WM_TIMER => {
            state.events.tick();
            0
        }
        CALLBACK if matches!(lparam, WM_LBUTTONUP | WM_RBUTTONUP) => {
            match TrayIcon::menu(window, &state.menu) {
                RESTART => state.events.restart(),
                CLOSE => {
                    state.events.close_all();
                    TrayIcon::quit(window, &state.icon);
                }
                _ => {}
            }
            0
        }
        // `--stop` and updates: only the icon goes, the rest is theirs to close.
        WM_CLOSE => {
            TrayIcon::quit(window, &state.icon);
            0
        }
        WM_DESTROY => {
            state.icon.remove();
            PostQuitMessage(0);
            0
        }
        _ if message == state.taskbar_created && message != 0 => {
            state.icon.add();
            state.events.tick();
            0
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    };
    STATE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = Some(state);
        }
    });
    result
}

/// NOTIFYICONDATAW of the icon.
struct NotifyIcon(NotifyIconData);
impl NotifyIcon {
    fn new(window: Raw, instance: Raw, tooltip: &str) -> Self {
        let mut data = NotifyIconData {
            size: std::mem::size_of::<NotifyIconData>() as u32,
            window,
            id: 1,
            // NIF_MESSAGE | NIF_ICON | NIF_TIP
            flags: 0x1 | 0x2 | 0x4,
            callback: CALLBACK,
            icon: Self::image(instance),
            ..NotifyIconData::default()
        };
        for (slot, unit) in data.tip.iter_mut().zip(tooltip.encode_utf16().take(127)) {
            *slot = unit;
        }
        Self(data)
    }
    /// The executable's icon at the small-icon size of the system DPI.
    fn image(instance: Raw) -> Raw {
        unsafe {
            let dpi = GetDpiForSystem();
            // SM_CXSMICON, SM_CYSMICON
            let (width, height) = (
                GetSystemMetricsForDpi(49, dpi),
                GetSystemMetricsForDpi(50, dpi),
            );
            // The icon group 1 of the version resources; IMAGE_ICON.
            LoadImageW(instance, ptr::without_provenance(1), 1, width, height, 0)
        }
    }
    fn add(&self) {
        unsafe {
            // NIM_ADD; after a new Explorer an icon with this id may linger, so
            // NIM_MODIFY covers that case.
            if Shell_NotifyIconW(0, &self.0) == 0 {
                Shell_NotifyIconW(1, &self.0);
            }
        }
    }
    fn remove(&self) {
        unsafe {
            Shell_NotifyIconW(2, &self.0);
        }
    }
}

#[repr(C)]
struct NotifyIconData {
    size: u32,
    window: Raw,
    id: u32,
    flags: u32,
    callback: u32,
    icon: Raw,
    tip: [u16; 128],
    state: u32,
    state_mask: u32,
    info: [u16; 256],
    version: u32,
    info_title: [u16; 64],
    info_flags: u32,
    guid: Guid,
    balloon_icon: Raw,
}
impl Default for NotifyIconData {
    fn default() -> Self {
        Self {
            size: 0,
            window: ptr::null_mut(),
            id: 0,
            flags: 0,
            callback: 0,
            icon: ptr::null_mut(),
            tip: [0; 128],
            state: 0,
            state_mask: 0,
            info: [0; 256],
            version: 0,
            info_title: [0; 64],
            info_flags: 0,
            guid: Guid::from_u128(0),
            balloon_icon: ptr::null_mut(),
        }
    }
}
// WNDCLASSEXW and MSG laid out as the metrics window declares them.
#[repr(C)]
struct WindowClass {
    size: u32,
    style: u32,
    procedure: unsafe extern "system" fn(Raw, u32, usize, isize) -> isize,
    class_extra: i32,
    window_extra: i32,
    instance: Raw,
    icon: Raw,
    cursor: Raw,
    background: Raw,
    menu: *const u16,
    name: *const u16,
    small_icon: Raw,
}
#[repr(C)]
#[derive(Default)]
struct Point {
    x: i32,
    y: i32,
}
#[repr(C)]
#[derive(Default)]
struct Message {
    hwnd: Raw,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    x: i32,
    y: i32,
    private: u32,
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassExW(class: *const WindowClass) -> u16;
    #[allow(clippy::too_many_arguments)]
    fn CreateWindowExW(
        extended: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Raw,
        menu: Raw,
        instance: Raw,
        parameter: Raw,
    ) -> Raw;
    fn DefWindowProcW(window: Raw, message: u32, wparam: usize, lparam: isize) -> isize;
    fn DestroyWindow(window: Raw) -> i32;
    fn PostQuitMessage(code: i32);
    fn GetMessageW(message: *mut Message, window: Raw, first: u32, last: u32) -> i32;
    fn TranslateMessage(message: *const Message) -> i32;
    fn DispatchMessageW(message: *const Message) -> isize;
    fn PostMessageW(window: Raw, message: u32, wparam: usize, lparam: isize) -> i32;
    fn SetTimer(window: Raw, id: usize, period: u32, callback: Raw) -> usize;
    fn RegisterWindowMessageW(name: *const u16) -> u32;
    fn CreatePopupMenu() -> Raw;
    fn AppendMenuW(menu: Raw, flags: u32, id: usize, text: *const u16) -> i32;
    fn TrackPopupMenu(
        menu: Raw,
        flags: u32,
        x: i32,
        y: i32,
        reserved: i32,
        window: Raw,
        rect: *const u8,
    ) -> i32;
    fn DestroyMenu(menu: Raw) -> i32;
    fn GetCursorPos(point: *mut Point) -> i32;
    fn SetForegroundWindow(window: Raw) -> i32;
    fn GetDpiForSystem() -> u32;
    fn GetSystemMetricsForDpi(index: i32, dpi: u32) -> i32;
    fn LoadImageW(
        instance: Raw,
        name: *const u16,
        kind: u32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> Raw;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Raw;
}
#[link(name = "shell32")]
extern "system" {
    fn Shell_NotifyIconW(message: u32, data: *const NotifyIconData) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layouts_match_the_windows_sdk() {
        assert_eq!(std::mem::size_of::<NotifyIconData>(), 976);
        assert_eq!(std::mem::size_of::<WindowClass>(), 80);
        assert_eq!(std::mem::size_of::<Message>(), 48);
    }
}
