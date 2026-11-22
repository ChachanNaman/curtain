//! Persisted hotkey bindings. Plain JSON file (same pattern as
//! `alerts::config`) rather than `tauri-plugin-store`, so it's trivial
//! to include in the settings export/import JSON blob unchanged.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::Shortcut;

use crate::cover;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub combo: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HotkeysFile {
    pub actions: HashMap<String, Binding>,
}

#[derive(Serialize, Clone)]
pub struct HotkeyRow {
    pub id: String,
    pub label: String,
    pub combo: Option<String>,
    pub default_combo: Option<String>,
    pub enabled: bool,
    pub is_display: bool,
    pub display_name: Option<String>,
    /// `None` for non-display rows; `Some(bool)` for display rows.
    pub display_connected: Option<bool>,
}

struct StaticAction {
    id: &'static str,
    label: &'static str,
    #[cfg(target_os = "macos")]
    default: &'static str,
    #[cfg(not(target_os = "macos"))]
    default: &'static str,
}

const STATIC_ACTIONS: &[StaticAction] = &[
    StaticAction { id: "panic", label: "Panic: cover all", default: cfg_default("ctrl+alt+h", "ctrl+alt+h") },
    StaticAction { id: "restore_all", label: "Restore all", default: cfg_default("ctrl+alt+r", "ctrl+alt+r") },
    StaticAction { id: "away_mode", label: "Away Mode", default: cfg_default("ctrl+alt+a", "ctrl+alt+a") },
    StaticAction { id: "toggle_keep_awake", label: "Toggle keep-awake", default: cfg_default("ctrl+alt+k", "ctrl+alt+k") },
    StaticAction { id: "open_popover", label: "Open popover", default: cfg_default("ctrl+alt+c", "ctrl+alt+c") },
    StaticAction { id: "open_app_window", label: "Open app window", default: cfg_default("ctrl+alt+shift+c", "ctrl+alt+shift+c") },
    StaticAction { id: "jump_to_waiting_agent", label: "Jump to waiting agent", default: cfg_default("ctrl+alt+j", "ctrl+alt+j") },
    StaticAction { id: "dim_up", label: "Dim up (focused display)", default: cfg_default("ctrl+alt+equal", "ctrl+alt+equal") },
    StaticAction { id: "dim_down", label: "Dim down (focused display)", default: cfg_default("ctrl+alt+minus", "ctrl+alt+minus") },
];

const fn cfg_default(mac: &'static str, _other: &'static str) -> &'static str {
    // Mac and Windows defaults happen to share the same Ctrl+Alt+<key>
    // shape for every non-display action, per the PRD table.
    mac
}

fn display_toggle_default(ordinal: usize) -> Option<&'static str> {
    match ordinal {
        0 => Some("ctrl+alt+1"),
        1 => Some("ctrl+alt+2"),
        2 => Some("ctrl+alt+3"),
        _ => None,
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("dev.naman.curtain")
        .join("hotkeys.json")
}

fn load_from_disk() -> HotkeysFile {
    fs::read(config_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_to_disk(file: &HotkeysFile) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(file) {
        let _ = fs::write(path, bytes);
    }
}

static STATE: OnceLock<Mutex<HotkeysFile>> = OnceLock::new();

fn cell() -> &'static Mutex<HotkeysFile> {
    STATE.get_or_init(|| Mutex::new(load_from_disk()))
}

pub fn load() -> HotkeysFile {
    cell().lock().unwrap().clone()
}

fn mutate(f: impl FnOnce(&mut HotkeysFile)) {
    let mut guard = cell().lock().unwrap();
    f(&mut guard);
    save_to_disk(&guard);
}

pub fn set_combo(action_id: &str, combo: Option<String>) {
    mutate(|file| {
        let entry = file.actions.entry(action_id.to_string()).or_insert(Binding {
            combo: None,
            enabled: true,
        });
        entry.combo = combo;
    });
}

pub fn set_enabled(action_id: &str, enabled: bool) {
    mutate(|file| {
        let entry = file.actions.entry(action_id.to_string()).or_insert(Binding {
            combo: None,
            enabled: true,
        });
        entry.enabled = enabled;
    });
}

fn default_combo_for(action_id: &str, display_ordinals: &HashMap<String, usize>) -> Option<String> {
    if let Some(id) = action_id.strip_prefix("display:") {
        let ordinal = *display_ordinals.get(id)?;
        return display_toggle_default(ordinal).map(str::to_string);
    }
    STATIC_ACTIONS
        .iter()
        .find(|a| a.id == action_id)
        .map(|a| a.default.to_string())
}

