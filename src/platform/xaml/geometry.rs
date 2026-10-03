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
            weather: false,
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

/// Class of the container of the weather (Widgets) button.
const WIDGETS: &str = "Taskbar.AugmentedEntryPointButton";
/// Width of the hover border inside the weather's background, in pixels.
const WEATHER_BORDER: f64 = 1.0;

/// One depth-first walk over the tree, bounded to 2048 nodes.
struct Walk<'a> {
    geometry: &'a TaskbarGeometry,
    ranges: Vec<OccupiedRange>,
    remaining: usize,
    /// Inside the weather's container, where only its button counts.
    weather: bool,
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
            // Of the weather, only its background shows: the container reserves
            // about 60 px of empty space after it.
            if self.weather {
                if name.is("BackgroundElement") {
                    return self.push(framework, &element, true);
                }
                return self.children(object, depth);
            }
            let class = framework.hstring(4)?;
            let class = class.units();
            if class.iter().copied().eq(WIDGETS.encode_utf16()) {
                let before = self.ranges.len();
                self.weather = true;
                let found = self.children(object, depth);
                self.weather = false;
                found?;
                // Without its background, as in another Windows build, the whole container.
                if self.ranges.len() == before {
                    self.push(framework, &element, false)?;
                }
                return Ok(());
            }
            // Stop at interactive containers; nested child geometry is redundant.
            if (starts_with(class, "Taskbar.") && contains(class, "Button"))
                || class
                    .iter()
                    .copied()
                    .eq("SystemTray.SystemTrayFrame".encode_utf16())
            {
                return self.push(framework, &element, false);
            }
        }
        self.children(object, depth)
    }

    /// The range `element` takes in the root's coordinates, when it has a size;
    /// `snug` for the weather, which the tiles follow at their own spacing.
    fn push(&mut self, framework: Com, element: &Com, snug: bool) -> Result<()> {
        let framework = XamlElement(framework);
        let width = framework.number(13)?;
        let height = framework.number(14)?;
        if width <= 0.0 || height <= 0.0 {
            return Ok(());
        }
        let mut transform = ptr::null_mut();
        let mut bounds = Rect::default();
        unsafe {
            let get: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = element.slot(98);
            check(get(element.raw(), self.geometry.root.raw(), &mut transform))?;
            let transform = Com::owned(transform)?;
            let map: unsafe extern "system" fn(Raw, Rect, *mut Rect) -> Hr = transform.slot(9);
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
        let right = (bounds.x + bounds.width) as f64;
        self.ranges.push(OccupiedRange {
            left: bounds.x as f64,
            // The weather's hover border lies inside its background, a tile's outside
            // the tile: measured to its fill, the weather is as far as a tile.
            right: if snug { right - WEATHER_BORDER } else { right },
            snug,
        });
        Ok(())
    }

    fn children(&mut self, object: &Com, depth: usize) -> Result<()> {
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
