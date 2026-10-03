use super::model::{Device, Resource};
use crate::platform::devices::{DeviceCatalog, DeviceId, Ordinals, WatchList};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

/// Devices of the navigation menu: the machine's, re-enumerated every 10 s, or a
/// fixed demo set. While the window is open it tells the recorder which devices
/// it shows, so their history starts even without a taskbar tile.
pub struct DeviceMenu {
    pub devices: Vec<Device>,
    catalog: Option<DeviceCatalog>,
    checked: Instant,
    watch: Option<(PathBuf, Instant)>,
}

impl DeviceMenu {
    const PERIOD: Duration = Duration::from_secs(10);
    const WATCH: Duration = Duration::from_secs(2);

    /// The machine's devices; `watch` is the recorder's watch list, if any.
    pub fn system(watch: Option<PathBuf>) -> Self {
        let mut catalog = DeviceCatalog::new();
        // The disk counter lists its instances from the second collection on.
        std::thread::sleep(Duration::from_millis(50));
        Self {
            devices: Self::listed(&mut catalog),
            catalog: Some(catalog),
            checked: Instant::now(),
            watch: watch.map(|path| (path, Instant::now() - Self::WATCH)),
        }
    }

    pub fn fixed(ids: Vec<DeviceId>) -> Self {
        Self {
            devices: Self::menu(ids, None),
            catalog: None,
            checked: Instant::now(),
            watch: None,
        }
    }

    /// Menu entries, numbered within their group: «GPU 1», «GPU 2», but «SSD»
    /// when it is the only one. `catalog` names the machine's disks by media.
    fn menu(ids: Vec<DeviceId>, catalog: Option<&DeviceCatalog>) -> Vec<Device> {
        let mut devices: Vec<Device> = ids
            .into_iter()
            .filter_map(Device::new)
            .map(|device| {
                let disk = match (device.resource, &device.id.tag, catalog) {
                    (Resource::Disk, Some(tag), Some(catalog)) => catalog.disk(tag).cloned(),
                    _ => None,
                };
                let link = match (device.resource, &device.id.tag, catalog) {
                    (Resource::Net, Some(tag), Some(catalog)) => catalog.link(tag),
                    _ => None,
                };
                device.with_disk(disk).with_link(link)
            })
            .collect();
        let groups: Vec<_> = devices.iter().map(Device::group).collect();
        for ((device, group), ordinal) in devices.iter_mut().zip(&groups).zip(Ordinals::of(&groups))
        {
            device.ordinal = ordinal.filter(|_| group.is_some());
        }
        devices
    }

    fn listed(catalog: &mut DeviceCatalog) -> Vec<Device> {
        let ids = catalog.devices();
        Self::menu(ids, Some(catalog))
    }

    /// Re-enumerates the machine's devices; true when the menu changed.
    pub fn refresh(&mut self) -> bool {
        if let Some((path, written)) = &mut self.watch {
            if written.elapsed() >= Self::WATCH {
                *written = Instant::now();
                let ids: Vec<_> = self.devices.iter().map(|d| d.id.clone()).collect();
                let _ = WatchList::write(path, &ids);
            }
        }
        let Some(catalog) = &mut self.catalog else {
            return false;
        };
        if self.checked.elapsed() < Self::PERIOD {
            return false;
        }
        self.checked = Instant::now();
        let devices = Self::listed(catalog);
        let changed = devices != self.devices;
        if changed {
            self.devices = devices;
        }
        changed
    }

    /// The device a bare kind stands for: the machine's main device, or the first
    /// of its kind in a fixed menu.
    pub fn canonical(&self, id: &DeviceId) -> DeviceId {
        if self.catalog.is_some() {
            return DeviceCatalog::canonical(id);
        }
        match &id.tag {
            Some(_) => id.clone(),
            None => self
                .devices
                .iter()
                .find(|d| d.id.kind == id.kind)
                .map_or_else(|| id.clone(), |d| d.id.clone()),
        }
    }

    /// Position of `id` in the menu; a bare kind (`disk`, from a taskbar tile) is
    /// its main device.
    pub fn position(&self, id: &DeviceId) -> Option<usize> {
        let canonical = self.canonical(id);
        self.devices
            .iter()
            .position(|d| d.id == canonical)
            .or_else(|| self.devices.iter().position(|d| d.id.kind == id.kind))
    }
}
