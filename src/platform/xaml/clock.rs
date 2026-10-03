use super::{visual_tree::VisualTree, *};
use std::ptr;

const WEAK_SOURCE: Guid = Guid::from_u128(0x00000038_0000_0000_c000_000000000046);
thread_local! { static CLOCKS: RefCell<HashMap<u64, Com>> = RefCell::new(HashMap::new()); }

/// The tray and task-list can be separate XAML roots. Diagnostics discovers both.
/// Only weak references are retained, exclusively on the clock's own UI thread.
pub(crate) struct ClockCatalog;
impl ClockCatalog {
    pub fn register(handle: u64, clock: &Com) -> Result<()> {
        let weak = clock.query(&WEAK_SOURCE)?.object(3)?;
        CLOCKS.with(|clocks| {
            clocks.borrow_mut().insert(handle, weak);
        });
        log("Native clock discovered on its UI thread");
        Ok(())
    }

    pub fn remove(handle: u64) {
        CLOCKS.with(|clocks| {
            clocks.borrow_mut().remove(&handle);
        });
    }

    pub(super) fn find(panel: &Com) -> Result<Com> {
        if let Some(clock) = VisualTree::new()?.find(panel, "NotificationCenterButton")? {
            return Ok(clock);
        }
        let references =
            CLOCKS.with(|clocks| clocks.borrow().values().cloned().collect::<Vec<_>>());
        for reference in references {
            let mut object = ptr::null_mut();
            unsafe {
                let resolve: unsafe extern "system" fn(Raw, *const Guid, *mut Raw) -> Hr =
                    reference.slot(3);
                check(resolve(reference.raw(), &FRAMEWORK, &mut object))?;
                if !object.is_null() {
                    return Com::owned(object);
                }
            }
        }
        Err(E_UNEXPECTED)
    }
}
