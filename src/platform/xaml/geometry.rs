use super::interop::Rect;
use super::*;
use std::ptr;
/// Screen ranges taken by taskbar buttons and the tray, read from the visual tree of
/// one taskbar on every update. The tree helper and the root are resolved once, and
/// names are compared as UTF-16 without allocating: the walk visits hundreds of nodes.
#[derive(Clone)]
pub(super) struct TaskbarGeometry {
    helper: Com,
    root: Com,
}
impl TaskbarGeometry {
    pub(super) fn new(root: &Com) -> Result<Self> {
        Ok(Self {
            helper: factory("Windows.UI.Xaml.Media.VisualTreeHelper", &TREE_HELPER)?,
            root: root.query(&UI_ELEMENT)?,
        })
    }
    pub(super) fn read(&self, root: &Com) -> Result<Vec<OccupiedRange>> {
        let mut walk = Walk {
            geometry: self,
            ranges: Vec::new(),
            remaining: 2048,
        };
        walk.visit(root, 0)?;
        if walk.ranges.is_empty() {
            return Err(E_UNEXPECTED);
        }
        Ok(walk.ranges)
    }
}

/// `text` starts the UTF-16 `units`.
fn starts_with(units: &[u16], text: &str) -> bool {
    let mut rest = units.iter().copied();
    text.encode_utf16().all(|unit| rest.next() == Some(unit))
}
/// `text` occurs anywhere in the UTF-16 `units`.
fn contains(units: &[u16], text: &str) -> bool {
    (0..units.len()).any(|start| starts_with(&units[start..], text))
}

/// One depth-first walk over the tree, bounded to 2048 nodes.
struct Walk<'a> {
    geometry: &'a TaskbarGeometry,
    ranges: Vec<OccupiedRange>,
    remaining: usize,
}
impl Walk<'_> {
    fn visit(&mut self, object: &Com, depth: usize) -> Result<()> {
        if depth > 32 || self.remaining == 0 {
            return Err(E_UNEXPECTED);
        }
        self.remaining -= 1;
        if let Ok(framework) = object.query(&FRAMEWORK) {
            let name = framework.hstring(33)?;
            if name.is("TaskbarMetricsText") || name.is("TaskbarMetricsButton") {
                return Ok(());
            }
            let element = object.query(&UI_ELEMENT)?;
            let mut visibility = 0;
            unsafe {
                let get: unsafe extern "system" fn(Raw, *mut i32) -> Hr = element.slot(21);
                check(get(element.raw(), &mut visibility))?;
            }
            if visibility != 0 {
                return Ok(());
            }
            let class = framework.hstring(4)?;
            let class = class.units();
            // Stop at interactive containers; nested child geometry is redundant.
            if (starts_with(class, "Taskbar.") && contains(class, "Button"))
                || class
                    .iter()
                    .copied()
                    .eq("SystemTray.SystemTrayFrame".encode_utf16())
            {
                let framework = XamlElement(framework);
                let width = framework.number(13)?;
                let height = framework.number(14)?;
                if width > 0.0 && height > 0.0 {
                    let mut transform = ptr::null_mut();
                    let mut bounds = Rect::default();
                    unsafe {
                        let get: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr =
                            element.slot(98);
                        check(get(element.raw(), self.geometry.root.raw(), &mut transform))?;
                        let transform = Com::owned(transform)?;
                        let map: unsafe extern "system" fn(Raw, Rect, *mut Rect) -> Hr =
                            transform.slot(9);
                        check(map(
                            transform.raw(),
                            Rect {
                                x: 0.0,
                                y: 0.0,
                                width: width as f32,
                                height: height as f32,
                            },
                            &mut bounds,
                        ))?;
                    }
                    self.ranges.push(OccupiedRange {
                        left: bounds.x as f64,
                        right: (bounds.x + bounds.width) as f64,
                    });
                }
                return Ok(());
            }
        }
        let helper = &self.geometry.helper;
        let dependency = object.query(&DEPENDENCY_OBJECT)?;
        let mut count = 0;
        unsafe {
            let get: unsafe extern "system" fn(Raw, Raw, *mut i32) -> Hr = helper.slot(11);
            check(get(helper.raw(), dependency.raw(), &mut count))?;
        }
        if !(0..=2048).contains(&count) {
            return Err(E_UNEXPECTED);
        }
        for index in 0..count {
            let mut child = ptr::null_mut();
            unsafe {
                let get: unsafe extern "system" fn(Raw, Raw, i32, *mut Raw) -> Hr = helper.slot(10);
                check(get(helper.raw(), dependency.raw(), index, &mut child))?;
                self.visit(&Com::owned(child)?, depth + 1)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn class_names_match_as_utf16_without_allocating() {
        let class: Vec<u16> = "Taskbar.TaskListButton".encode_utf16().collect();
        assert!(starts_with(&class, "Taskbar.") && contains(&class, "Button"));
        let tray: Vec<u16> = "SystemTray.SystemTrayFrame".encode_utf16().collect();
        assert!(!starts_with(&tray, "Taskbar.") && !contains(&tray, "Button"));
        assert!(!starts_with(&class[..4], "Taskbar."));
        assert!(contains(&class, "List") && !contains(&class, "Frame"));
    }
}
