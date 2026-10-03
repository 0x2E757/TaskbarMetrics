//! COM TAP entry points. Subscription never runs inline in SetSite or DllMain.
mod islands;
mod session;
use super::{
    abi::*,
    com::*,
    composition::Composition,
    xaml::{self, Target},
};
use crate::{application::MetricSink, config::Settings};
use session::Session;
use std::{
    collections::HashMap,
    ptr,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
};

#[repr(C)]
#[derive(Clone, Copy)]
struct SourceInfo {
    file: Raw,
    line: u32,
    column: u32,
    position: u32,
    hash: Raw,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Relation {
    parent: u64,
    child: u64,
    index: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Element {
    handle: u64,
    source: SourceInfo,
    kind: Raw,
    name: Raw,
    children: u32,
}
#[repr(C)]
struct SiteVtbl {
    base: UnknownVtbl,
    set_site: unsafe extern "system" fn(Raw, Raw) -> Hr,
    get_site: unsafe extern "system" fn(Raw, *const Guid, *mut Raw) -> Hr,
}
#[repr(C)]
struct CallbackVtbl {
    base: UnknownVtbl,
    change: unsafe extern "system" fn(Raw, Relation, Element, i32) -> Hr,
    state: unsafe extern "system" fn(Raw, u64, i32, *const u16) -> Hr,
}
#[repr(C)]
struct Tap {
    site_vtable: &'static SiteVtbl,
    callback_vtable: &'static CallbackVtbl,
    refs: AtomicU32,
    session: OnceLock<Arc<Session>>,
}
// Holds the stop marker and session state as long as the pinned DLL exists.
static SESSION: OnceLock<Arc<Session>> = OnceLock::new();
static NEXT_TARGET: AtomicU64 = AtomicU64::new(1);
unsafe fn tap_from_callback(raw: Raw) -> *mut Tap {
    (raw as *mut u8)
        .sub(std::mem::offset_of!(Tap, callback_vtable))
        .cast()
}
unsafe fn callback_ptr(tap: *mut Tap) -> Raw {
    ptr::addr_of_mut!((*tap).callback_vtable).cast()
}
unsafe extern "system" fn tap_query(raw: Raw, iid: *const Guid, out: *mut Raw) -> Hr {
    if iid.is_null() || out.is_null() {
        return E_POINTER;
    }
    *out = ptr::null_mut();
    if *iid == UNKNOWN || *iid == OBJECT_WITH_SITE || *iid == AGILE_OBJECT {
        *out = raw;
    } else if *iid == CALLBACK || *iid == CALLBACK2 {
        *out = callback_ptr(raw.cast());
    } else {
        return E_NOINTERFACE;
    }
    tap_add(raw);
    0
}
unsafe extern "system" fn tap_add(raw: Raw) -> u32 {
    (*(raw as *const Tap)).refs.fetch_add(1, Ordering::Relaxed) + 1
}
unsafe extern "system" fn tap_release(raw: Raw) -> u32 {
    let count = (*(raw as *const Tap)).refs.fetch_sub(1, Ordering::AcqRel) - 1;
    if count == 0 {
        drop(Box::from_raw(raw as *mut Tap));
    }
    count
}
unsafe extern "system" fn callback_query(raw: Raw, iid: *const Guid, out: *mut Raw) -> Hr {
    tap_query(tap_from_callback(raw).cast(), iid, out)
}
unsafe extern "system" fn callback_add(raw: Raw) -> u32 {
    tap_add(tap_from_callback(raw).cast())
}
unsafe extern "system" fn callback_release(raw: Raw) -> u32 {
    tap_release(tap_from_callback(raw).cast())
}
unsafe extern "system" fn on_change(
    raw: Raw,
    relation: Relation,
    element: Element,
    mutation: i32,
) -> Hr {
    let hr = boundary(|| {
        if let Some(session) = (*tap_from_callback(raw)).session.get() {
            session.change(relation, element, mutation)?;
        }
        Ok(())
    });
    if hr < 0 {
        log(&format!("Tree callback failed: 0x{:08X}", hr as u32));
    }
    0
}
unsafe extern "system" fn on_state(_: Raw, _: u64, _: i32, _: *const u16) -> Hr {
    0
}

// One owning reference to our free-threaded COM object (not a XAML object).
struct TapRef(*mut Tap);
unsafe impl Send for TapRef {}
impl TapRef {
    fn work(self, session: Arc<Session>) {
        let hr = boundary(|| {
            let _apartment = Apartment::mta()?;
            unsafe {
                let pointer = callback_ptr(self.0);
                callback_add(pointer);
                let callback = Com::owned(pointer)?;
                loop {
                    let result = boundary(|| session.run(&callback));
                    session.cleanup();
                    if result < 0 {
                        log(&format!(
                            "Session stopped with HRESULT 0x{:08X}",
                            result as u32
                        ));
                    }
                    SetEvent(session.stop.0);
                    SetEvent(session.idle.0);
                    // Keep the pinned adapter reusable; no sampling or polling while stopped.
                    if WaitForSingleObject(session.resume.0, u32::MAX) != 0 {
                        return Err(last_error());
                    }
                    ResetEvent(session.idle.0);
                    ResetEvent(session.stop.0);
                }
            }
        });
        if hr < 0 {
            log(&format!("Worker failed: 0x{:08X}", hr as u32));
            session.stopping.store(true, Ordering::Release);
        }
    }
}
impl Drop for TapRef {
    fn drop(&mut self) {
        unsafe {
            tap_release(self.0.cast());
        }
    }
}
unsafe extern "system" fn set_site(raw: Raw, site: Raw) -> Hr {
    boundary(|| (*(raw as *const Tap)).set_site(site))
}
impl Tap {
    unsafe fn set_site(&self, site: Raw) -> Result<()> {
        let tap = self;
        if site.is_null() {
            if let Some(session) = tap.session.get() {
                SetEvent(session.stop.0);
            }
            return Ok(());
        }
        if tap.session.get().is_some() || SESSION.get().is_some() {
            return Err(E_UNEXPECTED);
        }
        super::launcher::WindowsSupport::validate()?;
        // Borrow site without stealing the caller's reference.
        let borrowed = std::mem::ManuallyDrop::new(Com::owned(site)?);
        let diagnostics = borrowed.query(&DIAGNOSTICS)?;
        let _service = diagnostics.query(&TREE_SERVICE)?;
        let site = Agile::new(&borrowed, &UNKNOWN)?;
        let diagnostics = Agile::new(&diagnostics, &DIAGNOSTICS)?;
        // PIN the module by an address in it. No loading, threads or COM in DllMain.
        let mut module = ptr::null_mut();
        if GetModuleHandleExW(
            0x1 | 0x4,
            DllGetClassObject as *const () as *const u16,
            &mut module,
        ) == 0
        {
            return Err(last_error());
        }
        let data = super::data_directory::DataDirectory::path().map_err(|_| E_FAIL)?;
        super::logging::DiagnosticLog::initialize(&data.join("taskbar-metrics.log"));
        let config = data.join("taskbar-metrics.conf");
        let settings = Settings::load(&config).map_err(|error| {
            log(&error);
            E_FAIL
        })?;
        let _ = Composition::monitor(&settings).map_err(|error| {
            log(&error);
            E_FAIL
        })?;
        let stop = Handle::new(CreateEventW(
            ptr::null_mut(),
            1,
            0,
            event_name("stop", GetCurrentProcessId()).as_ptr(),
        ))?;
        let resume = Handle::new(CreateEventW(
            ptr::null_mut(),
            0,
            0,
            event_name("resume", GetCurrentProcessId()).as_ptr(),
        ))?;
        let idle = Handle::new(CreateEventW(
            ptr::null_mut(),
            1,
            0,
            event_name("idle", GetCurrentProcessId()).as_ptr(),
        ))?;
        let session = Arc::new(Session {
            site,
            diagnostics,
            stop,
            resume,
            idle,
            stopping: AtomicBool::new(false),
            targets: Mutex::new(HashMap::new()),
            islands: Mutex::default(),
            settings: Mutex::new(settings),
            config,
        });
        SESSION.set(session.clone()).map_err(|_| E_UNEXPECTED)?;
        tap.session.set(session.clone()).map_err(|_| E_UNEXPECTED)?;
        let raw = self as *const Self as Raw;
        tap_add(raw);
        let worker_ref = TapRef(raw.cast());
        std::thread::Builder::new()
            .name("taskbar-stats".into())
            .spawn(move || worker_ref.work(session))
            .map_err(|_| E_FAIL)?;
        Ok(())
    }
}
unsafe extern "system" fn get_site(raw: Raw, iid: *const Guid, out: *mut Raw) -> Hr {
    if iid.is_null() || out.is_null() {
        return E_POINTER;
    }
    *out = ptr::null_mut();
    boundary(|| {
        let session = (*(raw as *const Tap)).session.get().ok_or(E_FAIL)?;
        *out = session.site.resolve(&*iid)?.into_raw();
        Ok(())
    })
}
static SITE_VTABLE: SiteVtbl = SiteVtbl {
    base: UnknownVtbl {
        query: tap_query,
        add_ref: tap_add,
        release: tap_release,
    },
    set_site,
    get_site,
};
static CALLBACK_VTABLE: CallbackVtbl = CallbackVtbl {
    base: UnknownVtbl {
        query: callback_query,
        add_ref: callback_add,
        release: callback_release,
    },
    change: on_change,
    state: on_state,
};
#[repr(C)]
struct FactoryVtbl {
    base: UnknownVtbl,
    create: unsafe extern "system" fn(Raw, Raw, *const Guid, *mut Raw) -> Hr,
    lock: unsafe extern "system" fn(Raw, i32) -> Hr,
}
#[repr(C)]
struct Factory {
    vtable: &'static FactoryVtbl,
    refs: AtomicU32,
}
unsafe extern "system" fn factory_query(raw: Raw, iid: *const Guid, out: *mut Raw) -> Hr {
    if iid.is_null() || out.is_null() {
        return E_POINTER;
    }
    *out = ptr::null_mut();
    if *iid != UNKNOWN && *iid != CLASS_FACTORY {
        return E_NOINTERFACE;
    }
    *out = raw;
    factory_add(raw);
    0
}
unsafe extern "system" fn factory_add(raw: Raw) -> u32 {
    (*(raw as *const Factory))
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "system" fn factory_release(raw: Raw) -> u32 {
    let count = (*(raw as *const Factory))
        .refs
        .fetch_sub(1, Ordering::AcqRel)
        - 1;
    if count == 0 {
        drop(Box::from_raw(raw as *mut Factory));
    }
    count
}
unsafe extern "system" fn create(_: Raw, outer: Raw, iid: *const Guid, out: *mut Raw) -> Hr {
    if out.is_null() || iid.is_null() {
        return E_POINTER;
    }
    *out = ptr::null_mut();
    if !outer.is_null() {
        return CLASS_E_NOAGGREGATION;
    }
    boundary(|| {
        let tap = Box::new(Tap {
            site_vtable: &SITE_VTABLE,
            callback_vtable: &CALLBACK_VTABLE,
            refs: AtomicU32::new(1),
            session: OnceLock::new(),
        });
        let object = Com::owned(Box::into_raw(tap).cast())?;
        *out = object.query(&*iid)?.into_raw();
        Ok(())
    })
}
unsafe extern "system" fn lock(_: Raw, _: i32) -> Hr {
    0
}
static FACTORY_VTABLE: FactoryVtbl = FactoryVtbl {
    base: UnknownVtbl {
        query: factory_query,
        add_ref: factory_add,
        release: factory_release,
    },
    create,
    lock,
};

#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    clsid: *const Guid,
    iid: *const Guid,
    out: *mut Raw,
) -> Hr {
    if clsid.is_null() || iid.is_null() || out.is_null() {
        return E_POINTER;
    }
    *out = ptr::null_mut();
    if *clsid != CLSID {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    boundary(|| {
        let factory = Box::new(Factory {
            vtable: &FACTORY_VTABLE,
            refs: AtomicU32::new(1),
        });
        let object = Com::owned(Box::into_raw(factory).cast())?;
        *out = object.query(&*iid)?.into_raw();
        Ok(())
    })
}
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> Hr {
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn factory_interfaces_and_com_identity() {
        unsafe {
            let mut out = ptr::null_mut();
            assert_eq!(
                DllGetClassObject(&UNKNOWN, &CLASS_FACTORY, &mut out),
                CLASS_E_CLASSNOTAVAILABLE
            );
            assert!(out.is_null());
            assert_eq!(DllGetClassObject(&CLSID, &CLASS_FACTORY, &mut out), 0);
            let factory = Com::owned(out).unwrap();
            let create: unsafe extern "system" fn(Raw, Raw, *const Guid, *mut Raw) -> Hr =
                factory.slot(3);
            assert_eq!(
                create(factory.raw(), factory.raw(), &OBJECT_WITH_SITE, &mut out),
                CLASS_E_NOAGGREGATION
            );
            assert_eq!(
                create(factory.raw(), ptr::null_mut(), &OBJECT_WITH_SITE, &mut out),
                0
            );
            let site = Com::owned(out).unwrap();
            let callback = site.query(&CALLBACK2).unwrap();
            assert_ne!(site.raw(), callback.raw());
            assert_eq!(
                site.query(&UNKNOWN).unwrap().raw(),
                callback.query(&UNKNOWN).unwrap().raw()
            );
            assert!(callback.query(&CALLBACK).is_ok());
            assert!(site.query(&DIAGNOSTICS).is_err());
            let get: unsafe extern "system" fn(Raw, *const Guid, *mut Raw) -> Hr = site.slot(4);
            assert_eq!(get(site.raw(), &UNKNOWN, &mut out), E_FAIL);
            assert!(out.is_null());
            assert_eq!(DllCanUnloadNow(), 1);
        }
    }
    #[test]
    fn sdk_layouts() {
        assert_eq!(std::mem::size_of::<SourceInfo>(), 32);
        assert_eq!(std::mem::size_of::<Relation>(), 24);
        assert_eq!(std::mem::size_of::<Element>(), 64);
        assert_eq!(std::mem::offset_of!(Element, kind), 40);
    }
}
