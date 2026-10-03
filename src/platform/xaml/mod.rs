//! Native XAML view; all element references remain on their UI thread.
use super::{abi::*, com::*, data_directory::DataDirectory};
use crate::{
    config::Settings,
    presentation::{LeftPlacement, OccupiedRange},
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::{atomic::Ordering, Arc},
};
mod alert;
pub(crate) mod appearance;
mod button;
pub(crate) use alert::AlertSettings;
mod palette;
mod preview;
pub(crate) use palette::Palette;
pub(crate) use preview::{TileStyle, WidgetPreview};
mod clock;
pub(crate) use clock::ClockCatalog;
mod dispatcher;
pub(crate) mod events;
mod geometry;
mod interop;
mod reorder;
mod resources;
mod rounded_clip;
mod visual_tree;
use button::MetricsButton;
pub(crate) fn verify_tiles() -> Result<()> {
    MetricsButton::verify()
}
pub use dispatcher::Target;
use geometry::TaskbarGeometry;
use interop::{XamlElement, XamlVector};
pub const FRAMEWORK: Guid = Guid::from_u128(0xa391d09b_4a99_4b7c_9d8d_6fa5d01f6fbf);
const UI_ELEMENT: Guid = Guid::from_u128(0x676d0be9_b65c_41c6_ba40_58cf87f201c1);
const DEPENDENCY_OBJECT: Guid = Guid::from_u128(0x5c526665_f60e_4912_af59_5fe0680f089d);
pub const PANEL: Guid = Guid::from_u128(0xa50a4bbd_8361_469c_90da_e9a40c7474df);
const TEXT: Guid = Guid::from_u128(0xae2d9271_3b4a_45fc_8468_f7949548f4d5);
const GRID: Guid = Guid::from_u128(0xfd104460_2e15_4ba3_8b8f_fa693a4161e9);
const GRID_STATICS: Guid = Guid::from_u128(0x64fe2e9f_f951_42b6_a9ce_bb179af11595);
const TREE_HELPER: Guid = Guid::from_u128(0xe75758c4_d25d_4b1d_971f_596f17f12baa);
const QUEUE: Guid = Guid::from_u128(0x603e88e4_a338_4ffe_a457_a5cfb9ceb899);
const QUEUE_STATICS: Guid = Guid::from_u128(0xa96d83d7_9371_4517_9245_d0824ac12c74);
const HANDLER: Guid = Guid::from_u128(0xdfa2dc9c_1a2d_4917_98f2_939af1d6e0c8);

#[derive(Clone)]
struct UiTarget {
    style: TileStyle,
    next_style_check: std::time::Instant,
    panel: Com,
    button: Option<MetricsButton>,
    settings: Settings,
    last_error: Rc<Cell<Option<Hr>>>,
    last_visible: Option<bool>,
    /// Reader of the taskbar's occupied ranges, created on the first update.
    geometry: Option<TaskbarGeometry>,
}
thread_local! { static UI: RefCell<HashMap<u64, UiTarget>> = RefCell::new(HashMap::new()); }
impl Target {
    pub fn insert(self: &Arc<Self>, panel: Com, settings: Settings) -> Result<()> {
        let target = self;
        UI.with(|ui| {
            ui.borrow_mut().insert(
                target.key,
                UiTarget {
                    style: DataDirectory::file("taskbar-metrics.appearance")
                        .ok()
                        .and_then(|p| appearance::Appearance::read(&p).ok())
                        .unwrap_or_default(),
                    next_style_check: std::time::Instant::now(),
                    panel,
                    button: None,
                    settings,
                    last_error: Rc::new(Cell::new(None)),
                    last_visible: None,
                    geometry: None,
                },
            )
        });
        // The clock may be discovered after this root. The next queued metric
        // update constructs the view, retrying if the clock is not available yet.
        Ok(())
    }
}

impl UiTarget {
    fn update(target: &Arc<Target>, readings: &[crate::metrics::MetricReading]) -> Result<()> {
        let Some(mut entry) = UI.with(|ui| ui.borrow_mut().remove(&target.key)) else {
            return Ok(());
        };
        let result = (|| {
            if entry.button.is_none() {
                if let Err(hr) = entry.insert() {
                    if let Some(button) = entry.button.take() {
                        let _ = XamlVector(entry.panel.object(6)?).remove(&button.element()?);
                    }
                    return Err(hr);
                }
            }
            entry.refresh(readings)
        })();
        let error = result.as_ref().err().copied();
        if entry.last_error.replace(error) != error {
            if let Some(hr) = error {
                log(&format!("Metrics view is waiting: 0x{:08X}", hr as u32));
            }
        }
        if target.active.load(Ordering::Acquire) {
            UI.with(|ui| {
                ui.borrow_mut().insert(target.key, entry);
            });
        } else {
            entry.cleanup()?;
        }
        Ok(())
    }

