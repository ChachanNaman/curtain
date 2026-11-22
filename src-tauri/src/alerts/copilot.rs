//! GitHub Copilot CLI hook integration (PRD F4/F5). Per `docs/agents.md`:
//! `~/.copilot/hooks/<name>.json`, one file per hook, schema
//! `{ "version": 1, "hooks": { "<event>": [{ "type": "command", "bash":
//! "..." }] } }`. `agentStop` is the "turn complete" signal.

use serde_json::json;
use std::fs;
use std::path::PathBuf;

const FILE_NAME: &str = "curtain.json";

fn hooks_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".copilot").join("hooks")
}

fn our_hook_path() -> PathBuf {
    hooks_dir().join(FILE_NAME)
}

pub fn connect() -> std::io::Result<()> {
    let dir = hooks_dir();
    fs::create_dir_all(&dir)?;
    let doc = json!({
        "version": 1,
        "hooks": {
            "agentStop": [{
                "type": "command",
                "bash": "curtain-notify --agent copilot --event done",
            }]
        }
    });
    fs::write(our_hook_path(), serde_json::to_vec_pretty(&doc).unwrap())
}

pub fn disconnect() -> std::io::Result<()> {
    let path = our_hook_path();
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn is_connected() -> bool {
    our_hook_path().exists()
}
