//! Touch ID / Windows Hello unlock (PRD F8). Off by default. When on,
//! restoring any cover requires biometric or device-password auth; the
//! emergency restore hotkey still works, but also requires auth when
//! this is enabled — falling back to the device password means the
//! user can never be fully locked out.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
use windows as platform;

use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub fn set_unlock_required(enabled: bool) {
    ENABLED.store(enabled, Ordering::SeqCst);
}

#[tauri::command]
pub fn is_unlock_required() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

/// Returns true if restore should proceed: either auth isn't required,
/// or the user successfully authenticated.
pub fn authorize_restore() -> bool {
    if !ENABLED.load(Ordering::SeqCst) {
        return true;
    }
    platform::authenticate("restore your covered screens")
}
