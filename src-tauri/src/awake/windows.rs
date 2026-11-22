//! Smart Keep Awake (PRD F3) — Windows implementation via
//! `SetThreadExecutionState`. `ES_SYSTEM_REQUIRED` alone (deliberately
//! without `ES_DISPLAY_REQUIRED`) stops the system from sleeping but
//! leaves the display free to sleep on its own, matching the macOS
//! side's `PreventUserIdleSystemSleep` behavior and the PRD's "screens
//! may still sleep" requirement. `ES_CONTINUOUS` makes the state
//! persist until explicitly cleared rather than being a one-shot reset
//! of the idle timer.

use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED};

static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn enable(_reason: &str) {
    unsafe {
        SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
    }
    ENABLED.store(true, Ordering::SeqCst);
}

pub fn disable() {
    unsafe {
        SetThreadExecutionState(ES_CONTINUOUS);
    }
    ENABLED.store(false, Ordering::SeqCst);
}

pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}
