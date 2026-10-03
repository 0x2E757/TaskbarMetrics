//! Small owning COM/HSTRING wrappers. Com is intentionally !Send + !Sync.
//! Vtable slots below are the ABI from Windows SDK, not Rust trait-object vtables.

use super::abi::*;
use std::{marker::PhantomData, ptr, rc::Rc};

pub const UNKNOWN: Guid = Guid::from_u128(0x00000000_0000_0000_c000_000000000046);
pub const CLASS_FACTORY: Guid = Guid::from_u128(0x00000001_0000_0000_c000_000000000046);
pub const OBJECT_WITH_SITE: Guid = Guid::from_u128(0xfc4801a3_2ba9_11cf_a229_00aa003d7352);
pub const CALLBACK: Guid = Guid::from_u128(0xaa7a8931_80e4_4fec_8f3b_553f87b4966e);
pub const CALLBACK2: Guid = Guid::from_u128(0xbad9eb88_ae77_4397_b948_5fa2db0a19ea);
pub const DIAGNOSTICS: Guid = Guid::from_u128(0x18c9e2b6_3f43_4116_9f2b_ff935d7770d2);
pub const TREE_SERVICE: Guid = Guid::from_u128(0xa593b11a_d17f_48bb_8f66_83910731c8a5);
pub const AGILE_OBJECT: Guid = Guid::from_u128(0x94ea2b94_e9cc_49e0_c0ff_ee64ca8f5b90);

#[repr(C)]
pub struct UnknownVtbl {
    pub query: unsafe extern "system" fn(Raw, *const Guid, *mut Raw) -> Hr,
    pub add_ref: unsafe extern "system" fn(Raw) -> u32,
    pub release: unsafe extern "system" fn(Raw) -> u32,
}

pub struct Com {
    raw: Raw,
    _apartment: PhantomData<Rc<()>>,
}

impl Com {
    /// Takes ownership of one COM reference. Null pointers are errors.
    pub unsafe fn owned(raw: Raw) -> Result<Self> {
        if raw.is_null() {
            Err(E_POINTER)
        } else {
            Ok(Self {
                raw,
                _apartment: PhantomData,
            })
        }
    }

    pub fn raw(&self) -> Raw {
        self.raw
    }

    pub fn into_raw(self) -> Raw {
        let raw = self.raw;
        std::mem::forget(self);
        raw
    }

    /// Caller must use the exact signature and interface slot defined in the SDK.
    pub unsafe fn slot<T: Copy>(&self, index: usize) -> T {
        assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<Raw>());
        let table = *(self.raw as *const *const Raw);
        std::mem::transmute_copy(&*table.add(index))
    }

    pub fn query(&self, iid: &Guid) -> Result<Self> {
        unsafe {
            let call: unsafe extern "system" fn(Raw, *const Guid, *mut Raw) -> Hr = self.slot(0);
            let mut result = ptr::null_mut();
            check(call(self.raw, iid, &mut result))?;
            Self::owned(result)
        }
    }

    pub fn object(&self, slot: usize) -> Result<Self> {
        unsafe {
            let call: unsafe extern "system" fn(Raw, *mut Raw) -> Hr = self.slot(slot);
            let mut result = ptr::null_mut();
            check(call(self.raw, &mut result))?;
            Self::owned(result)
        }
    }

    pub fn string(&self, slot: usize) -> Result<String> {
        unsafe {
            let call: unsafe extern "system" fn(Raw, *mut Raw) -> Hr = self.slot(slot);
            let mut result = ptr::null_mut();
            check(call(self.raw, &mut result))?;
            Ok(HString(result).text())
        }
    }

    /// The HSTRING a getter at `slot` returns, without converting it.
    pub fn hstring(&self, slot: usize) -> Result<HString> {
        unsafe {
            let call: unsafe extern "system" fn(Raw, *mut Raw) -> Hr = self.slot(slot);
            let mut result = ptr::null_mut();
            check(call(self.raw, &mut result))?;
            Ok(HString(result))
        }
    }

    pub fn set_string(&self, slot: usize, value: &str) -> Result<()> {
        let value = HString::new(value)?;
        unsafe {
            let call: unsafe extern "system" fn(Raw, Raw) -> Hr = self.slot(slot);
            check(call(self.raw, value.0))
        }
    }
}

impl Clone for Com {
    fn clone(&self) -> Self {
        unsafe {
            let call: unsafe extern "system" fn(Raw) -> u32 = self.slot(1);
            call(self.raw);
            Self::owned(self.raw).expect("non-null COM pointer")
        }
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        unsafe {
            let call: unsafe extern "system" fn(Raw) -> u32 = self.slot(2);
            call(self.raw);
        }
    }
}

pub struct HString(pub Raw);

impl HString {
    pub fn new(text: &str) -> Result<Self> {
        let units: Vec<_> = text.encode_utf16().collect();
        let mut result = ptr::null_mut();
        unsafe {
            check(WindowsCreateString(
                units.as_ptr(),
                units.len() as u32,
                &mut result,
            ))?;
        }
        Ok(Self(result))
    }

    /// UTF-16 code units of the string.
    pub fn units(&self) -> &[u16] {
        unsafe {
            let mut length = 0;
            let raw = WindowsGetStringRawBuffer(self.0, &mut length);
            if length == 0 || raw.is_null() {
                &[]
            } else {
                std::slice::from_raw_parts(raw, length as usize)
            }
        }
    }

    /// Equal to `text`, compared without allocating.
    pub fn is(&self, text: &str) -> bool {
        self.units().iter().copied().eq(text.encode_utf16())
    }

    pub fn text(&self) -> String {
        unsafe {
            let mut length = 0;
            let raw = WindowsGetStringRawBuffer(self.0, &mut length);
            if length == 0 {
                String::new()
            } else {
                String::from_utf16_lossy(std::slice::from_raw_parts(raw, length as usize))
            }
        }
    }
}

impl Drop for HString {
    fn drop(&mut self) {
        unsafe {
            WindowsDeleteString(self.0);
        }
    }
}

pub fn activate(class: &str, iid: &Guid) -> Result<Com> {
    let name = HString::new(class)?;
    let mut result = ptr::null_mut();
    unsafe {
        check(RoActivateInstance(name.0, &mut result))?;
        Com::owned(result)?.query(iid)
    }
}

pub fn factory(class: &str, iid: &Guid) -> Result<Com> {
    let name = HString::new(class)?;
    let mut result = ptr::null_mut();
    unsafe {
        check(RoGetActivationFactory(name.0, iid, &mut result))?;
        Com::owned(result)
    }
}
// IAgileReference is the documented cross-apartment transport, unlike a raw XAML pointer.
pub struct Agile(Com);

unsafe impl Send for Agile {}
unsafe impl Sync for Agile {}
impl Agile {
    pub fn new(object: &Com, iid: &Guid) -> Result<Self> {
        let mut raw = ptr::null_mut();
        unsafe {
            check(RoGetAgileReference(0, iid, object.raw(), &mut raw))?;
            Ok(Self(Com::owned(raw)?))
        }
    }

    pub fn resolve(&self, iid: &Guid) -> Result<Com> {
        unsafe {
            let call: unsafe extern "system" fn(Raw, *const Guid, *mut Raw) -> Hr = self.0.slot(3);
            let mut raw = ptr::null_mut();
            check(call(self.0.raw(), iid, &mut raw))?;
            Com::owned(raw)
        }
    }
}
