//! The metrics window, plus the standalone tile editor behind `--editor`.
//! No code runs in Explorer.
use super::{abi::*, com::*};
use std::ptr;
mod color_picker;
mod colors;
mod monitor;
mod monitoring;
mod playground;
mod window;
mod window_memory;
use window::NativeWindow;

const CLOSABLE: Guid = Guid::from_u128(0x30d5a829_7fa4_4026_83bb_d75bae4ea99e);

pub struct MetricsWindow;
impl MetricsWindow {
    pub fn run() -> Result<()> {
        if !opens_editor() {
            return monitor::run();
        }
        let _apartment = StaApartment::new()?;
        let manager = XamlLifetime(
            factory(
                "Windows.UI.Xaml.Hosting.WindowsXamlManager",
                &Guid::from_u128(0x28258a12_7d82_505b_b210_712b04a58882),
            )?
            .object(6)?,
        );
        let theme = SystemTheme::read()?;
        if std::env::args().any(|arg| arg == "--verify-tiles") {
            let result = super::xaml::verify_tiles();
            drop(manager);
            NativeWindow::drain();
            return result;
        }
        let window = NativeWindow::new(theme.dark)?;
        let island = XamlIsland::new(&window)?;
        let root = island.load(&theme.markup(&playground::Playground::markup()))?;
        let mut playground = playground::Playground::new(&root)?;
        if std::env::args().any(|arg| arg == "--verify-settings") {
            window.show();
            NativeWindow::drain();
            let result = playground.verify(&root);
            drop(playground);
            drop(root);
            drop(island);
            drop(window);
            drop(manager);
            NativeWindow::drain();
            return result;
        }
        window.show();
        let result = window.run(&island.native, NativeWindow::POLL_FRAME, || {
            playground.refresh()
        });
        drop(playground);
        drop(root);
        drop(island);
        drop(window);
        drop(manager);
        NativeWindow::drain();
        result
    }
}

/// The tile editor and its checks; every other start opens the metrics window.
fn opens_editor() -> bool {
    std::env::args().any(|a| {
        matches!(
            a.as_str(),
            "--editor" | "--verify-settings" | "--verify-tiles"
        )
    })
}

/// Reads the Windows app palette, which can differ from the taskbar theme.
struct SystemTheme {
    dark: bool,
}
impl SystemTheme {
    fn read() -> Result<Self> {
        let settings = activate(
            "Windows.UI.ViewManagement.UISettings",
            &Guid::from_u128(0x03021be4_5254_4781_8194_5168f7d06d7b),
        )?;
        // Windows.UI.Color is four bytes in A,R,G,B order; Background = 0.
        let mut color = [0u8; 4];
        unsafe {
            let get: unsafe extern "system" fn(Raw, i32, *mut [u8; 4]) -> Hr = settings.slot(6);
            check(get(settings.raw(), 0, &mut color))?;
        }
        Ok(Self {
            dark: u32::from(color[1]) + u32::from(color[2]) + u32::from(color[3]) < 384,
        })
    }
    fn markup(&self, markup: &str) -> String {
        markup
            .replace(
                "@BACKGROUND@",
                if self.dark { "#202024" } else { "#F3F3F3" },
            )
            .replace(
                "@FOREGROUND@",
                if self.dark { "#F5F5F5" } else { "#181818" },
            )
            .replace(
                "RequestedTheme=\"Default\"",
                if self.dark {
                    "RequestedTheme=\"Dark\""
                } else {
                    "RequestedTheme=\"Light\""
                },
            )
    }
}

struct StaApartment;
impl StaApartment {
    fn new() -> Result<Self> {
        unsafe {
            check(RoInitialize(0))?;
        }
        Ok(Self)
    }
}
impl Drop for StaApartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}

struct XamlLifetime(Com);
impl Drop for XamlLifetime {
    fn drop(&mut self) {
        if let Ok(closable) = self.0.query(&CLOSABLE) {
            unsafe {
                let close: unsafe extern "system" fn(Raw) -> Hr = closable.slot(6);
                let _ = close(closable.raw());
            }
        }
    }
}

struct XamlIsland {
    source: XamlLifetime,
    native: Com,
}
impl XamlIsland {
    fn new(window: &NativeWindow) -> Result<Self> {
        let source = XamlLifetime(activate(
            "Windows.UI.Xaml.Hosting.DesktopWindowXamlSource",
            &Guid::from_u128(0xd585bfe1_00ff_51be_ba1d_a1329956ea0a),
        )?);
        let native = source
            .0
            .query(&Guid::from_u128(0xe3dcd8c7_3057_4692_99c3_7b7720afda31))?;
        unsafe {
            let attach: unsafe extern "system" fn(Raw, Raw) -> Hr = native.slot(3);
            check(attach(native.raw(), window.raw()))?;
            let get: unsafe extern "system" fn(Raw, *mut Raw) -> Hr = native.slot(4);
            let mut child = ptr::null_mut();
            check(get(native.raw(), &mut child))?;
            window.attach(child);
        }
        Ok(Self { source, native })
    }

    fn load(&self, markup: &str) -> Result<Com> {
        let reader = factory(
            "Windows.UI.Xaml.Markup.XamlReader",
            &Guid::from_u128(0x9891c6bd_534f_4955_b85a_8a8dc0dca602),
        )?;
        let markup = HString::new(markup)?;
        unsafe {
            let load: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = reader.slot(6);
            let mut root = ptr::null_mut();
            check(load(reader.raw(), markup.0, &mut root))?;
            let root = Com::owned(root)?
                .query(&Guid::from_u128(0x676d0be9_b65c_41c6_ba40_58cf87f201c1))?;
            let set: unsafe extern "system" fn(Raw, Raw) -> Hr = self.source.0.slot(7);
            check(set(self.source.0.raw(), root.raw()))?;
            Ok(root)
        }
    }
}
