//! User-extensible agent profiles (PRD F4). Replaces the old hardcoded
//! claude/codex matcher pair with a persisted, editable list. Presets
//! ship enabled (except the generic long-task templates, which are
//! opt-in since matching on a bare process name like `npm` is more
//! prone to false positives than matching a specific agent binary).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchRule {
    pub process_name: Option<String>,
    pub argv_contains: Option<String>,
    pub cwd_contains: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertSource {
    Hooks,
    Wrapper,
    IdleHeuristic,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyConfig {
    pub on_waiting: bool,
    pub on_done: bool,
    pub on_crash: bool,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self {
            on_waiting: true,
            on_done: true,
            on_crash: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub id: String,
    pub display_name: String,
    /// lucide icon name, a single emoji, or a path to a custom image.
    pub icon: String,
    /// Hex color used for this profile's badge/card accent.
    pub color: String,
    pub match_rules: Vec<MatchRule>,
    pub alert_source: AlertSource,
    pub keep_awake: bool,
    pub notify: NotifyConfig,
    pub enabled: bool,
    pub is_preset: bool,
}

impl MatchRule {
    pub fn matches(&self, name_lower: &str, argv_lower: &str, cwd_lower: &str) -> bool {
        let mut any_field = false;
        if let Some(p) = &self.process_name {
            any_field = true;
            if !name_lower.contains(&p.to_lowercase()) {
                return false;
            }
        }
        if let Some(a) = &self.argv_contains {
            any_field = true;
            if !argv_lower.contains(&a.to_lowercase()) {
                return false;
            }
        }
        if let Some(c) = &self.cwd_contains {
            any_field = true;
            if !cwd_lower.contains(&c.to_lowercase()) {
                return false;
            }
        }
        any_field
    }
}

fn rule(process_name: Option<&str>, argv_contains: Option<&str>) -> MatchRule {
    MatchRule {
        process_name: process_name.map(str::to_string),
        argv_contains: argv_contains.map(str::to_string),
        cwd_contains: None,
    }
}

fn preset(
    id: &str,
    display_name: &str,
    icon: &str,
    color: &str,
    rules: Vec<MatchRule>,
    alert_source: AlertSource,
    enabled: bool,
) -> AgentProfile {
    AgentProfile {
        id: id.to_string(),
        display_name: display_name.to_string(),
        icon: icon.to_string(),
        color: color.to_string(),
        match_rules: rules,
        alert_source,
        keep_awake: true,
        notify: NotifyConfig::default(),
        enabled,
        is_preset: true,
    }
}

/// Built-in presets. Hook support per `docs/agents.md` research —
/// Gemini CLI, Cursor CLI, GitHub Copilot CLI and Qwen Code have
/// confirmed official hook/notify mechanisms; Aider has a dedicated
/// notifications-command flag that functions like one; OpenCode, Amp
/// and Goose don't have a confirmed "waiting for input" hook, so they
/// default to the PTY wrapper. The generic long-task templates ship
/// disabled — matching e.g. bare `npm`/`cargo` is broad enough that a
/// user should opt in deliberately.
pub fn default_presets() -> Vec<AgentProfile> {
    vec![
        preset(
            "claude",
            "Claude Code",
            "terminal",
            "#D97757",
            vec![rule(Some("claude"), Some("claude-code")), rule(None, Some("@anthropic-ai/claude-code"))],
            AlertSource::Hooks,
            true,
        ),
        preset(
            "codex",
            "Codex",
            "terminal",
            "#74AA9C",
            vec![rule(Some("codex"), None), rule(None, Some("@openai/codex"))],
            AlertSource::Hooks,
            true,
        ),
        preset(
            "gemini",
            "Gemini CLI",
            "sparkles",
            "#4C8DF6",
            vec![rule(Some("gemini"), None), rule(None, Some("gemini-cli"))],
            AlertSource::Hooks,
            true,
        ),
        preset(
            "cursor",
            "Cursor CLI",
            "mouse-pointer",
            "#6E6ADE",
            vec![rule(Some("cursor-agent"), None)],
            AlertSource::Hooks,
            true,
        ),
        preset(
            "copilot",
            "GitHub Copilot CLI",
            "github",
            "#8957E5",
            vec![rule(Some("copilot"), None)],
            AlertSource::Hooks,
            true,
        ),
        preset(
            "qwen",
            "Qwen Code",
            "bot",
            "#6F42C1",
            vec![rule(Some("qwen"), Some("qwen-code"))],
            AlertSource::Hooks,
            true,
        ),
        preset(
            "aider",
            "Aider",
            "pencil-line",
            "#F2A33C",
            vec![rule(Some("aider"), None)],
            AlertSource::Hooks,
            true,
        ),
        preset(
            "opencode",
            "OpenCode",
            "code",
            "#53B48A",
            vec![rule(Some("opencode"), None)],
            AlertSource::Wrapper,
            true,
        ),
        preset(
            "amp",
            "Amp",
            "zap",
            "#D97757",
            vec![rule(Some("amp"), None)],
            AlertSource::Wrapper,
            true,
        ),
        preset(
            "goose",
            "Goose",
            "feather",
            "#60A5FA",
            vec![rule(Some("goose"), None)],
            AlertSource::Wrapper,
            true,
        ),
        preset(
            "build",
            "Build",
            "hammer",
            "#A1A1AA",
            vec![
                rule(Some("npm"), None),
                rule(Some("cargo"), None),
                rule(Some("xcodebuild"), None),
                rule(Some("gradle"), None),
            ],
            AlertSource::Wrapper,
            false,
        ),
        preset(
            "python-task",
            "Python script / ML training",
            "flask-conical",
            "#A1A1AA",
            vec![rule(Some("python"), None), rule(Some("python3"), None)],
            AlertSource::Wrapper,
            false,
        ),
        preset(
            "render",
            "Render / export",
            "film",
            "#A1A1AA",
            vec![
                rule(Some("ffmpeg"), None),
                rule(Some("blender"), None),
                rule(None, Some("davinci")),
                rule(None, Some("premiere")),
            ],
            AlertSource::Wrapper,
            false,
        ),
    ]
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("dev.naman.curtain")
        .join("agents.json")
}

fn load_from_disk() -> Vec<AgentProfile> {
    fs::read(config_path())
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<AgentProfile>>(&b).ok())
        .unwrap_or_else(default_presets)
}

fn save_to_disk(profiles: &[AgentProfile]) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(profiles) {
        let _ = fs::write(path, bytes);
    }
}

