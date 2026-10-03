use super::super::*;
pub struct Ui;
impl Ui {
    pub fn background(element: &Com, brush: &Com) -> Result<()> {
        let element = element.query(&crate::platform::xaml::PANEL)?;
        let brush = brush.query(&Guid::from_u128(0x9d850850_66f3_48df_9a8f_824bd5e070af))?;
        unsafe {
            let set: unsafe extern "system" fn(Raw, Raw) -> Hr = element.slot(8);
            check(set(element.raw(), brush.raw()))
        }
    }
    pub fn height(element: &Com, value: f64) -> Result<()> {
        let element = element.query(&crate::platform::xaml::FRAMEWORK)?;
        unsafe {
            let set: unsafe extern "system" fn(Raw, f64) -> Hr = element.slot(18);
            check(set(element.raw(), value))
        }
    }
    pub fn min_width(element: &Com, value: f64) -> Result<()> {
        let element = element.query(&crate::platform::xaml::FRAMEWORK)?;
        unsafe {
            let set: unsafe extern "system" fn(Raw, f64) -> Hr = element.slot(20);
            check(set(element.raw(), value))
        }
    }
    pub fn size(element: &Com) -> Result<(f64, f64)> {
        let element = element.query(&crate::platform::xaml::FRAMEWORK)?;
        let mut values = [0.0; 2];
        for (slot, value) in [13, 14].into_iter().zip(&mut values) {
            unsafe {
                let get: unsafe extern "system" fn(Raw, *mut f64) -> Hr = element.slot(slot);
                check(get(element.raw(), value))?;
            }
        }
        Ok((values[0], values[1]))
    }
    /// Unconstrained desired size of a detached element (`UIElement.Measure` + `DesiredSize`).
    pub fn measure(element: &Com) -> Result<(f64, f64)> {
        #[repr(C)]
        #[derive(Default)]
        struct Size {
            width: f32,
            height: f32,
        }
        let element = element.query(&Guid::from_u128(0x676d0be9_b65c_41c6_ba40_58cf87f201c1))?;
        let mut desired = Size::default();
        unsafe {
            let measure: unsafe extern "system" fn(Raw, Size) -> Hr = element.slot(91);
            check(measure(
                element.raw(),
                Size {
                    width: f32::INFINITY,
                    height: f32::INFINITY,
                },
            ))?;
            let get: unsafe extern "system" fn(Raw, *mut Size) -> Hr = element.slot(6);
            check(get(element.raw(), &mut desired))?;
        }
        Ok((desired.width as f64, desired.height as f64))
    }
    /// `Control.Focus(FocusState.Keyboard)`.
    pub fn focus(element: &Com) -> Result<()> {
        let control = element.query(&Guid::from_u128(0xa8912263_2951_4f58_a9c5_5a134eaa7f07))?;
        let mut focused = 0u8;
        unsafe {
            let focus: unsafe extern "system" fn(Raw, i32, *mut u8) -> Hr = control.slot(46);
            check(focus(control.raw(), 2, &mut focused))
        }
    }
    /// `Control.FocusState` is `Keyboard`.
    pub fn keyboard_focused(element: &Com) -> Result<bool> {
        let control = element.query(&Guid::from_u128(0xa8912263_2951_4f58_a9c5_5a134eaa7f07))?;
        let mut state = 0;
        unsafe {
            let get: unsafe extern "system" fn(Raw, *mut i32) -> Hr = control.slot(42);
            check(get(control.raw(), &mut state))?;
        }
        Ok(state == 2)
    }
    /// `ScrollViewer.VerticalOffset` and `ViewportHeight`.
    pub fn scroll(element: &Com) -> Result<(f64, f64)> {
        let viewer = element.query(&Guid::from_u128(0x64e9be00_4dc1_493d_abe7_cbd3c577490d))?;
        let mut values = [0.0; 2];
        for (slot, value) in [45, 46].into_iter().zip(&mut values) {
            unsafe {
                let get: unsafe extern "system" fn(Raw, *mut f64) -> Hr = viewer.slot(slot);
                check(get(viewer.raw(), value))?;
            }
        }
        Ok((values[0], values[1]))
    }
    /// `UIElement.Opacity`.
    pub fn opacity(element: &Com, value: f64) -> Result<()> {
        let element = element.query(&Guid::from_u128(0x676d0be9_b65c_41c6_ba40_58cf87f201c1))?;
        unsafe {
            let set: unsafe extern "system" fn(Raw, f64) -> Hr = element.slot(10);
            check(set(element.raw(), value))
        }
    }
    /// `ToggleButton.IsChecked`, a boxed `IReference<bool>`.
    pub fn checked(element: &Com, value: bool) -> Result<()> {
        let toggle = element.query(&Guid::from_u128(0x589877fb_0fc7_4036_9d8b_127dfa75c16d))?;
        let values = factory(
            "Windows.Foundation.PropertyValue",
            &Guid::from_u128(0x629bdbc8_d932_4ff4_96b9_8d96c5c1e858),
        )?;
        let mut boxed = ptr::null_mut();
        unsafe {
            let create: unsafe extern "system" fn(Raw, u8, *mut Raw) -> Hr = values.slot(17);
            check(create(values.raw(), u8::from(value), &mut boxed))?;
            let boxed = Com::owned(boxed)?
                .query(&Guid::from_u128(0x3c00fd60_2950_5939_a21a_2d12c5a01b8a))?;
            let set: unsafe extern "system" fn(Raw, Raw) -> Hr = toggle.slot(7);
            check(set(toggle.raw(), boxed.raw()))
        }
    }
    /// `ToggleButton.IsChecked`; the indeterminate (null) state reads as unchecked.
    pub fn is_checked(element: &Com) -> Result<bool> {
        let toggle = element.query(&Guid::from_u128(0x589877fb_0fc7_4036_9d8b_127dfa75c16d))?;
        unsafe {
            let get: unsafe extern "system" fn(Raw, *mut Raw) -> Hr = toggle.slot(6);
            let mut boxed = ptr::null_mut();
            check(get(toggle.raw(), &mut boxed))?;
            if boxed.is_null() {
                return Ok(false);
            }
            let boxed = Com::owned(boxed)?;
            let value: unsafe extern "system" fn(Raw, *mut u8) -> Hr = boxed.slot(6);
            let mut checked = 0u8;
            check(value(boxed.raw(), &mut checked))?;
            Ok(checked != 0)
        }
    }
    /// `Control.IsEnabled`.
    pub fn enable(element: &Com, enabled: bool) -> Result<()> {
        let control = element.query(&Guid::from_u128(0xa8912263_2951_4f58_a9c5_5a134eaa7f07))?;
        unsafe {
            let set: unsafe extern "system" fn(Raw, u8) -> Hr = control.slot(23);
            check(set(control.raw(), u8::from(enabled)))
        }
    }
    /// `RangeBase.Value` of `control`.
    pub fn range(control: &Com, value: Option<f64>) -> Result<f64> {
        let control = control.query(&Guid::from_u128(0xfa002c1a_494e_46cf_91d4_e14a8d798675))?;
        unsafe {
            if let Some(value) = value {
                let set: unsafe extern "system" fn(Raw, f64) -> Hr = control.slot(15);
                check(set(control.raw(), value))?;
                Ok(value)
            } else {
                let get: unsafe extern "system" fn(Raw, *mut f64) -> Hr = control.slot(14);
                let mut value = 0.0;
                check(get(control.raw(), &mut value))?;
                Ok(value)
            }
        }
    }
    /// `FlyoutBase.Hide`.
    pub fn hide_flyout(flyout: &Com) -> Result<()> {
        let flyout = flyout.query(&Guid::from_u128(0x723eea0b_d12e_430d_a9f0_9bb32bbf9913))?;
        unsafe {
            let hide: unsafe extern "system" fn(Raw) -> Hr = flyout.slot(15);
            check(hide(flyout.raw()))
        }
    }
    pub fn find(root: &Com, name: &str) -> Result<Com> {
        playground::Playground::find(root, name)
    }
    pub fn text(root: &Com, name: &str, value: &str) -> Result<()> {
        Self::find(root, name)?
            .query(&Guid::from_u128(0xae2d9271_3b4a_45fc_8468_f7949548f4d5))?
            .set_string(27, value)
    }
    pub fn load(markup: &str) -> Result<Com> {
        let factory = factory(
            "Windows.UI.Xaml.Markup.XamlReader",
            &Guid::from_u128(0x9891c6bd_534f_4955_b85a_8a8dc0dca602),
        )?;
        let text = HString::new(markup)?;
        let mut raw = ptr::null_mut();
        unsafe {
            let load: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = factory.slot(6);
            check(load(factory.raw(), text.0, &mut raw))?;
            Com::owned(raw)
        }
    }
    pub fn children(host: &Com, child: &Com) -> Result<()> {
        let children = host.query(&crate::platform::xaml::PANEL)?.object(6)?;
        let child = child.query(&Guid::from_u128(0x676d0be9_b65c_41c6_ba40_58cf87f201c1))?;
        unsafe {
            let clear: unsafe extern "system" fn(Raw) -> Hr = children.slot(15);
            let append: unsafe extern "system" fn(Raw, Raw) -> Hr = children.slot(13);
            check(clear(children.raw()))?;
            check(append(children.raw(), child.raw()))
        }
    }
    pub fn content(host: &Com, child: &Com) -> Result<()> {
        let host = host.query(&Guid::from_u128(0xa26dd1dc_cd44_435c_be94_01d6241c231c))?;
        unsafe {
            let set: unsafe extern "system" fn(Raw, Raw) -> Hr = host.slot(7);
            check(set(host.raw(), child.raw()))
        }
    }
    pub fn visible(root: &Com, name: &str, visible: bool) -> Result<()> {
        Self::show(&Self::find(root, name)?, visible)
    }
    pub fn show(element: &Com, visible: bool) -> Result<()> {
        let element = element.query(&Guid::from_u128(0x676d0be9_b65c_41c6_ba40_58cf87f201c1))?;
        unsafe {
            let set: unsafe extern "system" fn(Raw, i32) -> Hr = element.slot(22);
            check(set(element.raw(), if visible { 0 } else { 1 }))
        }
    }
    pub fn xml(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

/// Markup each named host shows; loading the same markup into it again is skipped.
/// Only hosts filled through it may be named, and it is cleared with the page.
#[derive(Default)]
pub struct Shown(std::cell::RefCell<std::collections::HashMap<String, String>>);
impl Shown {
    /// `Ui::content` of `markup` into the host `name` under `root`.
    pub fn content(&self, root: &Com, name: &str, markup: &str) -> Result<()> {
        self.show(name, markup, || {
            Ui::content(&Ui::find(root, name)?, &Ui::load(markup)?)
        })
    }
    /// `Ui::children` of `markup` into the panel `name` under `root`.
    pub fn children(&self, root: &Com, name: &str, markup: &str) -> Result<()> {
        self.show(name, markup, || {
            Ui::children(&Ui::find(root, name)?, &Ui::load(markup)?)
        })
    }
    fn show(&self, name: &str, markup: &str, load: impl FnOnce() -> Result<()>) -> Result<()> {
        if self
            .0
            .borrow()
            .get(name)
            .is_some_and(|shown| shown == markup)
        {
            return Ok(());
        }
        self.0.borrow_mut().remove(name);
        load()?;
        self.0
            .borrow_mut()
            .insert(name.to_owned(), markup.to_owned());
        Ok(())
    }
    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }
}
