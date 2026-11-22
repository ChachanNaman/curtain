//! Generic agent/long-task detection (PRD F4). Polled every 3s from a
//! background thread (see `lib.rs`'s ticker) against the user's
//! `AgentProfile` list instead of a hardcoded matcher pair. Hook- or
//! wrapper-driven status changes (waiting/done/crashed) come from the
//! local alert listener (F5) and are merged in here so a single source
//! of truth backs both the popover and the keep-awake "while agents
//! run" mode.

pub mod history;
pub mod profiles;
pub mod wrapper;

use crate::agents::profiles::AlertSource;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tauri::{AppHandle, Emitter, Runtime};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Running,
    Waiting,
    IdleMaybe,
    Done,
    Crashed,
}

#[derive(Debug, Clone, Serialize)]
pub struct Agent {
    pub key: String,
    pub display_name: String,
    pub icon: String,
    pub color: String,
    pub pid: u32,
    pub started_at_ms: u64,
    pub status: AgentStatus,
    pub exit_code: Option<i32>,
}

struct Tracked {
    pid: u32,
    started_at_ms: u64,
    status: AgentStatus,
    exit_code: Option<i32>,
    /// Idle-heuristic bookkeeping: was this process measurably busy at
    /// some point, and since when has it been ~idle?
    was_busy: bool,
    idle_since: Option<Instant>,
}

static AGENTS: std::sync::LazyLock<Mutex<HashMap<String, Tracked>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));
const IDLE_CPU_THRESHOLD: f32 = 1.0;
const BUSY_CPU_THRESHOLD: f32 = 5.0;
const IDLE_SECONDS_BEFORE_MAYBE: u64 = 15;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn snapshot<R: Runtime>(app: &AppHandle<R>) -> Vec<Agent> {
    let profiles = profiles::all();
    let agents = AGENTS.lock().unwrap();
    let list: Vec<Agent> = agents
        .iter()
        .filter_map(|(key, t)| {
            let profile = profiles.iter().find(|p| &p.id == key)?;
            Some(Agent {
                key: key.clone(),
                display_name: profile.display_name.clone(),
                icon: profile.icon.clone(),
                color: profile.color.clone(),
                pid: t.pid,
                started_at_ms: t.started_at_ms,
                status: t.status,
                exit_code: t.exit_code,
            })
        })
        .collect();
    let _ = app.emit("agents://state", &list);
    list
}

