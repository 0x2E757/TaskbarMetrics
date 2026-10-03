use super::*;
use std::ptr;

const DICTIONARY: Guid = Guid::from_u128(0xc1ea4f24_d6de_4191_8e3a_f48601f7489c);
const MAP: Guid = Guid::from_u128(0xf5f69427_55ed_5512_8429_d4f6626dfcdd);
const PROPERTY_VALUE: Guid = Guid::from_u128(0x629bdbc8_d932_4ff4_96b9_8d96c5c1e858);
const INSPECTABLE: Guid = Guid::from_u128(0xaf86e2e0_b12d_4c6a_9c5a_d7aa65101e90);
const BRUSH_KEYS: [&str; 9] = [
    "OmniButtonBackground",
    "OmniButtonBackgroundPointerOver",
    "OmniButtonBackgroundPressed",
    "OmniButtonForeground",
    "OmniButtonForegroundPointerOver",
    "OmniButtonForegroundPressed",
    "OmniButtonBorderBrush",
    "OmniButtonBorderBrushPointerOver",
    "OmniButtonBorderBrushPressed",
];

struct ResourceMap(Com);
impl ResourceMap {
    fn new(object: &Com) -> Result<Self> {
        Ok(Self(object.query(&MAP)?))
    }
    fn key(name: &str) -> Result<Com> {
        let factory = factory("Windows.Foundation.PropertyValue", &PROPERTY_VALUE)?;
        let name = HString::new(name)?;
        let mut value = ptr::null_mut();
        unsafe {
            let create: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = factory.slot(18);
            check(create(factory.raw(), name.0, &mut value))?;
            Com::owned(value)
        }
    }
    fn lookup(&self, name: &str) -> Result<Option<Com>> {
        let key = Self::key(name)?;
        let mut found = 0u8;
        let mut value = ptr::null_mut();
        unsafe {
            let has: unsafe extern "system" fn(Raw, Raw, *mut u8) -> Hr = self.0.slot(8);
            check(has(self.0.raw(), key.raw(), &mut found))?;
            if found == 0 {
                return Ok(None);
            }
            let lookup: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = self.0.slot(6);
            check(lookup(self.0.raw(), key.raw(), &mut value))?;
            if value.is_null() {
                Ok(None)
            } else {
                Ok(Some(Com::owned(value)?))
            }
        }
    }
    fn insert(&self, name: &str, value: &Com) -> Result<()> {
        let key = Self::key(name)?;
        let value = value.query(&INSPECTABLE)?;
        let mut replaced = 0u8;
        unsafe {
            let insert: unsafe extern "system" fn(Raw, Raw, Raw, *mut u8) -> Hr = self.0.slot(10);
            check(insert(self.0.raw(), key.raw(), value.raw(), &mut replaced))
        }
    }
}

/// ResourceDictionary has a single owner in UWP. Copy its structure instead of
/// reparenting the clock's dictionary. Brush instances are safely shareable.
pub(super) struct ClockResourceCopier;
impl ClockResourceCopier {
    pub(super) fn copy(source: &Com) -> Result<Com> {
        Self::copy_at(source, 0)
    }

    fn copy_at(source: &Com, depth: usize) -> Result<Com> {
        if depth > 16 {
            return Err(E_UNEXPECTED);
        }
        let target = activate("Windows.UI.Xaml.ResourceDictionary", &DICTIONARY)?;
        let source_map = ResourceMap::new(source)?;
        let target_map = ResourceMap::new(&target)?;
        for key in BRUSH_KEYS {
            if let Some(value) = source_map.lookup(key)? {
                target_map.insert(key, &value)?;
            }
        }
        let source_themes = ResourceMap::new(&source.object(9)?)?;
        let target_themes = ResourceMap::new(&target.object(9)?)?;
        for theme in ["Default", "Light", "Dark", "HighContrast"] {
            if let Some(dictionary) = source_themes.lookup(theme)? {
                let dictionary = dictionary.query(&DICTIONARY)?;
                target_themes.insert(theme, &Self::copy_at(&dictionary, depth + 1)?)?;
            }
        }
        let source_merged = XamlVector(source.object(8)?);
        let target_merged = XamlVector(target.object(8)?);
        let count = source_merged.size()?;
        if count > 128 {
            return Err(E_UNEXPECTED);
        }
        for index in 0..count {
            let mut dictionary = ptr::null_mut();
            let dictionary = unsafe {
                let get: unsafe extern "system" fn(Raw, u32, *mut Raw) -> Hr =
                    source_merged.0.slot(6);
                check(get(source_merged.0.raw(), index, &mut dictionary))?;
                Com::owned(dictionary)?
            };
            target_merged.append(&Self::copy_at(&dictionary, depth + 1)?)?;
        }
        Ok(target)
    }
}
