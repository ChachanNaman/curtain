//! Fully customizable global hotkeys (PRD F7). Every action is
//! rebindable and persisted; only the emergency restore is fixed and
//! shown read-only in the UI — that's the anti-lockout guarantee.

mod store;

use serde::Serialize;
use std::str::FromStr;
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

use crate::cover;
pub use store::{HotkeyRow, HotkeysFile};

#[cfg(target_os = "macos")]
pub const EMERGENCY_RESTORE: &str = "ctrl+alt+cmd+escape";
#[cfg(not(target_os = "macos"))]
pub const EMERGENCY_RESTORE: &str = "ctrl+alt+shift+escape";

fn emergency_shortcut() -> &'static Shortcut {
    static CELL: OnceLock<Shortcut> = OnceLock::new();
    CELL.get_or_init(|| Shortcut::from_str(EMERGENCY_RESTORE).expect("fixed combo must parse"))
}

/// Re-registers every enabled, bound shortcut against the OS. Called at
/// startup and after any binding change — unregister-all then
/// register-all is simpler and safer than diffing, and this only runs
/// on user-initiated edits, never on a hot path.
pub fn reapply<R: Runtime>(app: &AppHandle<R>) {
    let mgr = app.global_shortcut();
    let _ = mgr.unregister_all();
    let _ = mgr.register(*emergency_shortcut());

    let file = store::load();
    for row in file.actions.values() {
        if !row.enabled {
            continue;
        }
        let Some(combo) = &row.combo else { continue };
        if let Ok(shortcut) = Shortcut::from_str(combo) {
            let _ = mgr.register(shortcut);
        }
    }
}

#[derive(Serialize)]
pub struct SetHotkeyResult {
    pub ok: bool,
    /// Present on failure: "conflict" (ours) or "system" (OS/another app).
    pub reason: Option<String>,
    pub conflicting_action_label: Option<String>,
}

#[tauri::command]
pub fn get_hotkeys<R: Runtime>(app: AppHandle<R>) -> Vec<HotkeyRow> {
    store::rows(&app)
}

#[tauri::command]
pub fn set_hotkey<R: Runtime>(app: AppHandle<R>, action_id: String, combo: String) -> SetHotkeyResult {
    if combo == EMERGENCY_RESTORE {
        return SetHotkeyResult {
            ok: false,
            reason: Some("conflict".into()),
            conflicting_action_label: Some("Emergency restore".into()),
        };
    }
    if let Some(label) = store::conflicting_action(&action_id, &combo) {
        return SetHotkeyResult {
            ok: false,
            reason: Some("conflict".into()),
            conflicting_action_label: Some(label),
        };
    }
    let Ok(shortcut) = Shortcut::from_str(&combo) else {
        return SetHotkeyResult {
            ok: false,
            reason: Some("invalid".into()),
            conflicting_action_label: None,
        };
    };

    // Probe the OS directly: register against a throwaway state, then
    // tear the whole table down and rebuild from the persisted file so
    // we never leave a stray registration behind on failure.
    let mgr = app.global_shortcut();
    let _ = mgr.unregister_all();
    let probe_ok = mgr.register(shortcut).is_ok();
    reapply(&app);

    if !probe_ok {
        return SetHotkeyResult {
            ok: false,
            reason: Some("system".into()),
            conflicting_action_label: None,
        };
    }

    store::set_combo(&action_id, Some(combo));
    reapply(&app);
    let _ = app.emit("hotkeys://state", store::rows(&app));
    SetHotkeyResult {
        ok: true,
        reason: None,
        conflicting_action_label: None,
    }
}

#[tauri::command]
pub fn clear_hotkey<R: Runtime>(app: AppHandle<R>, action_id: String) {
    store::set_combo(&action_id, None);
    reapply(&app);
    let _ = app.emit("hotkeys://state", store::rows(&app));
}

#[tauri::command]
pub fn set_hotkey_enabled<R: Runtime>(app: AppHandle<R>, action_id: String, enabled: bool) {
    store::set_enabled(&action_id, enabled);
    reapply(&app);
    let _ = app.emit("hotkeys://state", store::rows(&app));
}

#[tauri::command]
pub fn reset_hotkey<R: Runtime>(app: AppHandle<R>, action_id: String) {
    store::reset_one(&app, &action_id);
    reapply(&app);
    let _ = app.emit("hotkeys://state", store::rows(&app));
}

#[tauri::command]
pub fn reset_all_hotkeys<R: Runtime>(app: AppHandle<R>) {
    store::reset_all();
    reapply(&app);
    let _ = app.emit("hotkeys://state", store::rows(&app));
}

/// Raw snapshot/restore of the persisted hotkeys file, for settings
/// export/import (PRD F7). Bypasses the per-action validation in
/// `set_hotkey` since import is trusted (it's our own prior export).
pub fn export_file() -> HotkeysFile {
    store::load()
}

pub fn import_file<R: Runtime>(app: &AppHandle<R>, file: HotkeysFile) {
    store::replace_all(file);
    reapply(app);
    let _ = app.emit("hotkeys://state", store::rows(app));
}

fn toggle_display<R: Runtime>(app: &AppHandle<R>, stable_id: &str) {
    let Some(index) = cover::index_for_stable_id(app.clone(), stable_id.to_string()) else {
        return;
    };
    let covered = cover::cover_state(app.clone()).covered.contains(&index);
    if covered {
        cover::uncover_display(app.clone(), index);
    } else {
        cover::cover_display(app.clone(), index);
    }
}

fn nudge_dim<R: Runtime>(app: &AppHandle<R>, delta: i16) {
    let state = cover::dim_state(app.clone());
    let current = state.first().map(|(_, p)| *p as i16).unwrap_or(0);
    let next = (current + delta).clamp(0, 100) as u8;
    cover::set_dim(app.clone(), 0, next);
}

pub fn handle<R: Runtime>(app: &AppHandle<R>, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state != ShortcutState::Pressed {
        return;
    }
    if shortcut == emergency_shortcut() {
        cover::emergency_restore(app);
        return;
    }

    let Some(action_id) = store::action_for_shortcut(shortcut) else {
        return;
    };

    let app = app.clone();
    std::thread::spawn(move || match action_id.as_str() {
        "panic" => cover::cover_all(app.clone()),
        "restore_all" => cover::uncover_all(app.clone()),
        "away_mode" => {
            cover::cover_all(app.clone());
            crate::awake::set_keep_awake_mode(crate::awake::KeepAwakeMode::Always, None);
        }
        "toggle_keep_awake" => {
            let active = crate::awake::is_keep_awake_active();
            crate::awake::set_keep_awake_mode(
                if active {
                    crate::awake::KeepAwakeMode::Off
                } else {
                    crate::awake::KeepAwakeMode::Always
                },
                None,
            );
        }
        "open_popover" => {
            if let Some(win) = app.get_webview_window("popover") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
        "open_app_window" => crate::show_app_window(&app),
        "jump_to_waiting_agent" => {
            crate::show_app_window(&app);
            let _ = app.emit("app://navigate", "agents");
        }
        "dim_up" => nudge_dim(&app, 10),
        "dim_down" => nudge_dim(&app, -10),
        other => {
            if let Some(stable_id) = other.strip_prefix("display:") {
                toggle_display(&app, stable_id);
            }
        }
    });
}
