//! Owning XAML event subscriptions; callbacks never unwind across the ABI.
use super::*;
use std::{
    rc::Rc,
    sync::atomic::{AtomicU32, Ordering},
};
const ROUTED: Guid = Guid::from_u128(0xa856e674_b0b6_4bc3_bba8_1ba06e40d4b5);
const POINTER: Guid = Guid::from_u128(0xe4385929_c004_4bcf_8970_359486e39f88);
const BUTTON: Guid = Guid::from_u128(0xfa002c1a_494e_46cf_91d4_e14a8d798674);
const UI_ELEMENT_STATICS: Guid = Guid::from_u128(0x58d3573b_f52c_45be_988b_a5869564873c);
const POINTER_ARGS: Guid = Guid::from_u128(0xda628f0a_9752_49e2_bde2_49eccab9194d);
#[repr(C)]
struct Vtable {
    base: UnknownVtbl,
    invoke: unsafe extern "system" fn(Raw, Raw, Raw) -> Hr,
}
#[repr(C)]
struct Handler {
    vtable: &'static Vtable,
    refs: AtomicU32,
    iid: Guid,
    action: Box<dyn Fn(Raw, Raw) -> Result<()>>,
}
unsafe extern "system" fn query(raw: Raw, iid: *const Guid, out: *mut Raw) -> Hr {
    if iid.is_null() || out.is_null() {
        return E_POINTER;
    }
    *out = std::ptr::null_mut();
    let h = &*(raw as *const Handler);
    if *iid != UNKNOWN && *iid != h.iid {
        return E_NOINTERFACE;
    }
    *out = raw;
    add(raw);
    0
}
unsafe extern "system" fn add(raw: Raw) -> u32 {
    (*(raw as *const Handler))
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "system" fn release(raw: Raw) -> u32 {
    let count = (*(raw as *const Handler))
        .refs
        .fetch_sub(1, Ordering::AcqRel)
        - 1;
    if count == 0 {
        drop(Box::from_raw(raw as *mut Handler));
    }
    count
}
unsafe extern "system" fn invoke(raw: Raw, sender: Raw, args: Raw) -> Hr {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ((*(raw as *const Handler)).action)(sender, args)
    }))
    .unwrap_or(Err(E_FAIL))
    .err()
    .unwrap_or(0)
}
static VTABLE: Vtable = Vtable {
    base: UnknownVtbl {
        query,
        add_ref: add,
        release,
    },
    invoke,
};
/// `IReference<PointerEventHandler>`: `UIElement.AddHandler` takes the
/// delegate boxed, as C++/WinRT's `box_value` does.
const POINTER_REFERENCE: Guid = Guid::from_u128(0xe33644c8_e0e0_5645_8722_60066c85873c);
const INSPECTABLE: Guid = Guid::from_u128(0xaf86e2e0_b12d_4c6a_9c5a_d7aa65101e90);
#[repr(C)]
struct BoxVtable {
    base: UnknownVtbl,
    iids: unsafe extern "system" fn(Raw, *mut u32, *mut Raw) -> Hr,
    class_name: unsafe extern "system" fn(Raw, *mut Raw) -> Hr,
    trust: unsafe extern "system" fn(Raw, *mut i32) -> Hr,
    value: unsafe extern "system" fn(Raw, *mut Raw) -> Hr,
}
#[repr(C)]
struct Boxed {
    vtable: &'static BoxVtable,
    refs: AtomicU32,
    iid: Guid,
    value: Com,
}
impl Boxed {
    fn wrap(iid: Guid, value: Com) -> Result<Com> {
        let boxed = Box::new(Self {
            vtable: &BOX_VTABLE,
            refs: AtomicU32::new(1),
            iid,
            value,
        });
        unsafe { Com::owned(Box::into_raw(boxed).cast()) }
    }
}
unsafe extern "system" fn box_query(raw: Raw, iid: *const Guid, out: *mut Raw) -> Hr {
    if iid.is_null() || out.is_null() {
        return E_POINTER;
    }
    *out = std::ptr::null_mut();
    if ![UNKNOWN, INSPECTABLE, (*(raw as *const Boxed)).iid].contains(&*iid) {
        return E_NOINTERFACE;
    }
    *out = raw;
    box_add(raw);
    0
}
unsafe extern "system" fn box_add(raw: Raw) -> u32 {
    (*(raw as *const Boxed))
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "system" fn box_release(raw: Raw) -> u32 {
    let count = (*(raw as *const Boxed)).refs.fetch_sub(1, Ordering::AcqRel) - 1;
    if count == 0 {
        drop(Box::from_raw(raw as *mut Boxed));
    }
    count
}
unsafe extern "system" fn box_iids(_: Raw, count: *mut u32, iids: *mut Raw) -> Hr {
    *count = 0;
    *iids = std::ptr::null_mut();
    0
}
unsafe extern "system" fn box_class_name(_: Raw, name: *mut Raw) -> Hr {
    *name = std::ptr::null_mut();
    0
}
unsafe extern "system" fn box_trust(_: Raw, level: *mut i32) -> Hr {
    *level = 0;
    0
}
unsafe extern "system" fn box_value(raw: Raw, value: *mut Raw) -> Hr {
    let inner = &(*(raw as *const Boxed)).value;
    let add: unsafe extern "system" fn(Raw) -> u32 = inner.slot(1);
    add(inner.raw());
    *value = inner.raw();
    0
}
static BOX_VTABLE: BoxVtable = BoxVtable {
    base: UnknownVtbl {
        query: box_query,
        add_ref: box_add,
        release: box_release,
    },
    iids: box_iids,
    class_name: box_class_name,
    trust: box_trust,
    value: box_value,
};
enum Registration {
    Token {
        source: Com,
        remove: usize,
        token: i64,
    },
    /// `UIElement.AddHandler`, removed by the same event and handler.
    Routed {
        source: Com,
        event: Com,
        handler: Com,
    },
}
impl Drop for Registration {
    fn drop(&mut self) {
        unsafe {
            match self {
                Self::Token {
                    source,
                    remove,
                    token,
                } => {
                    let remove: unsafe extern "system" fn(Raw, i64) -> Hr = source.slot(*remove);
                    let _ = remove(source.raw(), *token);
                }
                Self::Routed {
                    source,
                    event,
                    handler,
                } => {
                    let remove: unsafe extern "system" fn(Raw, Raw, Raw) -> Hr = source.slot(97);
                    let _ = remove(source.raw(), event.raw(), handler.raw());
                }
            }
        }
    }
}
#[derive(Clone)]
pub(crate) struct Subscription {
    _registration: Rc<Registration>,
}
impl Subscription {
    pub fn click(source: &Com, action: impl Fn() -> Result<()> + 'static) -> Result<Self> {
        Self::new(source.query(&BUTTON)?, ROUTED, 14, move |_, _| action())
    }
    /// UIElement routed events such as GotFocus (45) and LostFocus (47).
    pub fn routed(
        source: &Com,
        slot: usize,
        action: impl Fn() -> Result<()> + 'static,
    ) -> Result<Self> {
        Self::new(source.query(&UI_ELEMENT)?, ROUTED, slot, move |_, _| {
            action()
        })
    }
    pub fn pointer(
        source: &Com,
        slot: usize,
        action: impl Fn(Raw, Raw) -> Result<()> + 'static,
    ) -> Result<Self> {
        Self::new(source.query(&UI_ELEMENT)?, POINTER, slot, action)
    }
    /// Pointer events of `source` and its children, including those a child
    /// already handled (a Button handles its own presses). `event` is the
    /// UIElementStatics getter: PointerPressed 9, Moved 10, Released 11,
    /// CaptureLost 13, Canceled 14.
    pub fn pointer_handled(
        source: &Com,
        event: usize,
        action: impl Fn(Raw, Raw) -> Result<()> + 'static,
    ) -> Result<Self> {
        let source = source.query(&UI_ELEMENT)?;
        let statics = factory("Windows.UI.Xaml.UIElement", &UI_ELEMENT_STATICS)?;
        let handler = Boxed::wrap(POINTER_REFERENCE, Self::handler(POINTER, action)?)?;
        unsafe {
            let get: unsafe extern "system" fn(Raw, *mut Raw) -> Hr = statics.slot(event);
            let mut routed = std::ptr::null_mut();
            check(get(statics.raw(), &mut routed))?;
            let event = Com::owned(routed)?;
            let add: unsafe extern "system" fn(Raw, Raw, Raw, u8) -> Hr = source.slot(96);
            check(add(source.raw(), event.raw(), handler.raw(), 1))?;
            Ok(Self {
                _registration: Rc::new(Registration::Routed {
                    source,
                    event,
                    handler,
                }),
            })
        }
    }
    fn handler(iid: Guid, action: impl Fn(Raw, Raw) -> Result<()> + 'static) -> Result<Com> {
        let handler = Box::new(Handler {
            vtable: &VTABLE,
            refs: AtomicU32::new(1),
            iid,
            action: Box::new(action),
        });
        unsafe { Com::owned(Box::into_raw(handler).cast()) }
    }
    fn new(
        source: Com,
        iid: Guid,
        slot: usize,
        action: impl Fn(Raw, Raw) -> Result<()> + 'static,
    ) -> Result<Self> {
        let handler = Self::handler(iid, action)?;
        unsafe {
            let add: unsafe extern "system" fn(Raw, Raw, *mut i64) -> Hr = source.slot(slot);
            let mut token = 0;
            check(add(source.raw(), handler.raw(), &mut token))?;
            Ok(Self {
                _registration: Rc::new(Registration::Token {
                    source,
                    remove: slot + 1,
                    token,
                }),
            })
        }
    }
    /// Routes further pointer events to `sender` while the button is held (drag).
    pub fn capture(sender: Raw, args: Raw) -> Result<()> {
        unsafe {
            let sender = Self::retain(sender)?.query(&UI_ELEMENT)?;
            let args = Self::retain(args)?.query(&POINTER_ARGS)?;
            let get: unsafe extern "system" fn(Raw, *mut Raw) -> Hr = args.slot(6);
            let mut pointer = std::ptr::null_mut();
            check(get(args.raw(), &mut pointer))?;
            let pointer = Com::owned(pointer)?;
            let capture: unsafe extern "system" fn(Raw, Raw, *mut u8) -> Hr = sender.slot(93);
            let mut captured = 0;
            check(capture(sender.raw(), pointer.raw(), &mut captured))
        }
    }
    /// Borrowed ABI arguments: take our own reference before wrapping.
    unsafe fn retain(raw: Raw) -> Result<Com> {
        if raw.is_null() {
            return Err(E_POINTER);
        }
        let table = *(raw as *const *const Raw);
        let add: unsafe extern "system" fn(Raw) -> u32 = std::mem::transmute(*table.add(1));
        add(raw);
        Com::owned(raw)
    }
    pub fn position(sender: Raw, args: Raw) -> Result<(f32, f32)> {
        unsafe {
            let sender = Self::retain(sender)?.query(&UI_ELEMENT)?;
            let args = Self::retain(args)?.query(&POINTER_ARGS)?;
            let get: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = args.slot(10);
            let mut point = std::ptr::null_mut();
            check(get(args.raw(), sender.raw(), &mut point))?;
            let point = Com::owned(point)?;
            let get: unsafe extern "system" fn(
                Raw,
                *mut crate::presentation::history::ChartPoint,
            ) -> Hr = point.slot(7);
            let mut value = crate::presentation::history::ChartPoint { x: 0.0, y: 0.0 };
            check(get(point.raw(), &mut value))?;
            Ok((value.x, value.y))
        }
    }
    /// The left mouse button, or the contact of a pen or finger, is down.
    pub fn primary(sender: Raw, args: Raw) -> Result<bool> {
        Self::button(sender, args, 16)
    }
    /// The right mouse button is down.
    pub fn secondary(sender: Raw, args: Raw) -> Result<bool> {
        Self::button(sender, args, 17)
    }
    /// `slot` is the IPointerPointProperties getter: IsLeftButtonPressed 16,
    /// IsRightButtonPressed 17.
    fn button(sender: Raw, args: Raw, slot: usize) -> Result<bool> {
        Ok(Self::property::<u8>(sender, args, slot)? != 0)
    }
    /// The turn of the vertical mouse wheel, 120 per notch, positive away from the
    /// user; a horizontal wheel gives 0.
    pub fn wheel(sender: Raw, args: Raw) -> Result<i32> {
        // IsHorizontalMouseWheel 20, MouseWheelDelta 19.
        if Self::property::<u8>(sender, args, 20)? != 0 {
            return Ok(0);
        }
        Self::property::<i32>(sender, args, 19)
    }
    fn property<T: Default>(sender: Raw, args: Raw, slot: usize) -> Result<T> {
        unsafe {
            let sender = Self::retain(sender)?.query(&UI_ELEMENT)?;
            let args = Self::retain(args)?.query(&POINTER_ARGS)?;
            let get: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = args.slot(10);
            let mut point = std::ptr::null_mut();
            check(get(args.raw(), sender.raw(), &mut point))?;
            let properties = Com::owned(point)?.object(13)?;
            let get: unsafe extern "system" fn(Raw, *mut T) -> Hr = properties.slot(slot);
            let mut value = T::default();
            check(get(properties.raw(), &mut value))?;
            Ok(value)
        }
    }
    /// Keeps the event from reaching the taskbar behind `sender`.
    pub fn handle(args: Raw) -> Result<()> {
        unsafe {
            let args = Self::retain(args)?.query(&POINTER_ARGS)?;
            let set: unsafe extern "system" fn(Raw, u8) -> Hr = args.slot(9);
            check(set(args.raw(), 1))
        }
    }
    /// Whether `sender` still holds a pointer capture.
    pub fn captured(sender: Raw) -> Result<bool> {
        unsafe {
            let sender = Self::retain(sender)?.query(&UI_ELEMENT)?;
            let get: unsafe extern "system" fn(Raw, *mut Raw) -> Hr = sender.slot(40);
            let mut captures = std::ptr::null_mut();
            check(get(sender.raw(), &mut captures))?;
            // No capture at all is reported as a null collection.
            if captures.is_null() {
                return Ok(false);
            }
            let captures = Com::owned(captures)?;
            let size: unsafe extern "system" fn(Raw, *mut u32) -> Hr = captures.slot(7);
            let mut count = 0;
            check(size(captures.raw(), &mut count))?;
            Ok(count > 0)
        }
    }
    pub fn release(sender: Raw) -> Result<()> {
        unsafe {
            let sender = Self::retain(sender)?.query(&UI_ELEMENT)?;
            let release: unsafe extern "system" fn(Raw) -> Hr = sender.slot(95);
            check(release(sender.raw()))
        }
    }
}
