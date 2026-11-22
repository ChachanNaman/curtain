//! Persisted alert settings (token + ntfy topic + channel toggles).
//! Lives outside Tauri's plugin-store because the `curtain-notify`
//! sidecar (a separate process, no Tauri runtime) needs to read the
//! token too.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn random_string(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertsConfig {
    pub token: String,
    pub ntfy_topic: String,
    pub notification_enabled: bool,
    pub sound_enabled: bool,
    pub ntfy_enabled: bool,
    /// F13: lift the cover only on the display holding a waiting
    /// agent's terminal, instead of leaving every display covered
    /// until a manual restore.
    #[serde(default = "default_true")]
    pub uncover_on_alert_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AlertsConfig {
    fn default() -> Self {
        Self {
            token: random_string(32),
            ntfy_topic: format!("curtain-{}", random_string(12)),
            notification_enabled: true,
            sound_enabled: true,
            ntfy_enabled: true,
            uncover_on_alert_enabled: true,
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("dev.naman.curtain")
        .join("alerts.json")
}

/// Path the `curtain-notify` sidecar reads from. Kept as a free function
/// (not a method) since the sidecar links this same module but never
/// touches the rest of the app.
pub fn load() -> AlertsConfig {
    let path = config_path();
    if let Ok(bytes) = fs::read(&path)
        && let Ok(cfg) = serde_json::from_slice::<AlertsConfig>(&bytes)
    {
        return cfg;
    }
    let cfg = AlertsConfig::default();
    save(&cfg);
    cfg
}

pub fn save(cfg: &AlertsConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(cfg) {
        let _ = fs::write(path, bytes);
    }
}

static CONFIG: OnceLock<std::sync::Mutex<AlertsConfig>> = OnceLock::new();

pub fn get() -> AlertsConfig {
    CONFIG
        .get_or_init(|| std::sync::Mutex::new(load()))
        .lock()
        .unwrap()
        .clone()
}

pub fn update(f: impl FnOnce(&mut AlertsConfig)) -> AlertsConfig {
    let cell = CONFIG.get_or_init(|| std::sync::Mutex::new(load()));
    let mut guard = cell.lock().unwrap();
    f(&mut guard);
    save(&guard);
    guard.clone()
}
