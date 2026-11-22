//! Needs-you alerts (PRD F5) — the hero feature. A local listener
//! receives events from the `curtain-notify` sidecar (invoked by Claude
//! Code / Codex hooks) and fans them out to native notifications and an
//! ntfy.sh push, plus updates the agent's status for the popover.

pub mod aider;
pub mod claude;
pub mod codex;
pub mod config;
pub mod copilot;
pub mod cursor;
pub mod gemini;
pub mod qwen;
pub mod server;

use crate::agents::{self, AgentStatus};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_notification::NotificationExt;

pub fn start<R: Runtime + 'static>(app: AppHandle<R>) {
    server::spawn(app);
}

#[derive(Serialize, Clone)]
struct AlertPayload {
    agent_key: String,
    agent_name: String,
    message: String,
}

pub fn fire_alert<R: Runtime>(app: &AppHandle<R>, agent_key: &str, status: AgentStatus, exit_code: Option<i32>) {
    let name = agents::display_name(agent_key);
    let profile = agents::profiles::all().into_iter().find(|p| p.id == agent_key);

    let (message, should_notify) = match status {
        AgentStatus::Waiting => (
            format!("{name} is waiting for you"),
            profile.as_ref().map(|p| p.notify.on_waiting).unwrap_or(true),
        ),
        AgentStatus::Done => (
            format!("{name} finished"),
            profile.as_ref().map(|p| p.notify.on_done).unwrap_or(true),
        ),
        AgentStatus::Crashed => (
            match exit_code {
                Some(code) => format!("{name} crashed (exit {code})"),
                None => format!("{name} crashed"),
            },
            profile.as_ref().map(|p| p.notify.on_crash).unwrap_or(true),
        ),
        // Idle-maybe is a soft guess, never a push — it would be noisy
        // and wrong too often to earn a notification.
        AgentStatus::Running | AgentStatus::IdleMaybe => return,
    };
    if !should_notify {
        return;
    }

    let cfg = config::get();

    if status == AgentStatus::Waiting
        && cfg.uncover_on_alert_enabled
        && let Some(pid) = agents::pid_for(agent_key)
    {
        crate::cover::uncover_on_alert(app, pid);
    }

    if cfg.notification_enabled
        && let Err(e) = app.notification().builder().title("Curtain").body(&message).show()
    {
        log::error!("notification failed: {e}");
    }

    if cfg.ntfy_enabled {
        let topic = cfg.ntfy_topic.clone();
        let msg = message.clone();
        std::thread::spawn(move || {
            let _ = ureq::post(&format!("https://ntfy.sh/{topic}"))
                .timeout(std::time::Duration::from_secs(5))
                .send_string(&msg);
        });
    }

    let _ = app.emit(
        "alerts://notify",
        AlertPayload {
            agent_key: agent_key.to_string(),
            agent_name: name,
            message,
        },
    );
}

/// Fires an alert with a caller-supplied message, bypassing the
/// waiting/done/crashed templates — backs `curtain-notify --event
/// custom --message "..."` for scripts/git-hooks that just want to ping
/// the user with their own text.
pub fn fire_custom_alert<R: Runtime>(app: &AppHandle<R>, agent_key: &str, message: &str) {
    let cfg = config::get();
    if cfg.notification_enabled
        && let Err(e) = app.notification().builder().title("Curtain").body(message).show()
    {
        log::error!("notification failed: {e}");
    }
    if cfg.ntfy_enabled {
        let topic = cfg.ntfy_topic.clone();
        let msg = message.to_string();
        std::thread::spawn(move || {
            let _ = ureq::post(&format!("https://ntfy.sh/{topic}"))
                .timeout(std::time::Duration::from_secs(5))
                .send_string(&msg);
        });
    }
    let _ = app.emit(
        "alerts://notify",
        AlertPayload {
            agent_key: agent_key.to_string(),
            agent_name: agents::display_name(agent_key),
            message: message.to_string(),
        },
    );
}

#[tauri::command]
pub fn get_alerts_config() -> config::AlertsConfig {
    config::get()
}

#[tauri::command]
pub fn set_alert_channel(channel: String, enabled: bool) -> config::AlertsConfig {
    config::update(|cfg| match channel.as_str() {
        "notification" => cfg.notification_enabled = enabled,
        "sound" => cfg.sound_enabled = enabled,
        "ntfy" => cfg.ntfy_enabled = enabled,
        "uncover_on_alert" => cfg.uncover_on_alert_enabled = enabled,
        _ => {}
    })
}

#[tauri::command]
pub fn send_test_alert<R: Runtime>(app: AppHandle<R>) {
    fire_alert(&app, "claude", AgentStatus::Waiting, None);
}

/// Symlinks the `curtain-notify` sidecar *and* the main `curtain`
/// binary (which doubles as the `curtain run` wrapper CLI — see
/// `main.rs`) into `~/.local/bin`, so hook commands and the shell
/// aliases shown in the Agents tab find them on the user's normal PATH,
/// not ours. Safe to call repeatedly: it just re-points the symlinks.
fn ensure_notify_symlink() -> std::io::Result<()> {
    let current_exe = std::env::current_exe()?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| std::io::Error::other("no parent dir for current_exe"))?;

    let target_dir = dirs::home_dir()
        .ok_or_else(|| std::io::Error::other("no home dir"))?
        .join(".local")
        .join("bin");
    std::fs::create_dir_all(&target_dir)?;

    for name in ["curtain-notify", "curtain"] {
        let source = exe_dir.join(name);
        let link = target_dir.join(name);
        if link.symlink_metadata().is_ok() {
            std::fs::remove_file(&link)?;
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&source, &link)?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&source, &link)?;
    }
    Ok(())
}

#[tauri::command]
pub fn connect_claude_code() -> Result<(), String> {
    ensure_notify_symlink().map_err(|e| e.to_string())?;
    claude::connect().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn disconnect_claude_code() -> Result<(), String> {
    claude::disconnect().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn claude_connected() -> bool {
    claude::is_connected()
}

#[tauri::command]
pub fn connect_codex() -> Result<(), String> {
    ensure_notify_symlink().map_err(|e| e.to_string())?;
    codex::connect().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn disconnect_codex() -> Result<(), String> {
    codex::disconnect().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn codex_connected() -> bool {
    codex::is_connected()
}

/// Generates `connect_<tool>` / `disconnect_<tool>` / `<tool>_connected`
/// Tauri commands for a hook module exposing `connect()`/`disconnect()`/
/// `is_connected()`, so the 5 extra integrations beyond Claude/Codex
/// don't each need hand-written boilerplate.
macro_rules! hook_commands {
    ($module:ident, $connect_fn:ident, $disconnect_fn:ident, $connected_fn:ident) => {
        #[tauri::command]
        pub fn $connect_fn() -> Result<(), String> {
            ensure_notify_symlink().map_err(|e| e.to_string())?;
            $module::connect().map_err(|e| e.to_string())
        }

        #[tauri::command]
        pub fn $disconnect_fn() -> Result<(), String> {
            $module::disconnect().map_err(|e| e.to_string())
        }

        #[tauri::command]
        pub fn $connected_fn() -> bool {
            $module::is_connected()
        }
    };
}

hook_commands!(gemini, connect_gemini, disconnect_gemini, gemini_connected);
hook_commands!(cursor, connect_cursor, disconnect_cursor, cursor_connected);
hook_commands!(copilot, connect_copilot, disconnect_copilot, copilot_connected);
hook_commands!(qwen, connect_qwen, disconnect_qwen, qwen_connected);
hook_commands!(aider, connect_aider, disconnect_aider, aider_connected);
