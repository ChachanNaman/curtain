//! Codex `notify` integration (PRD F5 / §9). Sets `notify = [...]` in
//! `~/.codex/config.toml` via `toml_edit` so existing formatting and
//! comments survive, and backs the file up first.

use std::fs;
use std::path::PathBuf;
use toml_edit::{Array, DocumentMut, Item, Value};

const NOTIFY_COMMAND: [&str; 4] = ["curtain-notify", "--agent", "codex", "--from-codex-json"];

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".codex")
        .join("config.toml")
}

fn backup_path() -> PathBuf {
    let mut p = config_path();
    p.set_extension("toml.curtain-backup");
    p
}

fn backup_if_needed() {
    let path = config_path();
    let backup = backup_path();
    if path.exists() && !backup.exists() {
        let _ = fs::copy(&path, &backup);
    }
}

fn read_doc() -> DocumentMut {
    let path = config_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.parse::<DocumentMut>().ok())
        .unwrap_or_default()
}

fn write_doc(doc: &DocumentMut) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, doc.to_string())
}

fn is_our_notify(item: &Item) -> bool {
    item.as_array()
        .map(|arr| {
            arr.iter()
                .zip(NOTIFY_COMMAND.iter())
                .all(|(v, expected)| v.as_str() == Some(*expected))
                && arr.len() == NOTIFY_COMMAND.len()
        })
        .unwrap_or(false)
}

pub fn connect() -> std::io::Result<()> {
    backup_if_needed();
    let mut doc = read_doc();

    let mut arr = Array::new();
    for part in NOTIFY_COMMAND {
        arr.push(Value::from(part));
    }
    doc["notify"] = Item::Value(Value::Array(arr));

    write_doc(&doc)
}

pub fn disconnect() -> std::io::Result<()> {
    let mut doc = read_doc();
    if doc
        .get("notify")
        .map(is_our_notify)
        .unwrap_or(false)
    {
        doc.remove("notify");
    }
    write_doc(&doc)
}

pub fn is_connected() -> bool {
    read_doc().get("notify").map(is_our_notify).unwrap_or(false)
}
