//! Windows side of F13 "uncover-on-alert" — maps a process to the
//! display its frontmost window lives on, the same job
//! `cover::window_display` does on macOS via CGWindowListCopyWindowInfo.
//! `EnumWindows` enumerates top-level windows in top-to-bottom Z order,
//! so the first visible, non-minimized window belonging to `pid` is its
//! frontmost one.

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{MonitorFromRect, MONITOR_DEFAULTTONULL};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
};

struct SearchState {
    target_pid: u32,
    found_rect: Option<RECT>,
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let state = &mut *(lparam.0 as *mut SearchState);
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return TRUE;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != state.target_pid {
            return TRUE;
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            state.found_rect = Some(rect);
        }
        // Found the frontmost matching window — stop enumerating.
        windows::Win32::Foundation::FALSE
    }
}

fn frontmost_window_rect(pid: u32) -> Option<RECT> {
    let mut state = SearchState { target_pid: pid, found_rect: None };
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut state as *mut SearchState as isize));
    }
    state.found_rect
}

/// Which display index (matching `cover::display_info()`'s order)
/// contains `pid`'s frontmost window, if resolvable.
pub fn display_index_for_pid<R: tauri::Runtime>(app: &tauri::AppHandle<R>, pid: u32) -> Option<u32> {
    let rect = frontmost_window_rect(pid)?;
    let hmonitor = unsafe { MonitorFromRect(&rect, MONITOR_DEFAULTTONULL) };
    if hmonitor.is_invalid() {
        return None;
    }
    let cx = (rect.left + rect.right) as f64 / 2.0;
    let cy = (rect.top + rect.bottom) as f64 / 2.0;
    super::get_displays(app.clone())
        .into_iter()
        .find(|d| cx >= d.x && cx < d.x + d.width && cy >= d.y && cy < d.y + d.height)
        .map(|d| d.index)
}
