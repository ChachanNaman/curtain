//! Settings export/import (PRD F7/F4 acceptance check): bundles
//! hotkeys + agent profiles into one JSON file so a fresh install can
//! be restored from a single document.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_dialog::DialogExt;

use crate::agents::profiles::{self, AgentProfile};
use crate::hotkeys::{self, HotkeysFile};

const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct SettingsBundle {
    schema_version: u32,
    hotkeys: HotkeysFile,
    agent_profiles: Vec<AgentProfile>,
}

fn bundle() -> SettingsBundle {
    SettingsBundle {
        schema_version: SCHEMA_VERSION,
        hotkeys: hotkeys::export_file(),
        agent_profiles: profiles::all(),
    }
}

#[tauri::command]
pub fn export_settings<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let json = serde_json::to_vec_pretty(&bundle()).map_err(|e| e.to_string())?;
    let path = app
        .dialog()
        .file()
        .set_file_name("curtain-settings.json")
        .add_filter("Curtain settings", &["json"])
        .blocking_save_file();
    let Some(path) = path else { return Ok(false) };
    let path = path.into_path().map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn import_settings<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let path = app
        .dialog()
        .file()
        .add_filter("Curtain settings", &["json"])
        .blocking_pick_file();
    let Some(path) = path else { return Ok(false) };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let parsed: SettingsBundle = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;

    hotkeys::import_file(&app, parsed.hotkeys);
    profiles::replace_all(parsed.agent_profiles);
    let _ = app.emit("agents://profiles-changed", ());
    Ok(true)
}
