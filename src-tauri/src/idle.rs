//! Idle auto-cover (PRD F10): cover all screens after N minutes with no
//! input. macOS via `CGEventSourceSecondsSinceLastEventType`, Windows
//! via `GetLastInputInfo`; both polled from the same 3s ticker the
//! agent/keep-awake loop already runs on.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::{AppHandle, Runtime};

static ENABLED: AtomicBool = AtomicBool::new(false);
static THRESHOLD_SECS: AtomicU64 = AtomicU64::new(10 * 60);
/// Set once auto-cover fires, so we don't re-trigger every tick while
/// the user stays away; cleared the moment idle time drops (they're back).
static ALREADY_FIRED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "macos")]
fn idle_seconds() -> f64 {
    const K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: i32 = 1;
    const K_CG_ANY_INPUT_EVENT_TYPE: u32 = u32::MAX;

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
    }
    unsafe { CGEventSourceSecondsSinceLastEventType(K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE, K_CG_ANY_INPUT_EVENT_TYPE) }
}

#[cfg(target_os = "windows")]
fn idle_seconds() -> f64 {
    use windows::Win32::System::SystemInformation::GetTickCount64;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut info = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetLastInputInfo(&mut info) }.as_bool() {
        // Both GetTickCount64 and dwTime tick in ms since boot;
        // dwTime is the 32-bit truncated form, so compare against the
        // low 32 bits of the 64-bit tick count rather than the full
        // value to avoid a false-huge idle reading after ~49.7 days
        // of uptime (the same wraparound window dwTime itself has).
        let now_low = (unsafe { GetTickCount64() } & 0xFFFF_FFFF) as u32;
        (now_low.wrapping_sub(info.dwTime) as f64) / 1000.0
    } else {
        0.0
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn idle_seconds() -> f64 {
    0.0
}

#[tauri::command]
pub fn set_idle_auto_cover(enabled: bool, minutes: u64) {
    ENABLED.store(enabled, Ordering::SeqCst);
    THRESHOLD_SECS.store(minutes.max(1) * 60, Ordering::SeqCst);
    ALREADY_FIRED.store(false, Ordering::SeqCst);
}

#[tauri::command]
pub fn get_idle_auto_cover() -> (bool, u64) {
    (ENABLED.load(Ordering::SeqCst), THRESHOLD_SECS.load(Ordering::SeqCst) / 60)
}

/// Called from the 3s ticker. Covers all screens once idle time crosses
/// the threshold; resets so it can fire again next time they step away.
pub fn tick<R: Runtime>(app: &AppHandle<R>) {
    if !ENABLED.load(Ordering::SeqCst) {
        return;
    }
    let idle = idle_seconds();
    let threshold = THRESHOLD_SECS.load(Ordering::SeqCst) as f64;
    if idle >= threshold {
        if !ALREADY_FIRED.swap(true, Ordering::SeqCst) {
            crate::cover::cover_all(app.clone());
        }
    } else {
        ALREADY_FIRED.store(false, Ordering::SeqCst);
    }
}
