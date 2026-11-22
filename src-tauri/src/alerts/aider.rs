//! Aider notification integration (PRD F4/F5). Per `docs/agents.md`:
//! not a generic hook system, just a `notifications-command` YAML key
//! in `.aider.conf.yml` (home dir) that fires once the LLM is done and
//! waiting for input — one event covers both "done" and "waiting" for
//! Aider's model, so we report it as `done`. Plain line-based editing
//! rather than a full YAML parser, since it's a single flat key.

use std::fs;
use std::path::PathBuf;

const KEY_PREFIX: &str = "notifications-command:";
const OUR_VALUE: &str = "curtain-notify --agent aider --event done";

fn config_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".aider.conf.yml")
}

fn backup_path() -> PathBuf {
    let mut p = config_path();
    p.set_extension("yml.curtain-backup");
    p
}

fn backup_if_needed() {
    let (p, b) = (config_path(), backup_path());
    if p.exists() && !b.exists() {
        let _ = fs::copy(&p, &b);
    }
}

fn read_lines() -> Vec<String> {
    fs::read_to_string(config_path())
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

fn write_lines(lines: &[String]) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, lines.join("\n") + "\n")
}

pub fn connect() -> std::io::Result<()> {
    backup_if_needed();
    let mut lines = read_lines();
    let our_line = format!("{KEY_PREFIX} {OUR_VALUE}");
    if let Some(existing) = lines.iter_mut().find(|l| l.trim_start().starts_with(KEY_PREFIX)) {
        *existing = our_line;
    } else {
        lines.push(our_line);
    }
    write_lines(&lines)
}

pub fn disconnect() -> std::io::Result<()> {
    let mut lines = read_lines();
    lines.retain(|l| !(l.trim_start().starts_with(KEY_PREFIX) && l.contains("curtain-notify")));
    write_lines(&lines)
}

pub fn is_connected() -> bool {
    read_lines()
        .iter()
        .any(|l| l.trim_start().starts_with(KEY_PREFIX) && l.contains("curtain-notify"))
}
