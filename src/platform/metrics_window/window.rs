use super::{window_memory::Placement, *};
use crate::platform::app_icon::AppIcon;

#[repr(C)]
#[derive(Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
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

/// `COPYDATASTRUCT.dwData` of a request to show a device in the running window.
pub(super) const DEVICE_REQUEST: usize = 0x544D_4445;
type Refresh = dyn FnMut() -> Result<()>;
thread_local! {
    static TICK: std::cell::Cell<Option<*mut Refresh>> = const { std::cell::Cell::new(None) };
    static TICK_ERROR: std::cell::Cell<Option<Hr>> = const { std::cell::Cell::new(None) };
    /// Frame of the window when it was closed.
    static CLOSED: std::cell::Cell<Option<Placement>> = const { std::cell::Cell::new(None) };
    /// Smallest visible frame the user can size the window to, in 96-DPI pixels.
    static MINIMUM: std::cell::Cell<(i32, i32)> = const { std::cell::Cell::new((0, 0)) };
}
/// MINMAXINFO: points as `[x, y]`.
#[repr(C)]
struct MinMaxInfo {
    reserved: [i32; 2],
    max_size: [i32; 2],
    max_position: [i32; 2],
    min_track: [i32; 2],
    max_track: [i32; 2],
}
/// MONITORINFO.
#[repr(C)]
#[derive(Default)]
struct MonitorInfo {
    size: u32,
    monitor: Rect,
    work: Rect,
    flags: u32,
}
/// WINDOWPLACEMENT; the normal frame is in workspace coordinates both ways.
#[repr(C)]
#[derive(Default)]
struct WindowPlacement {
    length: u32,
    flags: u32,
    show: u32,
    minimized: [i32; 2],
    maximized: [i32; 2],
    normal: Rect,
}
/// Refresh callback of `NativeWindow::run`, reachable from the window procedure:
/// while the window is resized or moved, the modal loop of Windows dispatches
/// WM_TIMER there instead of returning to `run`, and the content would freeze.
struct Tick<'a>(std::marker::PhantomData<&'a mut ()>);
impl<'a> Tick<'a> {
    fn install(refresh: &'a mut (dyn FnMut() -> Result<()> + 'a)) -> Self {
        // SAFETY: only the lifetime is erased; `Drop` removes the pointer before it ends.
        let raw: *mut Refresh = unsafe {
            std::mem::transmute::<*mut (dyn FnMut() -> Result<()> + 'a), *mut Refresh>(refresh)
        };
        TICK.with(|tick| tick.set(Some(raw)));
        Self(std::marker::PhantomData)
    }
    /// Runs the callback unless it is already running further up the stack.
    fn fire() -> Result<()> {
        let Some(refresh) = TICK.with(|tick| tick.take()) else {
            return Ok(());
        };
        // SAFETY: installed by a live `Tick`; taking it out prevents re-entrant calls.
        let result = unsafe { (*refresh)() };
        TICK.with(|tick| tick.set(Some(refresh)));
        result
    }
    /// Error of a refresh that ran inside a modal loop.
    fn failure() -> Result<()> {
        TICK_ERROR.with(|error| error.take()).map_or(Ok(()), Err)
    }
}
impl Drop for Tick<'_> {
    fn drop(&mut self) {
        TICK.with(|tick| tick.set(None));
    }
}

