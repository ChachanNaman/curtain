//! Smart Keep Awake (PRD F3) — macOS implementation via an IOKit power
//! assertion. `PreventUserIdleSystemSleep` stops the *system* from
//! sleeping but deliberately leaves the display free to sleep on its own,
//! matching the PRD's "screens may still sleep" requirement.
//!
//! `io-kit-sys` doesn't bind the IOPMAssertion* functions, so these are
//! declared directly against IOKit.framework.

use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use std::sync::Mutex;

type IOPMAssertionID = u32;
type IOReturn = i32;

const K_IO_RETURN_SUCCESS: IOReturn = 0;
const K_IO_PM_ASSERTION_LEVEL_ON: u32 = 255;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        assertion_type: CFStringRef,
        assertion_level: u32,
        assertion_name: CFStringRef,
        assertion_id: *mut IOPMAssertionID,
    ) -> IOReturn;

    fn IOPMAssertionRelease(assertion_id: IOPMAssertionID) -> IOReturn;
}

static ASSERTION: Mutex<Option<IOPMAssertionID>> = Mutex::new(None);

const ASSERTION_TYPE: &str = "PreventUserIdleSystemSleep";

/// Start (or keep alive) the keep-awake assertion. Idempotent.
pub fn enable(reason: &str) {
    let mut guard = ASSERTION.lock().unwrap();
    if guard.is_some() {
        return;
    }
    let assertion_type = CFString::new(ASSERTION_TYPE);
    let assertion_name = CFString::new(reason);
    let mut id: IOPMAssertionID = 0;
    let result = unsafe {
        IOPMAssertionCreateWithName(
            assertion_type.as_concrete_TypeRef(),
            K_IO_PM_ASSERTION_LEVEL_ON,
            assertion_name.as_concrete_TypeRef(),
            &mut id,
        )
    };
    if result == K_IO_RETURN_SUCCESS {
        *guard = Some(id);
    }
}

/// Release the keep-awake assertion if one is active. Idempotent.
pub fn disable() {
    let mut guard = ASSERTION.lock().unwrap();
    if let Some(id) = guard.take() {
        unsafe {
            IOPMAssertionRelease(id);
        }
    }
}

pub fn is_enabled() -> bool {
    ASSERTION.lock().unwrap().is_some()
}