static STATE: OnceLock<Mutex<Vec<AgentProfile>>> = OnceLock::new();

fn cell() -> &'static Mutex<Vec<AgentProfile>> {
    STATE.get_or_init(|| Mutex::new(load_from_disk()))
}

pub fn all() -> Vec<AgentProfile> {
    cell().lock().unwrap().clone()
}

fn mutate(f: impl FnOnce(&mut Vec<AgentProfile>)) {
    let mut guard = cell().lock().unwrap();
    f(&mut guard);
    save_to_disk(&guard);
}

#[tauri::command]
pub fn get_agent_profiles() -> Vec<AgentProfile> {
    all()
}

#[tauri::command]
pub fn upsert_agent_profile(profile: AgentProfile) {
    mutate(|profiles| {
        if let Some(existing) = profiles.iter_mut().find(|p| p.id == profile.id) {
            *existing = profile;
        } else {
            profiles.push(profile);
        }
    });
}

#[tauri::command]
pub fn delete_agent_profile(id: String) -> Result<(), String> {
    let is_preset = all().iter().any(|p| p.id == id && p.is_preset);
    if is_preset {
        return Err("Presets can only be disabled, not deleted.".into());
    }
    mutate(|profiles| profiles.retain(|p| p.id != id));
    Ok(())
}

#[tauri::command]
pub fn set_agent_profile_enabled(id: String, enabled: bool) {
    mutate(|profiles| {
        if let Some(p) = profiles.iter_mut().find(|p| p.id == id) {
            p.enabled = enabled;
        }
    });
}

#[tauri::command]
pub fn duplicate_agent_profile(id: String) {
    mutate(|profiles| {
        if let Some(src) = profiles.iter().find(|p| p.id == id).cloned() {
            let mut copy = src;
            copy.id = format!("{}-copy-{}", copy.id, uuid::Uuid::new_v4().simple());
            copy.display_name = format!("{} (copy)", copy.display_name);
            copy.is_preset = false;
            profiles.push(copy);
        }
    });
}

/// Wholesale replace, used by settings import (PRD F7/export-import).
pub fn replace_all(profiles: Vec<AgentProfile>) {
    mutate(|p| *p = profiles);
}

#[tauri::command]
pub fn restore_preset_agents() {
    mutate(|profiles| {
        let defaults = default_presets();
        for d in defaults {
            if !profiles.iter().any(|p| p.id == d.id) {
                profiles.push(d);
            }
        }
    });
}
