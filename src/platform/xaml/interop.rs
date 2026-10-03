use super::*;

#[repr(C)]
#[derive(Default)]
pub(super) struct Thickness {
    pub(super) left: f64,
    pub(super) top: f64,
    pub(super) right: f64,
    pub(super) bottom: f64,
}

#[repr(C)]
#[derive(Default)]
pub(super) struct Rect {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

/// Typed operations for the small subset of XAML used by this adapter.
pub(super) struct XamlElement(pub(super) Com);

impl XamlElement {
    pub(super) fn number(&self, slot: usize) -> Result<f64> {
        let mut value = 0.0;
        unsafe {
            let get: unsafe extern "system" fn(Raw, *mut f64) -> Hr = self.0.slot(slot);
            check(get(self.0.raw(), &mut value))?;
        }
        Ok(value)
    }

    pub(super) fn set_number(&self, slot: usize, value: f64) -> Result<()> {
        unsafe {
            let set: unsafe extern "system" fn(Raw, f64) -> Hr = self.0.slot(slot);
            check(set(self.0.raw(), value))
        }
    }

    pub(super) fn set_enum(&self, slot: usize, value: i32) -> Result<()> {
        unsafe {
            let set: unsafe extern "system" fn(Raw, i32) -> Hr = self.0.slot(slot);
            check(set(self.0.raw(), value))
        }
    }

    pub(super) fn margin(&self, left: f64, right: f64) -> Result<()> {
        unsafe {
            let set: unsafe extern "system" fn(Raw, Thickness) -> Hr = self.0.slot(32);
            check(set(
                self.0.raw(),
                Thickness {
                    left,
                    right,
                    top: 0.0,
                    bottom: 0.0,
                },
            ))
        }
    }
}

pub(super) struct XamlVector(pub(super) Com);

impl XamlVector {
    pub(super) fn size(&self) -> Result<u32> {
        let mut size = 0;
        unsafe {
            let get: unsafe extern "system" fn(Raw, *mut u32) -> Hr = self.0.slot(7);
            check(get(self.0.raw(), &mut size))?;
        }
        Ok(size)
    }

    pub(super) fn append(&self, item: &Com) -> Result<()> {
        unsafe {
            let call: unsafe extern "system" fn(Raw, Raw) -> Hr = self.0.slot(13);
            check(call(self.0.raw(), item.raw()))
        }
    }

    pub(super) fn insert(&self, index: u32, item: &Com) -> Result<()> {
        unsafe {
            let call: unsafe extern "system" fn(Raw, u32, Raw) -> Hr = self.0.slot(11);
            check(call(self.0.raw(), index, item.raw()))
        }
    }

    pub(super) fn remove(&self, item: &Com) -> Result<()> {
        let (mut index, mut found) = (0, 0u8);
        unsafe {
            let find: unsafe extern "system" fn(Raw, Raw, *mut u32, *mut u8) -> Hr = self.0.slot(9);
            check(find(self.0.raw(), item.raw(), &mut index, &mut found))?;
            if found != 0 {
                let remove: unsafe extern "system" fn(Raw, u32) -> Hr = self.0.slot(12);
                check(remove(self.0.raw(), index))?;
            }
        }
        Ok(())
    }
}
