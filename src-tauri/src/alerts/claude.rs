//! Claude Code hook integration (PRD F5 / §9). Merges our hooks into
//! `~/.claude/settings.json` without clobbering whatever's already
//! there, and backs the file up first.

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

const HOOK_COMMAND_PREFIX: &str = "curtain-notify --agent claude";

fn settings_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("settings.json")
}

fn backup_path() -> PathBuf {
    let mut p = settings_path();
    p.set_extension("json.curtain-backup");
    p
}

fn read_settings() -> Value {
    let path = settings_path();
    fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_settings(value: &Value) -> std::io::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value).unwrap())
}

fn backup_if_needed() {
    let settings = settings_path();
    let backup = backup_path();
    if settings.exists() && !backup.exists() {
        let _ = fs::copy(&settings, &backup);
    }
}

fn hook_block(event_suffix: &str) -> Value {
    json!({
        "matcher": "",
        "hooks": [{
            "type": "command",
            "command": format!("{HOOK_COMMAND_PREFIX} --event {event_suffix}")
        }]
    })
}

fn has_our_hook(entries: &[Value]) -> bool {
    entries.iter().any(|entry| {
        entry["hooks"]
            .as_array()
            .map(|hooks| {
                hooks.iter().any(|h| {
                    h["command"]
                        .as_str()
                        .is_some_and(|c| c.starts_with(HOOK_COMMAND_PREFIX))
                })
            })
            .unwrap_or(false)
    })
}

pub fn connect() -> std::io::Result<()> {
    backup_if_needed();
    let mut settings = read_settings();

    if !settings.is_object() {
        settings = json!({});
    }
    let hooks = settings
        .as_object_mut()
        .unwrap()
        .entry("hooks")
        .or_insert_with(|| json!({}));

    for (event, suffix) in [("Notification", "needs_input"), ("Stop", "done")] {
        let arr = hooks
            .as_object_mut()
            .unwrap()
            .entry(event)
            .or_insert_with(|| json!([]));
        let arr_mut = arr.as_array_mut().unwrap();
        if !has_our_hook(arr_mut) {
            arr_mut.push(hook_block(suffix));
        }
    }

    write_settings(&settings)
}

pub fn disconnect() -> std::io::Result<()> {
    let mut settings = read_settings();
    if let Some(hooks) = settings.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        for event in ["Notification", "Stop"] {
            if let Some(arr) = hooks.get_mut(event).and_then(|a| a.as_array_mut()) {
                arr.retain(|entry| {
                    entry["hooks"]
                        .as_array()
                        .map(|hooks| {
                            !hooks.iter().all(|h| {
                                h["command"]
                                    .as_str()
                                    .is_some_and(|c| c.starts_with(HOOK_COMMAND_PREFIX))
                            })
                        })
                        .unwrap_or(true)
                });
            }
            if hooks.get(event).and_then(|a| a.as_array()).is_some_and(Vec::is_empty) {
                hooks.remove(event);
            }
        }
    }
    if settings.get("hooks").and_then(|h| h.as_object()).is_some_and(serde_json::Map::is_empty) {
        settings.as_object_mut().unwrap().remove("hooks");
    }
    write_settings(&settings)
}

pub fn is_connected() -> bool {
    let settings = read_settings();
    ["Notification", "Stop"].iter().all(|event| {
        settings["hooks"][event]
            .as_array()
            .map(|a| has_our_hook(a))
            .unwrap_or(false)
    })
}
