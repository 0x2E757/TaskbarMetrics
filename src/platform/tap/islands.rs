//! Each taskbar is a XAML island; its taskbar window tells the monitor it is on.

use super::*;
use std::collections::HashMap;

const XAML_SOURCE: &str = "Windows.UI.Xaml.Hosting.DesktopWindowXamlSource";
const XAML_SOURCE_NATIVE: Guid = Guid::from_u128(0x3cbcf1bf_2f76_4e9c_96ab_e84b37972554);

/// Taskbar windows by the diagnostics handle of an island source and of the
/// island's root element, which the visual tree reports as the source's child.
#[derive(Default)]
pub(super) struct Islands {
    sources: HashMap<u64, isize>,
    roots: HashMap<u64, isize>,
}

impl Islands {
    /// Notes an added tree root: an island source, or the root element of one.
    pub(super) fn add(&mut self, relation: Relation, handle: u64, object: &Com) -> Result<()> {
        if let Some(window) = self.sources.get(&relation.parent) {
            self.roots.insert(handle, *window);
        } else if relation.parent == 0 && object.string(4)? == XAML_SOURCE {
            self.sources.insert(handle, Self::host(object)?);
        }
        Ok(())
    }

    /// Diagnostics handles are reused after removal.
    pub(super) fn remove(&mut self, handle: u64) {
        self.sources.remove(&handle);
        self.roots.remove(&handle);
    }

    /// The taskbar window of the island whose root element is `root`.
    pub(super) fn window(&self, root: u64) -> Option<isize> {
        self.roots.get(&root).copied()
    }

    /// `Shell_TrayWnd` or `Shell_SecondaryTrayWnd` above the island's own window.
    fn host(source: &Com) -> Result<isize> {
        let native = source.query(&XAML_SOURCE_NATIVE)?;
        let mut window = ptr::null_mut();
        unsafe {
            let get: unsafe extern "system" fn(Raw, *mut Raw) -> Hr = native.slot(4);
            check(get(native.raw(), &mut window))?;
            // GA_ROOT.
            Ok(GetAncestor(window, 2) as isize)
        }
    }
}

#[link(name = "user32")]
extern "system" {
    fn GetAncestor(window: Raw, flags: u32) -> Raw;
}
