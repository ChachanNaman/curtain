//! Windows cover implementation — stub for M1 (Mac-first milestone, see PRD M5).
//! Will use WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE windows sized
//! to each monitor's full bounds, re-asserted topmost on a 1s timer.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfo {
    pub index: u32,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale_factor: f64,
    pub is_main: bool,
    /// TODO(M5): populate from the monitor's device path
    /// (`EnumDisplayDevices` → `DISPLAY_DEVICE.DeviceID`), which is
    /// stable across port/order changes the same way the macOS
    /// CGDisplay UUID is.
    pub stable_id: Option<String>,
}

pub fn display_count() -> u32 {
    0
}

pub fn cover_display(_index: u32) {}

pub fn uncover_display(_index: u32) {}

pub fn cover_all() {}

pub fn uncover_all() {}

pub fn covered_indices() -> Vec<u32> {
    Vec::new()
}

pub fn display_info() -> Vec<DisplayInfo> {
    Vec::new()
}

pub fn set_dim(_index: u32, _percent: u8) {}

pub fn dim_state() -> Vec<(u32, u8)> {
    Vec::new()
}