pub fn reset_one<R: Runtime>(app: &AppHandle<R>, action_id: &str) {
    // Needs the current display ordering to know the right default for
    // display-toggle actions; rebuild it the same way `rows` does.
    let ordinals = display_ordinals(app);
    let default = default_combo_for(action_id, &ordinals);
    mutate(|file| {
        file.actions.insert(
            action_id.to_string(),
            Binding {
                combo: default,
                enabled: true,
            },
        );
    });
}

pub fn reset_all() {
    mutate(|file| file.actions.clear());
}

/// Wholesale replace, used by settings import (PRD F7/export-import).
pub fn replace_all(file: HotkeysFile) {
    mutate(|f| *f = file);
}

/// Returns the label of whichever *other*, enabled action already owns
/// `combo`, if any.
pub fn conflicting_action(action_id: &str, combo: &str) -> Option<String> {
    let file = load();
    for (id, binding) in file.actions.iter() {
        if id == action_id || !binding.enabled {
            continue;
        }
        if binding.combo.as_deref() == Some(combo) {
            return Some(label_for(id));
        }
    }
    None
}

fn label_for(action_id: &str) -> String {
    if let Some(id) = action_id.strip_prefix("display:") {
        return format!("Toggle display ({id})");
    }
    STATIC_ACTIONS
        .iter()
        .find(|a| a.id == action_id)
        .map(|a| a.label.to_string())
        .unwrap_or_else(|| action_id.to_string())
}

pub fn action_for_shortcut(shortcut: &Shortcut) -> Option<String> {
    let file = load();
    let combo = shortcut.to_string();
    file.actions
        .iter()
        .find(|(_, b)| b.enabled && b.combo.as_deref() == Some(combo.as_str()))
        .map(|(id, _)| id.clone())
}

/// Ordinal position of each currently-connected display's stable ID,
/// used only to pick sensible defaults (1/2/3) for newly-seen displays.
fn display_ordinals<R: Runtime>(app: &AppHandle<R>) -> HashMap<String, usize> {
    cover::get_displays(app.clone())
        .into_iter()
        .enumerate()
        .filter_map(|(i, d)| d.stable_id.map(|sid| (sid, i)))
        .collect()
}

/// Builds the full row list: static actions in a fixed order, then one
/// row per display the app has ever seen a binding for, plus any
/// currently-connected display that doesn't have one yet.
pub fn rows<R: Runtime>(app: &AppHandle<R>) -> Vec<HotkeyRow> {
    let file = load();
    let connected = cover::get_displays(app.clone());
    let connected_ids: HashMap<String, String> = connected
        .iter()
        .filter_map(|d| d.stable_id.clone().map(|sid| (sid, d.name.clone())))
        .collect();
    let ordinals = display_ordinals(app);

    let mut rows: Vec<HotkeyRow> = STATIC_ACTIONS
        .iter()
        .map(|a| {
            let binding = file.actions.get(a.id);
            HotkeyRow {
                id: a.id.to_string(),
                label: a.label.to_string(),
                combo: binding.and_then(|b| b.combo.clone()).or(Some(a.default.to_string())),
                default_combo: Some(a.default.to_string()),
                enabled: binding.map(|b| b.enabled).unwrap_or(true),
                is_display: false,
                display_name: None,
                display_connected: None,
            }
        })
        .collect();

    let mut seen_display_ids: Vec<String> = Vec::new();
    for (action_id, binding) in file.actions.iter() {
        let Some(sid) = action_id.strip_prefix("display:") else { continue };
        seen_display_ids.push(sid.to_string());
        let connected_name = connected_ids.get(sid).cloned();
        rows.push(HotkeyRow {
            id: action_id.clone(),
            label: connected_name.clone().unwrap_or_else(|| "Display".to_string()),
            combo: binding.combo.clone(),
            default_combo: default_combo_for(action_id, &ordinals),
            enabled: binding.enabled,
            is_display: true,
            display_name: Some(connected_name.unwrap_or_else(|| "Not connected".to_string())),
            display_connected: Some(connected_ids.contains_key(sid)),
        });
    }
    for (sid, name) in connected_ids.iter() {
        if seen_display_ids.iter().any(|s| s == sid) {
            continue;
        }
        let action_id = format!("display:{sid}");
        rows.push(HotkeyRow {
            id: action_id.clone(),
            label: name.clone(),
            combo: default_combo_for(&action_id, &ordinals),
            default_combo: default_combo_for(&action_id, &ordinals),
            enabled: true,
            is_display: true,
            display_name: Some(name.clone()),
            display_connected: Some(true),
        });
    }

    rows
}
