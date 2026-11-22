//! Per-display blackout covers (PRD F1). Platform work happens on the
//! main thread; Tauri commands dispatch there via `run_on_main_thread`
//! and reply through a oneshot channel so the async command can await it.

#[cfg(target_os = "macos")]
mod display_id;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod window_display;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
mod window_display_windows;

#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
use windows as platform;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

#[cfg(target_os = "macos")]
pub use macos::DisplayInfo;
#[cfg(target_os = "windows")]
pub use windows::DisplayInfo;

#[derive(Serialize, Clone)]
pub struct CoverState {
    pub display_count: u32,
    pub covered: Vec<u32>,
}

fn emit_state<R: Runtime>(app: &AppHandle<R>) {
    let state = CoverState {
        display_count: platform::display_count(),
        covered: platform::covered_indices(),
    };
    let _ = app.emit("cover://state", state);
}

/// Run `f` on the main thread and emit the updated cover state afterward.
fn on_main<R: Runtime>(app: &AppHandle<R>, f: impl FnOnce() + Send + 'static) {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        f();
        emit_state(&app2);
    });
}

#[tauri::command]
pub fn cover_display<R: Runtime>(app: AppHandle<R>, index: u32) {
    on_main(&app, move || platform::cover_display(index));
}

#[tauri::command]
pub fn uncover_display<R: Runtime>(app: AppHandle<R>, index: u32) {
    if !crate::auth::authorize_restore() {
        return;
    }
    on_main(&app, move || platform::uncover_display(index));
}

#[tauri::command]
pub fn cover_all<R: Runtime>(app: AppHandle<R>) {
    on_main(&app, platform::cover_all);
}

#[tauri::command]
pub fn uncover_all<R: Runtime>(app: AppHandle<R>) {
    if !crate::auth::authorize_restore() {
        return;
    }
    on_main(&app, platform::uncover_all);
}

#[tauri::command]
pub fn cover_state<R: Runtime>(app: AppHandle<R>) -> CoverState {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(CoverState {
            display_count: platform::display_count(),
            covered: platform::covered_indices(),
        });
    });
    rx.recv().unwrap_or(CoverState {
        display_count: 0,
        covered: Vec::new(),
    })
}

#[tauri::command]
pub fn get_displays<R: Runtime>(app: AppHandle<R>) -> Vec<DisplayInfo> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(platform::display_info());
    });
    rx.recv().unwrap_or_default()
}

#[tauri::command]
pub fn set_dim<R: Runtime>(app: AppHandle<R>, index: u32, percent: u8) {
    let _ = app.run_on_main_thread(move || platform::set_dim(index, percent));
}

#[tauri::command]
pub fn dim_state<R: Runtime>(app: AppHandle<R>) -> Vec<(u32, u8)> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(platform::dim_state());
    });
    rx.recv().unwrap_or_default()
}

/// Current display index for a stable display ID, or `None` if that
/// display isn't connected right now. Used by hotkey dispatch so a
/// per-display binding keeps working after monitors are replugged in a
/// different order.
pub fn index_for_stable_id<R: Runtime>(app: AppHandle<R>, stable_id: String) -> Option<u32> {
    get_displays(app)
        .into_iter()
        .find(|d| d.stable_id.as_deref() == Some(stable_id.as_str()))
        .map(|d| d.index)
}

/// Which display index holds `pid`'s frontmost window, if resolvable.
pub fn display_index_for_pid<R: Runtime>(app: &AppHandle<R>, pid: u32) -> Option<u32> {
    #[cfg(target_os = "macos")]
    {
        window_display::display_index_for_pid(app, pid)
    }
    #[cfg(target_os = "windows")]
    {
        window_display_windows::display_index_for_pid(app, pid)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (app, pid);
        None
    }
}

/// F13 "uncover-on-alert": lift the cover only on the display holding
/// `pid`'s frontmost window, leaving every other covered display
/// alone. No-op if that display isn't currently covered, if the
/// display can't be resolved, or if F8's unlock gate is on and the
/// user isn't authorized.
pub fn uncover_on_alert<R: Runtime>(app: &AppHandle<R>, pid: u32) {
    if pid == 0 || !crate::auth::authorize_restore() {
        return;
    }
    let Some(index) = display_index_for_pid(app, pid) else { return };
    on_main(app, move || platform::uncover_display(index));
}

/// Hard-coded emergency restore (⌃⌥⌘Esc on Mac). Requires auth when F8
/// is on (falls back to device password, so the user can never be
/// locked out), same as a normal restore. Runs the (possibly blocking)
/// auth prompt on its own thread so it never ties up the hotkey
/// dispatch thread.
pub fn emergency_restore<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    std::thread::spawn(move || {
        if crate::auth::authorize_restore() {
            on_main(&app, platform::uncover_all);
        }
    });
}
