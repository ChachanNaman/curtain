//! Launch-at-login toggle (PRD F9). Thin wrapper over
//! `tauri-plugin-autostart` so the Privacy tab has something to bind to.

use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn set_autostart<R: Runtime>(app: AppHandle<R>, enabled: bool) -> Result<(), String> {
    let mgr = app.autolaunch();
    if enabled {
        mgr.enable().map_err(|e| e.to_string())
    } else {
        mgr.disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn is_autostart_enabled<R: Runtime>(app: AppHandle<R>) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}
