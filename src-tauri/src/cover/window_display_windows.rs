//! Windows side of F13 "uncover-on-alert" — stub for M1 (Mac-first
//! milestone, see PRD M5), matching `cover::windows`'s own stub state.
//!
//! TODO(M5): `EnumWindows` + `GetWindowThreadProcessId` to find the
//! given pid's top-level window, then `MonitorFromWindow` +
//! `GetMonitorInfoW` to resolve which display it's on.

pub fn display_index_for_pid<R: tauri::Runtime>(_app: &tauri::AppHandle<R>, _pid: u32) -> Option<u32> {
    None
}