/// Refresh the process list, update agent state, and emit it to the
/// frontend. Returns whether any *enabled, keep-awake* profile has a
/// running match (drives the "while agents run" keep-awake mode).
pub fn poll_once<R: Runtime>(app: &AppHandle<R>) -> bool {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_cpu()
            .with_cwd(UpdateKind::Always)
            .with_cmd(UpdateKind::Always),
    );

    let profiles = profiles::all();
    let enabled_profiles: Vec<_> = profiles.iter().filter(|p| p.enabled).collect();

    // Lowest matching PID per profile, mirroring the old
    // "shell wrapper + child" dedup behavior.
    let mut by_profile: HashMap<String, u32> = HashMap::new();
    let mut cpu_by_profile: HashMap<String, f32> = HashMap::new();

    for (pid, process) in sys.processes() {
        let name_lower = process.name().to_string_lossy().to_lowercase();
        let argv_lower = process
            .cmd()
            .iter()
            .map(|a| a.to_string_lossy().to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        let cwd_lower = process
            .cwd()
            .map(|p| p.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        for profile in &enabled_profiles {
            if profile.match_rules.iter().any(|r| r.matches(&name_lower, &argv_lower, &cwd_lower)) {
                let pid_u32 = pid.as_u32();
                by_profile
                    .entry(profile.id.clone())
                    .and_modify(|p| *p = (*p).min(pid_u32))
                    .or_insert(pid_u32);
                cpu_by_profile
                    .entry(profile.id.clone())
                    .and_modify(|c| *c += process.cpu_usage())
                    .or_insert(process.cpu_usage());
            }
        }
    }

    let mut agents = AGENTS.lock().unwrap();

    // A process that vanished without ever reporting done/crashed
    // (no hook, or the hook fired after the process was already gone)
    // still needs its history entry closed — `finish` is a no-op if an
    // explicit status change already closed it.
    for (key, tracked) in agents.iter() {
        if !by_profile.contains_key(key) {
            let end_status =
                if tracked.status == AgentStatus::Crashed { history::EndStatus::Crashed } else { history::EndStatus::Done };
            history::finish(key, end_status, tracked.exit_code);
        }
    }
    agents.retain(|key, _| by_profile.contains_key(key));

    for (profile_id, pid) in &by_profile {
        let profile = profiles.iter().find(|p| &p.id == profile_id);
        let cpu = cpu_by_profile.get(profile_id).copied().unwrap_or(0.0);

        let is_new = !agents.contains_key(profile_id);
        let entry = agents.entry(profile_id.clone()).or_insert_with(|| Tracked {
            pid: *pid,
            started_at_ms: now_ms(),
            status: AgentStatus::Running,
            exit_code: None,
            was_busy: false,
            idle_since: None,
        });
        entry.pid = *pid;
        if is_new {
            let name = profile.map(|p| p.display_name.clone()).unwrap_or_else(|| profile_id.clone());
            history::start_run(profile_id, &name);
        }

        if profile.map(|p| p.alert_source) == Some(AlertSource::IdleHeuristic) {
            if cpu > BUSY_CPU_THRESHOLD {
                entry.was_busy = true;
                entry.idle_since = None;
                if entry.status == AgentStatus::IdleMaybe {
                    entry.status = AgentStatus::Running;
                }
            } else if cpu < IDLE_CPU_THRESHOLD && entry.was_busy {
                let since = entry.idle_since.get_or_insert_with(Instant::now);
                if since.elapsed().as_secs() >= IDLE_SECONDS_BEFORE_MAYBE {
                    entry.status = AgentStatus::IdleMaybe;
                }
            }
        }
    }

    let any_keep_awake = by_profile.keys().any(|id| {
        profiles
            .iter()
            .find(|p| &p.id == id)
            .map(|p| p.keep_awake)
            .unwrap_or(false)
    });
    drop(agents);

    snapshot(app);
    any_keep_awake
}

/// Called by the local alert listener (F5) / wrapper CLI when a hook
/// or PTY-watched process reports a status change.
pub fn set_status(key: &str, status: AgentStatus, exit_code: Option<i32>) -> bool {
    let mut agents = AGENTS.lock().unwrap();
    let prev_status = agents.get(key).map(|a| a.status);
    if let Some(a) = agents.get_mut(key) {
        a.status = status;
        a.exit_code = exit_code;
    } else {
        // The wrapper/hook may fire before the next poll tick notices
        // the process — register a placeholder entry so the alert
        // still has somewhere to land.
        agents.insert(
            key.to_string(),
            Tracked {
                pid: 0,
                started_at_ms: now_ms(),
                status,
                exit_code,
                was_busy: false,
                idle_since: None,
            },
        );
        history::start_run(key, &display_name(key));
    }
    drop(agents);

    match status {
        AgentStatus::Waiting => {
            if prev_status != Some(AgentStatus::Waiting) {
                history::mark_waiting_start(key);
            }
        }
        AgentStatus::Done => history::finish(key, history::EndStatus::Done, exit_code),
        AgentStatus::Crashed => history::finish(key, history::EndStatus::Crashed, exit_code),
        AgentStatus::Running | AgentStatus::IdleMaybe => {
            if prev_status == Some(AgentStatus::Waiting) {
                history::mark_waiting_end(key);
            }
        }
    }
    true
}

#[tauri::command]
pub fn get_agents<R: Runtime>(app: AppHandle<R>) -> Vec<Agent> {
    snapshot(&app)
}

/// PID of a tracked agent, if any — backs F13 "uncover-on-alert"
/// (resolving which display its frontmost window is on). `0` (the
/// wrapper/hook placeholder PID) counts as "unknown".
pub fn pid_for(key: &str) -> Option<u32> {
    let agents = AGENTS.lock().unwrap();
    agents.get(key).map(|t| t.pid).filter(|&pid| pid != 0)
}

pub fn display_name(key: &str) -> String {
    profiles::all()
        .iter()
        .find(|p| p.id == key)
        .map(|p| p.display_name.clone())
        .unwrap_or_else(|| key.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct RunningProcess {
    pub pid: u32,
    pub name: String,
    pub cmd: String,
    pub cwd: Option<String>,
}

/// Backs the Custom-profile process picker: a deduplicated, name-sorted
/// snapshot of currently running processes to search and click through.
#[tauri::command]
pub fn list_running_processes() -> Vec<RunningProcess> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_cwd(UpdateKind::Always)
            .with_cmd(UpdateKind::Always),
    );
    let mut out: Vec<RunningProcess> = sys
        .processes()
        .iter()
        .map(|(pid, process)| RunningProcess {
            pid: pid.as_u32(),
            name: process.name().to_string_lossy().to_string(),
            cmd: process
                .cmd()
                .iter()
                .map(|a| a.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" "),
            cwd: process.cwd().map(|p| p.to_string_lossy().to_string()),
        })
        .filter(|p| !p.name.is_empty())
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// How many currently-running processes a draft set of match rules
/// would match — backs the "Matches N running process(es) ✓" test
/// button in the Custom-profile sheet.
#[tauri::command]
pub fn test_match_rules(rules: Vec<profiles::MatchRule>) -> u32 {
    let processes = list_running_processes();
    processes
        .iter()
        .filter(|p| {
            let name_lower = p.name.to_lowercase();
            let cmd_lower = p.cmd.to_lowercase();
            let cwd_lower = p.cwd.clone().unwrap_or_default().to_lowercase();
            rules.iter().any(|r| r.matches(&name_lower, &cmd_lower, &cwd_lower))
        })
        .count() as u32
}