pub(super) struct NativeWindow(Raw);
impl NativeWindow {
    pub(super) fn new(dark: bool) -> Result<Self> {
        Self::named(
            dark,
            "TaskbarMetrics.Editor",
            "Taskbar Metrics · Tile appearance",
            900,
            800,
        )
    }
    pub(super) fn named(
        dark: bool,
        class_name: &str,
        title: &str,
        width: i32,
        height: i32,
    ) -> Result<Self> {
        unsafe {
            let name = wide(class_name);
            let instance = GetModuleHandleW(ptr::null());
            let class = WindowClass {
                size: std::mem::size_of::<WindowClass>() as u32,
                style: 0,
                procedure: Self::procedure,
                class_extra: 0,
                window_extra: 0,
                instance,
                icon: ptr::null_mut(),
                cursor: LoadCursorW(ptr::null_mut(), 32512usize as *const u16),
                background: ptr::null_mut(),
                menu: ptr::null(),
                name: name.as_ptr(),
                small_icon: ptr::null_mut(),
            };
            if RegisterClassExW(&class) == 0 {
                return Err(last_error());
            }
            let dpi = GetDpiForSystem() as i32;
            let hwnd = CreateWindowExW(
                0,
                name.as_ptr(),
                wide(title).as_ptr(),
                0x02cf0000,
                i32::MIN,
                i32::MIN,
                width * dpi / 96,
                height * dpi / 96,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                ptr::null_mut(),
            );
            if hwnd.is_null() {
                return Err(last_error());
            }
            let dark = i32::from(dark);
            let _ = DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as Raw, 4);
            Self::place(hwnd, width * dpi / 96, height * dpi / 96);
            Ok(Self(hwnd))
        }
    }
    /// A new window opens in the middle of the primary monitor's work area, no
    /// larger than it. The size asked for is the visible frame: Windows 11 adds
    /// invisible resize borders around it, which DWM leaves out of the extended
    /// frame bounds.
    unsafe fn place(hwnd: Raw, width: i32, height: i32) {
        let (mut outer, mut frame, mut area) = (Rect::default(), Rect::default(), Rect::default());
        // SPI_GETWORKAREA: the primary monitor without the taskbar.
        if GetWindowRect(hwnd, &mut outer) == 0
            || DwmGetWindowAttribute(hwnd, 9, &mut frame as *mut Rect as Raw, 16) < 0
            || SystemParametersInfoW(0x30, 0, &mut area as *mut Rect as Raw, 0) == 0
        {
            return;
        }
        let border_width = (outer.right - outer.left) - (frame.right - frame.left);
        let border_height = (outer.bottom - outer.top) - (frame.bottom - frame.top);
        let width = width.min(area.right - area.left);
        let height = height.min(area.bottom - area.top);
        // SWP_NOZORDER | SWP_NOACTIVATE.
        SetWindowPos(
            hwnd,
            ptr::null_mut(),
            area.left + (area.right - area.left - width) / 2 - (frame.left - outer.left),
            area.top + (area.bottom - area.top - height) / 2 - (frame.top - outer.top),
            width + border_width,
            height + border_height,
            0x14,
        );
    }
    /// The window cannot be sized below `width`×`height` of visible frame (96-DPI
    /// pixels), nor above the work area of its monitor when that is smaller.
    pub(super) fn set_minimum(&self, width: i32, height: i32) {
        MINIMUM.with(|minimum| minimum.set((width, height)));
    }
    /// The outer size for `MINIMUM` at the window's DPI: the invisible resize
    /// borders come on top of the visible frame.
    unsafe fn minimum_track(hwnd: Raw) -> Option<(i32, i32)> {
        let (width, height) = MINIMUM.with(std::cell::Cell::get);
        if width <= 0 || height <= 0 {
            return None;
        }
        let dpi = GetDpiForWindow(hwnd).max(96) as i32;
        let (mut width, mut height) = (width * dpi / 96, height * dpi / 96);
        let mut monitor = MonitorInfo {
            size: std::mem::size_of::<MonitorInfo>() as u32,
            ..Default::default()
        };
        // MONITOR_DEFAULTTONEAREST.
        if GetMonitorInfoW(MonitorFromWindow(hwnd, 2), &mut monitor) != 0 {
            width = width.min(monitor.work.right - monitor.work.left);
            height = height.min(monitor.work.bottom - monitor.work.top);
        }
        let (mut outer, mut frame) = (Rect::default(), Rect::default());
        if GetWindowRect(hwnd, &mut outer) != 0
            && DwmGetWindowAttribute(hwnd, 9, &mut frame as *mut Rect as Raw, 16) >= 0
        {
            width += (outer.right - outer.left) - (frame.right - frame.left);
            height += (outer.bottom - outer.top) - (frame.bottom - frame.top);
        }
        Some((width, height))
    }
    pub(super) fn raw(&self) -> Raw {
        self.0
    }
    pub(super) fn chrome(&self) -> WindowChrome {
        WindowChrome(self.0)
    }
    pub(super) fn attach(&self, child: Raw) {
        unsafe {
            SetWindowLongPtrW(self.0, -21, child as isize);
            Self::resize(self.0);
        }
    }
    pub(super) fn show(&self) {
        unsafe {
            ShowWindow(self.0, 5);
        }
    }
    pub(super) fn show_maximized(&self) {
        unsafe {
            ShowWindow(self.0, 3);
        }
    }
    /// Frame of the window as it was closed; None while it is open.
    pub(super) fn closed_placement() -> Option<Placement> {
        CLOSED.with(|closed| closed.get())
    }
    unsafe fn placement_of(hwnd: Raw) -> Option<Placement> {
        let mut placement = WindowPlacement {
            length: std::mem::size_of::<WindowPlacement>() as u32,
            ..Default::default()
        };
        if GetWindowPlacement(hwnd, &mut placement) == 0 {
            return None;
        }
        // SW_SHOWMAXIMIZED, or minimized from maximized (WPF_RESTORETOMAXIMIZED).
        let maximized = placement.show == 3 || (placement.show == 2 && placement.flags & 2 != 0);
        let normal = placement.normal;
        Some(Placement {
            maximized,
            left: normal.left,
            top: normal.top,
            right: normal.right,
            bottom: normal.bottom,
        })
    }
    /// Moves the hidden window back to `placement`; false when that frame is on no
    /// monitor any more, and the window stays centred.
    pub(super) fn restore(&self, placement: &Placement) -> bool {
        let normal = Rect {
            left: placement.left,
            top: placement.top,
            right: placement.right,
            bottom: placement.bottom,
        };
        unsafe {
            // MONITOR_DEFAULTTONULL.
            if MonitorFromRect(&normal, 0).is_null() {
                return false;
            }
            let placement = WindowPlacement {
                length: std::mem::size_of::<WindowPlacement>() as u32,
                // SW_HIDE: `show` or `show_maximized` reveals it.
                show: 0,
                normal,
                ..Default::default()
            };
            SetWindowPlacement(self.0, &placement) != 0
        }
    }
    /// Frame period of a window that follows the pointer: timers fire on the 15.6 ms
    /// system tick, so 15 ms is every tick (64 Hz) while 16 ms waits for two (32 Hz).
    pub(super) const POINTER_FRAME: u32 = 15;
    /// Frame period of a window that only polls its controls (32 Hz).
    pub(super) const POLL_FRAME: u32 = 16;
    /// Runs the message loop, calling `refresh` every `period` ms.
    pub(super) fn run(
        &self,
        native: &Com,
        period: u32,
        mut refresh: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        let _tick = Tick::install(&mut refresh);
        unsafe {
            if SetTimer(self.0, 1, period, ptr::null_mut()) == 0 {
                return Err(last_error());
            }
            let mut message = Message::default();
            loop {
                Tick::failure()?;
                let status = GetMessageW(&mut message, ptr::null_mut(), 0, 0);
                if status == -1 {
                    return Err(last_error());
                }
                if status == 0 {
                    return Ok(());
                }
                // Esc returns to live; Ctrl+F focuses the process search.
                if message.message == 0x0100
                    && (message.wparam == 27 || (message.wparam == 0x46 && GetKeyState(0x11) < 0))
                {
                    super::monitor::KEY
                        .store(message.wparam as u32, std::sync::atomic::Ordering::Relaxed);
                }
                if message.hwnd == self.0 && message.message == 0x0113 && message.wparam == 1 {
                    Tick::fire()?;
                    continue;
                }
                let pre: unsafe extern "system" fn(Raw, *const Message, *mut i32) -> Hr =
                    native.slot(5);
                let mut handled = 0;
                check(pre(native.raw(), &message, &mut handled))?;
                if handled == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
    }
    pub(super) fn drain() {
        unsafe {
            let mut message = Message::default();
            while PeekMessageW(&mut message, ptr::null_mut(), 0, 0, 1) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    unsafe fn resize(hwnd: Raw) {
        let child = GetWindowLongPtrW(hwnd, -21) as Raw;
        if !child.is_null() {
            let mut rect = Rect::default();
            GetClientRect(hwnd, &mut rect);
            SetWindowPos(child, ptr::null_mut(), 0, 0, rect.right, rect.bottom, 0x54);
        }
    }
    unsafe extern "system" fn procedure(
        hwnd: Raw,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        match message {
            // WM_COPYDATA from another launch: the device id to show, parsed later.
            0x004A => {
                #[repr(C)]
                struct CopyData {
                    kind: usize,
                    size: u32,
                    data: *const u8,
                }
                let data = &*(lparam as *const CopyData);
                if data.kind != DEVICE_REQUEST || data.data.is_null() || data.size > 256 {
                    return 0;
                }
                let bytes = std::slice::from_raw_parts(data.data, data.size as usize);
                if let (Ok(text), Ok(mut request)) =
                    (std::str::from_utf8(bytes), super::monitor::REQUEST.lock())
                {
                    *request = Some(text.to_owned());
                }
                1
            }
            // A minimized window keeps the island's size: shrinking it to 0×0 would
            // re-lay out the whole page twice for nothing.
            0x0005 => {
                if wparam != 1 {
                    Self::resize(hwnd);
                }
                0
            }
            // WM_TIMER reaches the procedure only from modal size/move loops.
            0x0113 if wparam == 1 => {
                if let Err(error) = Tick::fire() {
                    TICK_ERROR.with(|failure| failure.set(Some(error)));
                }
                0
            }
            0x0010 => {
                CLOSED.with(|closed| closed.set(Self::placement_of(hwnd)));
                ShowWindow(hwnd, 0);
                PostQuitMessage(0);
                0
            }
            // WM_GETMINMAXINFO: the smallest frame the window was given.
            0x0024 => {
                if let Some((width, height)) = Self::minimum_track(hwnd) {
                    (*(lparam as *mut MinMaxInfo)).min_track = [width, height];
                }
                0
            }
            // WM_DPICHANGED: icons for the new scale, then the suggested frame.
            0x02e0 => {
                WindowChrome(hwnd).icon_for((wparam & 0xFFFF) as u32);
                let rect = &*(lparam as *const Rect);
                SetWindowPos(
                    hwnd,
                    ptr::null_mut(),
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    0x14,
                );
                0
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }
}
/// System title bar tinted like the app background, with the app logo as the window icon.
#[derive(Clone, Copy)]
pub(super) struct WindowChrome(Raw);
impl WindowChrome {
    /// `caption` and `text` are `#RRGGBB` colors.
    pub(super) fn apply(self, dark: bool, caption: &str, text: &str) {
        let colorref = |hex: &str| {
            let value = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
            ((value & 0xFF) << 16) | (value & 0xFF00) | ((value >> 16) & 0xFF)
        };
        let dark = i32::from(dark);
        unsafe {
            let _ = DwmSetWindowAttribute(self.0, 20, &dark as *const _ as Raw, 4);
            for (attribute, color) in [(35, colorref(caption)), (36, colorref(text))] {
                let _ = DwmSetWindowAttribute(self.0, attribute, &color as *const _ as Raw, 4);
            }
        }
    }
    /// Nothing of the window is on screen: it is minimized, or cloaked on another
    /// virtual desktop (`DWMWA_CLOAKED`).
    pub(super) fn hidden(self) -> bool {
        let mut cloaked = 0u32;
        unsafe {
            IsIconic(self.0) != 0
                || (DwmGetWindowAttribute(self.0, 14, &mut cloaked as *mut u32 as Raw, 4) >= 0
                    && cloaked != 0)
        }
    }
    pub(super) fn icon(self) {
        // SAFETY: plain query of the window's DPI.
        self.icon_for(unsafe { GetDpiForWindow(self.0) });
    }
    /// Physical pixels per XAML pixel at the window's DPI.
    pub(super) fn scale(self) -> f64 {
        // SAFETY: plain query of the window's DPI.
        unsafe { GetDpiForWindow(self.0) }.max(96) as f64 / 96.0
    }
    /// Drawn for `dpi`, so neither icon is scaled: the taskbar shows the big one at
    /// 24 px per 96 DPI, the title bar the small one at 16. The replaced ones are freed.
    pub(super) fn icon_for(self, dpi: u32) {
        let dpi = dpi.max(96) as i32;
        for (kind, size) in [(0usize, 16 * dpi / 96), (1usize, 24 * dpi / 96)] {
            // SAFETY: the window owns the icons it is given until they are replaced.
            unsafe {
                let icon = WindowIcon::create(size);
                if !icon.is_null() {
                    let previous = SendMessageW(self.0, 0x0080, kind, icon as isize) as Raw;
                    if !previous.is_null() {
                        DestroyIcon(previous);
                    }
                }
            }
        }
    }
}

#[repr(C)]
struct IconInfoRaw {
    icon: i32,
    x: u32,
    y: u32,
    mask: Raw,
    color: Raw,
}

/// The logo as an icon handle of one size.
struct WindowIcon;
impl WindowIcon {
    unsafe fn create(size: i32) -> Raw {
        #[repr(C)]
        struct BitmapInfo {
            size: u32,
            width: i32,
            height: i32,
            planes: u16,
            bits: u16,
            compression: u32,
            image: u32,
            x: i32,
            y: i32,
            used: u32,
            important: u32,
            colors: [u32; 1],
        }
        let info = BitmapInfo {
            size: 40,
            width: size,
            height: -size,
            planes: 1,
            bits: 32,
            compression: 0,
            image: 0,
            x: 0,
            y: 0,
            used: 0,
            important: 0,
            colors: [0],
        };
        let mut bits: *mut u32 = ptr::null_mut();
        let color = CreateDIBSection(
            ptr::null_mut(),
            &info as *const _ as Raw,
            0,
            &mut bits as *mut _ as *mut Raw,
            ptr::null_mut(),
            0,
        );
        if color.is_null() || bits.is_null() {
            return ptr::null_mut();
        }
        let pixels = AppIcon::pixels(size as u32);
        ptr::copy_nonoverlapping(pixels.as_ptr(), bits, pixels.len());
        let mask = CreateBitmap(size, size, 1, 1, ptr::null());
        let icon = CreateIconIndirect(&IconInfoRaw {
            icon: 1,
            x: 0,
            y: 0,
            mask,
            color,
        });
        DeleteObject(mask);
        DeleteObject(color);
        icon
    }
}

impl Drop for NativeWindow {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn SetTimer(window: Raw, id: usize, milliseconds: u32, callback: Raw) -> usize;
    fn GetModuleHandleW(name: *const u16) -> Raw;
}
#[link(name = "gdi32")]
extern "system" {
    fn CreateDIBSection(
        dc: Raw,
        info: Raw,
        usage: u32,
        bits: *mut Raw,
        section: Raw,
        offset: u32,
    ) -> Raw;
    fn CreateBitmap(width: i32, height: i32, planes: u32, bits: u32, data: *const u8) -> Raw;
    fn DeleteObject(object: Raw) -> i32;
}
#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(hwnd: Raw, attribute: u32, value: Raw, size: u32) -> Hr;
    fn DwmGetWindowAttribute(hwnd: Raw, attribute: u32, value: Raw, size: u32) -> Hr;
}
#[link(name = "user32")]
extern "system" {
    fn RegisterClassExW(class: *const WindowClass) -> u16;
    fn CreateWindowExW(
        ex: u32,
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
        param: Raw,
    ) -> Raw;
    fn DefWindowProcW(hwnd: Raw, message: u32, wparam: usize, lparam: isize) -> isize;
    fn DestroyWindow(hwnd: Raw) -> i32;
    fn ShowWindow(hwnd: Raw, command: i32) -> i32;
    fn IsIconic(hwnd: Raw) -> i32;
    fn GetWindowPlacement(hwnd: Raw, placement: *mut WindowPlacement) -> i32;
    fn SetWindowPlacement(hwnd: Raw, placement: *const WindowPlacement) -> i32;
    fn MonitorFromRect(rect: *const Rect, flags: u32) -> Raw;
    fn MonitorFromWindow(hwnd: Raw, flags: u32) -> Raw;
    fn GetMonitorInfoW(monitor: Raw, info: *mut MonitorInfo) -> i32;
    fn GetMessageW(message: *mut Message, hwnd: Raw, min: u32, max: u32) -> i32;
    fn GetKeyState(key: i32) -> i16;
    fn PeekMessageW(message: *mut Message, hwnd: Raw, min: u32, max: u32, remove: u32) -> i32;
    fn TranslateMessage(message: *const Message) -> i32;
    fn DispatchMessageW(message: *const Message) -> isize;
    fn PostQuitMessage(code: i32);
    fn SetWindowLongPtrW(hwnd: Raw, index: i32, value: isize) -> isize;
    fn GetWindowLongPtrW(hwnd: Raw, index: i32) -> isize;
    fn GetClientRect(hwnd: Raw, rect: *mut Rect) -> i32;
    fn GetWindowRect(hwnd: Raw, rect: *mut Rect) -> i32;
    fn SystemParametersInfoW(action: u32, value: u32, data: Raw, flags: u32) -> i32;
    fn SetWindowPos(
        hwnd: Raw,
        after: Raw,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    fn LoadCursorW(instance: Raw, name: *const u16) -> Raw;
    fn SendMessageW(hwnd: Raw, message: u32, wparam: usize, lparam: isize) -> isize;
    fn CreateIconIndirect(info: *const IconInfoRaw) -> Raw;
    fn DestroyIcon(icon: Raw) -> i32;
    fn GetDpiForSystem() -> u32;
    fn GetDpiForWindow(hwnd: Raw) -> u32;
}
