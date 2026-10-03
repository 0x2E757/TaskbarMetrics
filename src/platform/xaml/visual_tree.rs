use super::*;
use std::ptr;

/// Bounded tree lookup used only for view construction and style refresh.
pub(super) struct VisualTree {
    helper: Com,
}

impl VisualTree {
    pub(super) fn new() -> Result<Self> {
        Ok(Self {
            helper: factory("Windows.UI.Xaml.Media.VisualTreeHelper", &TREE_HELPER)?,
        })
    }

    pub(super) fn find(&self, root: &Com, name: &str) -> Result<Option<Com>> {
        self.visit(root, name, 0, &mut 2048)
    }

    pub(super) fn ancestors(&self, element: &Com) -> Result<Vec<Com>> {
        let mut current = element.query(&DEPENDENCY_OBJECT)?;
        let mut result = Vec::new();
        for _ in 0..64 {
            result.push(current.clone());
            let mut parent = ptr::null_mut();
            unsafe {
                let get: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = self.helper.slot(12);
                check(get(self.helper.raw(), current.raw(), &mut parent))?;
                if parent.is_null() {
                    result.reverse();
                    return Ok(result);
                }
                current = Com::owned(parent)?;
            }
        }
        Err(E_UNEXPECTED)
    }

    fn visit(
        &self,
        object: &Com,
        name: &str,
        depth: usize,
        remaining: &mut usize,
    ) -> Result<Option<Com>> {
        if depth > 32 || *remaining == 0 {
            return Err(E_UNEXPECTED);
        }
        *remaining -= 1;
        if let Ok(framework) = object.query(&FRAMEWORK) {
            let current = framework.string(33)?;
            if current == name {
                return Ok(Some(object.clone()));
            }
            if current == "TaskbarMetricsButton" && name == "NotificationCenterButton" {
                return Ok(None);
            }
        }
        let dependency = object.query(&DEPENDENCY_OBJECT)?;
        let mut count = 0;
        unsafe {
            let get: unsafe extern "system" fn(Raw, Raw, *mut i32) -> Hr = self.helper.slot(11);
            check(get(self.helper.raw(), dependency.raw(), &mut count))?;
        }
        if !(0..=2048).contains(&count) {
            return Err(E_UNEXPECTED);
        }
        for index in 0..count {
            let mut child = ptr::null_mut();
            let child = unsafe {
                let get: unsafe extern "system" fn(Raw, Raw, i32, *mut Raw) -> Hr =
                    self.helper.slot(10);
                check(get(self.helper.raw(), dependency.raw(), index, &mut child))?;
                Com::owned(child)?
            };
            if let Some(found) = self.visit(&child, name, depth + 1, remaining)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }
}
