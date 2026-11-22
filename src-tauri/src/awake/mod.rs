//! Smart Keep Awake (PRD F3). M1 ships the manual modes (`Off`, `Always`,
//! `Timer`); the agent-aware `While agents run` mode is wired once F4
//! (agent detection) lands in M2.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
use windows as platform;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KeepAwakeMode {
    Off,
    WhileAgentsRun,
    Always,
    Timer,
}

struct AwakeState {
    mode: KeepAwakeMode,
    timer_until: Option<Instant>,
}

static STATE: Mutex<AwakeState> = Mutex::new(AwakeState {
    mode: KeepAwakeMode::Off,
    timer_until: None,
});

fn apply(active: bool) {
    if active {
        platform::enable("Curtain: keeping your Mac awake");
    } else {
        platform::disable();
    }
}

#[tauri::command]
pub fn set_keep_awake_mode(mode: KeepAwakeMode, timer_minutes: Option<u64>) {
    let mut state = STATE.lock().unwrap();
    state.mode = mode;
    state.timer_until = match mode {
        KeepAwakeMode::Timer => {
            Some(Instant::now() + Duration::from_secs(timer_minutes.unwrap_or(30) * 60))
        }
        _ => None,
    };
    let active = matches!(mode, KeepAwakeMode::Always | KeepAwakeMode::Timer);
    drop(state);
    apply(active);
}

#[tauri::command]
pub fn get_keep_awake_mode() -> KeepAwakeMode {
    STATE.lock().unwrap().mode
}

#[tauri::command]
pub fn is_keep_awake_active() -> bool {
    platform::is_enabled()
}

/// Called by the agent-count watcher (M2) and by a background ticker that
/// checks timer expiry. Keeping this separate from `set_keep_awake_mode`
/// means the timer can switch itself off without the UI re-asserting mode.
pub fn tick(agents_running: bool) {
    let mut state = STATE.lock().unwrap();
    let active = match state.mode {
        KeepAwakeMode::Off => false,
        KeepAwakeMode::Always => true,
        KeepAwakeMode::WhileAgentsRun => agents_running,
        KeepAwakeMode::Timer => match state.timer_until {
            Some(until) if Instant::now() < until => true,
            _ => {
                state.mode = KeepAwakeMode::Off;
                state.timer_until = None;
                false
            }
        },
    };
    drop(state);
    apply(active);
}
