//! Cursor CLI hook integration (PRD F4/F5). Per `docs/agents.md`:
//! `~/.cursor/hooks.json`, schema `{ "version": 1, "hooks": { "stop":
//! [...] } }`. The `stop` event is the only lifecycle signal
//! (status: completed/aborted/error) — there's no separate "waiting for
//! input" event, so we only wire `done`.

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

const OUR_COMMAND: &str = "curtain-notify --agent cursor --event done";

fn hooks_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".cursor").join("hooks.json")
}

fn backup_path() -> PathBuf {
    let mut p = hooks_path();
    p.set_extension("json.curtain-backup");
    p
}

fn read() -> Value {
    fs::read(hooks_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| json!({ "version": 1, "hooks": {} }))
}

fn write(value: &Value) -> std::io::Result<()> {
    let path = hooks_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value).unwrap())
}

fn backup_if_needed() {
    let (p, b) = (hooks_path(), backup_path());
    if p.exists() && !b.exists() {
        let _ = fs::copy(&p, &b);
    }
}

fn has_our_hook(entries: &[Value]) -> bool {
    entries.iter().any(|e| e["command"].as_str() == Some(OUR_COMMAND))
}

pub fn connect() -> std::io::Result<()> {
    backup_if_needed();
    let mut doc = read();
    if !doc.is_object() {
        doc = json!({ "version": 1, "hooks": {} });
    }
    let obj = doc.as_object_mut().unwrap();
    obj.entry("version").or_insert_with(|| json!(1));
    let hooks = obj.entry("hooks").or_insert_with(|| json!({}));
    let stop = hooks.as_object_mut().unwrap().entry("stop").or_insert_with(|| json!([]));
    let arr = stop.as_array_mut().unwrap();
    if !has_our_hook(arr) {
        arr.push(json!({ "command": OUR_COMMAND }));
    }
    write(&doc)
}

pub fn disconnect() -> std::io::Result<()> {
    let mut doc = read();
    if let Some(arr) = doc.get_mut("hooks").and_then(|h| h.get_mut("stop")).and_then(|s| s.as_array_mut()) {
        arr.retain(|e| e["command"].as_str() != Some(OUR_COMMAND));
    }
    if let Some(hooks) = doc.get_mut("hooks").and_then(|h| h.as_object_mut())
        && hooks.get("stop").and_then(|s| s.as_array()).is_some_and(Vec::is_empty)
    {
        hooks.remove("stop");
    }
    write(&doc)
}

pub fn is_connected() -> bool {
    read()
        .get("hooks")
        .and_then(|h| h.get("stop"))
        .and_then(|s| s.as_array())
        .map(|a| has_our_hook(a))
        .unwrap_or(false)
}
