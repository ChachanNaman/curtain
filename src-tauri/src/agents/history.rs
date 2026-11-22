//! Per-agent run history + "time saved" stat (PRD F11). A run is one
//! continuous detection of an agent/task from first-seen to
//! done/crashed, with however much of it was spent in the `Waiting`
//! status tracked separately — that waiting time is framed to the user
//! as time they didn't have to spend babysitting the terminal.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_ENTRIES: usize = 500;
const PRUNE_AFTER_MS: u64 = 30 * 24 * 60 * 60 * 1000; // 30 days
const WEEK_MS: u64 = 7 * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EndStatus {
    Done,
    Crashed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub agent_key: String,
    pub display_name: String,
    pub started_at_ms: u64,
    /// Total time spent in `Waiting` across this run (ms). Live runs
    /// that are *currently* waiting don't include the still-open
    /// window here — see `waiting_started_at_ms`.
    pub waiting_ms: u64,
    /// `Some` while a waiting window is open, so the frontend can keep
    /// a live-ticking "waiting Xm" without polling faster than the 3s
    /// tick.
    pub waiting_started_at_ms: Option<u64>,
    pub ended_at_ms: Option<u64>,
    pub end_status: Option<EndStatus>,
    pub exit_code: Option<i32>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn config_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("dev.naman.curtain").join("history.json")
}

fn load_from_disk() -> Vec<HistoryEntry> {
    fs::read(config_path())
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<HistoryEntry>>(&b).ok())
        .unwrap_or_default()
}

fn save_to_disk(entries: &[HistoryEntry]) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(entries) {
        let _ = fs::write(path, bytes);
    }
}

static STATE: OnceLock<Mutex<Vec<HistoryEntry>>> = OnceLock::new();

fn cell() -> &'static Mutex<Vec<HistoryEntry>> {
    STATE.get_or_init(|| Mutex::new(load_from_disk()))
}

fn mutate(f: impl FnOnce(&mut Vec<HistoryEntry>)) {
    let mut guard = cell().lock().unwrap();
    f(&mut guard);
    let cutoff = now_ms().saturating_sub(PRUNE_AFTER_MS);
    guard.retain(|e| e.started_at_ms >= cutoff);
    if guard.len() > MAX_ENTRIES {
        let drop_n = guard.len() - MAX_ENTRIES;
        guard.drain(0..drop_n);
    }
    save_to_disk(&guard);
}

/// Starts a new run if `agent_key` doesn't already have one open
/// (`ended_at_ms.is_none()`) — idempotent so both the poll loop and
/// `set_status`'s placeholder path can call it freely.
pub fn start_run(agent_key: &str, display_name: &str) {
    mutate(|entries| {
        if entries.iter().any(|e| e.agent_key == agent_key && e.ended_at_ms.is_none()) {
            return;
        }
        entries.push(HistoryEntry {
            id: uuid::Uuid::new_v4().simple().to_string(),
            agent_key: agent_key.to_string(),
            display_name: display_name.to_string(),
            started_at_ms: now_ms(),
            waiting_ms: 0,
            waiting_started_at_ms: None,
            ended_at_ms: None,
            end_status: None,
            exit_code: None,
        });
    });
}

fn open_entry_mut<'a>(entries: &'a mut [HistoryEntry], agent_key: &str) -> Option<&'a mut HistoryEntry> {
    entries.iter_mut().filter(|e| e.agent_key == agent_key && e.ended_at_ms.is_none()).last()
}

pub fn mark_waiting_start(agent_key: &str) {
    mutate(|entries| {
        if let Some(e) = open_entry_mut(entries, agent_key)
            && e.waiting_started_at_ms.is_none()
        {
            e.waiting_started_at_ms = Some(now_ms());
        }
    });
}

pub fn mark_waiting_end(agent_key: &str) {
    mutate(|entries| {
        if let Some(e) = open_entry_mut(entries, agent_key)
            && let Some(started) = e.waiting_started_at_ms.take()
        {
            e.waiting_ms += now_ms().saturating_sub(started);
        }
    });
}

/// Closes the open run for `agent_key`, if any. No-op if it's already
/// closed — both the alert-driven status change and the poll loop's
/// "process vanished" fallback may call this for the same run.
pub fn finish(agent_key: &str, status: EndStatus, exit_code: Option<i32>) {
    mutate(|entries| {
        let Some(e) = open_entry_mut(entries, agent_key) else { return };
        if let Some(started) = e.waiting_started_at_ms.take() {
            e.waiting_ms += now_ms().saturating_sub(started);
        }
        e.ended_at_ms = Some(now_ms());
        e.end_status = Some(status);
        e.exit_code = exit_code;
    });
}

/// Newest-first history, optionally filtered to one agent.
pub fn list(agent_key: Option<&str>) -> Vec<HistoryEntry> {
    let mut entries = cell().lock().unwrap().clone();
    if let Some(key) = agent_key {
        entries.retain(|e| e.agent_key == key);
    }
    entries.sort_by_key(|e| std::cmp::Reverse(e.started_at_ms));
    entries
}

/// Total waiting time this week (ms), including the still-open portion
/// of any run that's waiting right now — what the popover/Agents tab
/// frames as "You saved Xh Ym this week."
pub fn time_saved_ms_this_week() -> u64 {
    let now = now_ms();
    let cutoff = now.saturating_sub(WEEK_MS);
    cell()
        .lock()
        .unwrap()
        .iter()
        .filter(|e| e.started_at_ms >= cutoff)
        .map(|e| {
            let live = e.waiting_started_at_ms.map(|s| now.saturating_sub(s)).unwrap_or(0);
            e.waiting_ms + live
        })
        .sum()
}

#[tauri::command]
pub fn get_agent_history(agent_key: Option<String>) -> Vec<HistoryEntry> {
    list(agent_key.as_deref())
}

#[tauri::command]
pub fn get_time_saved_this_week_ms() -> u64 {
    time_saved_ms_this_week()
}