    fn insert(&mut self) -> Result<()> {
        let grid = self.panel.query(&GRID)?;
        let button = MetricsButton::new(
            &self.panel,
            &self.settings.metrics,
            &self.style,
            Rc::new(Self::store_order),
        )?;
        let framework = button.framework()?;
        framework.set_enum(28, 0)?; // Left
        framework.set_enum(30, 1)?; // Center
        framework.set_number(16, button.width().max(self.settings.width))?;
        let element = button.element()?;
        button.visible(false)?;
        unsafe {
            let factory = factory("Windows.UI.Xaml.Controls.Grid", &GRID_STATICS)?;
            for (collection, slot) in [(7, 17), (6, 14)] {
                let span = XamlVector(grid.object(collection)?).size()?.max(1);
                let set: unsafe extern "system" fn(Raw, Raw, i32) -> Hr = factory.slot(slot);
                check(set(factory.raw(), framework.0.raw(), span as i32))?;
            }
        }
        self.button = Some(button);
        XamlVector(self.panel.object(6)?).append(&element)?;
        log("Left metrics element inserted; waiting for layout");
        Ok(())
    }

    fn refresh(&mut self, readings: &[crate::metrics::MetricReading]) -> Result<()> {
        if std::time::Instant::now() >= self.next_style_check {
            self.next_style_check = std::time::Instant::now() + std::time::Duration::from_secs(1);
            // Our own drag already put the tiles in the order the file now holds.
            if let Some(ids) = self.button.as_ref().and_then(MetricsButton::ids) {
                self.settings.metrics = ids;
            }
            // A new appearance or a new set of tiles rebuilds the view; tiles that
            // stay keep their history.
            let directory = DataDirectory::path().ok();
            let style = directory
                .as_ref()
                .and_then(|p| {
                    appearance::Appearance::read(&p.join("taskbar-metrics.appearance")).ok()
                })
                .filter(|s| *s != self.style);
            let settings = directory
                .as_ref()
                .and_then(|p| Settings::load(&p.join("taskbar-metrics.conf")).ok())
                .filter(|s| s.metrics != self.settings.metrics);
            if style.is_some() || settings.is_some() {
                let old = self.button.take();
                let previous = (self.style, self.settings.clone());
                if let Some(style) = style {
                    self.style = style;
                }
                if let Some(settings) = settings {
                    self.settings = settings;
                }
                if let Err(error) = self.insert() {
                    if let Some(failed) = self.button.take() {
                        let _ = XamlVector(self.panel.object(6)?).remove(&failed.element()?);
                    }
                    (self.style, self.settings) = previous;
                    self.button = old;
                    return Err(error);
                }
                if let Some(old) = old {
                    if let Some(new) = &mut self.button {
                        new.restore_history(&old);
                    }
                    XamlVector(self.panel.object(6)?).remove(&old.element()?)?;
                }
            }
        }
        let Some(button) = &mut self.button else {
            return Ok(());
        };
        button.update(readings)?;
        let placement: Result<Option<f64>> = (|| {
            let width = XamlElement(self.panel.query(&FRAMEWORK)?).number(13)?;
            if self.geometry.is_none() {
                self.geometry = Some(TaskbarGeometry::new(&self.panel)?);
            }
            let occupied = self
                .geometry
                .as_ref()
                .ok_or(E_UNEXPECTED)?
                .read(&self.panel)?;
            let required = button.width().max(self.settings.width);
            let left = LeftPlacement::new(required, self.settings.gap).position(width, &occupied);
            if self.last_visible != Some(left.is_some()) {
                log(&format!("Metrics placement: visible={}, taskbar={width}, required={required}, occupied={:?}", left.is_some(), occupied.iter().map(|range| (range.left, range.right)).collect::<Vec<_>>()));
                self.last_visible = Some(left.is_some());
            }
            Ok(left)
        })();
        let error = placement.as_ref().err().copied();
        if self.last_error.replace(error) != error {
            if let Some(hr) = error {
                log(&format!("Metrics layout failed: 0x{:08X}", hr as u32));
            }
        }
        match placement {
            Ok(Some(left)) => {
                button.framework()?.margin(left, 0.0)?;
                button.visible(true)?;
                unsafe {
                    if let Ok(event) = Handle::new(OpenEventW(
                        2,
                        0,
                        event_name("ready", GetCurrentProcessId()).as_ptr(),
                    )) {
                        SetEvent(event.0);
                    }
                }
            }
            Ok(None) | Err(_) => button.visible(false)?,
        }
        Ok(())
    }

    /// A dragged order goes to the configuration; other taskbars and the
    /// settings window pick it up from there.
    fn store_order(ids: &[String]) {
        let stored = DataDirectory::path()
            .map_err(|error| error.to_string())
            .and_then(|directory| {
                crate::config::ConfigFile::locate(&directory)
                    .write(&[("metrics", ids.join(","))])
                    .map_err(|error| error.to_string())
            });
        if let Err(error) = stored {
            log(&format!("Saving the tile order failed: {error}"));
        }
    }

    fn cleanup(self) -> Result<()> {
        if let Some(button) = self.button {
            XamlVector(self.panel.object(6)?).remove(&button.element()?)?;
        }
        Ok(())
    }
}

impl UiTarget {
    fn remove(key: u64) -> Result<()> {
        let entry = UI.with(|ui| ui.borrow_mut().remove(&key));
        if let Some(entry) = entry {
            entry.cleanup()?;
        }
        Ok(())
    }
}
