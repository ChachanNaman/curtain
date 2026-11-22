//! Native per-display blackout covers for Windows.
//!
//! Each covered display gets its own borderless, black, click-capturing
//! top-level window (`WS_POPUP` + `WS_EX_TOPMOST | WS_EX_TOOLWINDOW |
//! WS_EX_NOACTIVATE`) sized to that monitor's full bounds — the same
//! idea as the macOS `NSScreenSaverWindowLevel` cover, just built from
//! raw Win32 calls since there's no cross-platform windowing crate
//! that exposes per-monitor always-on-top borderless windows directly.
//! These windows are created on whichever thread calls into this
//! module; `cover::mod.rs` already dispatches every call through
//! Tauri's `run_on_main_thread`, so window-handle thread affinity is
//! satisfied the same way it is on macOS.

use serde::Serialize;
use std::sync::{Mutex, OnceLock};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, HBRUSH, HDC,
    HMONITOR, MONITORINFO, MONITORINFOEXW, DISPLAY_DEVICEW,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, LoadCursorW, RegisterClassExW,
    SetLayeredWindowAttributes, SetWindowPos, ShowWindow, HWND_TOPMOST, IDC_ARROW,
    LAYERED_WINDOW_ATTRIBUTES_FLAGS, LWA_ALPHA, SET_WINDOW_POS_FLAGS,
    SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_SHOWWINDOW, WINDOW_EX_STYLE, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

const COVER_CLASS: PCWSTR = w!("CurtainCoverWindow");
const DIM_CLASS: PCWSTR = w!("CurtainDimWindow");
const EDD_GET_DEVICE_INTERFACE_NAME: u32 = 0x00000001;

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
    /// Stable across reconnects/port swaps (PRD F7): the monitor's
    /// `DISPLAY_DEVICE.DeviceID`, which encodes the EDID-derived
    /// hardware identity rather than which port/adapter it's plugged
    /// into — the Windows equivalent of macOS's CGDisplay UUID.
    pub stable_id: Option<String>,
}

/// A raw `HWND` isn't `Send`/`Sync` by default; every access to these
/// goes through `COVERS`'/`DIMS`' mutex, serializing them the same way
/// the macOS `CoverWindow` wrapper does for its `NSWindow`.
struct Handle(HWND);
unsafe impl Send for Handle {}

static COVERS: Mutex<Vec<Option<Handle>>> = Mutex::new(Vec::new());
struct DimHandle {
    hwnd: Handle,
    percent: u8,
}
static DIMS: Mutex<Vec<Option<DimHandle>>> = Mutex::new(Vec::new());

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // These overlay windows never need custom message handling — in
    // particular, never PostQuitMessage here, since tao/wry's own
    // message loop runs on this same thread and sending WM_QUIT from
    // one of our auxiliary windows would tear down the whole app.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn register_class_once(class_name: PCWSTR) {
    static REGISTERED: OnceLock<Mutex<Vec<usize>>> = OnceLock::new();
    let registered = REGISTERED.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = registered.lock().unwrap();
    let key = class_name.0 as usize;
    if guard.contains(&key) {
        return;
    }
    unsafe {
        let hinstance = GetModuleHandleW(None).unwrap_or_default();
        let brush: HBRUSH = CreateSolidBrush(windows::Win32::Foundation::COLORREF(0x00000000));
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: brush,
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassExW(&wc);
    }
    guard.push(key);
}

struct Monitor {
    rect: RECT,
    device_name: [u16; 32],
    is_primary: bool,
}

unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    rect: *mut RECT,
    lparam: LPARAM,
) -> windows::Win32::Foundation::BOOL {
    unsafe {
        let out = &mut *(lparam.0 as *mut Vec<Monitor>);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(hmonitor, &mut info.monitorInfo as *mut MONITORINFO).as_bool() {
            out.push(Monitor {
                rect: *rect,
                device_name: info.szDevice,
                is_primary: (info.monitorInfo.dwFlags & 1) != 0, // MONITORINFOF_PRIMARY
            });
        }
        windows::Win32::Foundation::TRUE
    }
}

/// Enumerated fresh on every call (never cached) so reconnects/
/// disconnects are always reflected immediately, matching the macOS
/// side's `NSScreen::screens()` call pattern.
fn enumerate_monitors() -> Vec<Monitor> {
    let mut monitors: Vec<Monitor> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(monitor_enum_proc),
            LPARAM(&mut monitors as *mut Vec<Monitor> as isize),
        );
    }
    // Primary display first, so index 0 matches "is_main" the same way
    // NSScreen::screens() puts the main display first.
    monitors.sort_by_key(|m| !m.is_primary);
    monitors
}

fn utf16_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// Walks the two-level `EnumDisplayDevicesW` hierarchy (graphics
/// adapters, then the monitors attached to each one) to find the
/// persistent `DeviceID` for the monitor whose device name matches
/// `target_device_name` — this needs `EDD_GET_DEVICE_INTERFACE_NAME`
/// on the *monitor* enumeration call, not the adapter one.
fn stable_id_for_device_name(target_device_name: &[u16; 32]) -> Option<String> {
    unsafe {
        let mut adapter_index = 0u32;
        loop {
            let mut adapter = DISPLAY_DEVICEW {
                cb: size_of::<DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };
            if !EnumDisplayDevicesW(None, adapter_index, &mut adapter, 0).as_bool() {
                return None;
            }
            let mut monitor_index = 0u32;
            loop {
                let mut monitor = DISPLAY_DEVICEW {
                    cb: size_of::<DISPLAY_DEVICEW>() as u32,
                    ..Default::default()
                };
                let adapter_name = PCWSTR(adapter.DeviceName.as_ptr());
                if !EnumDisplayDevicesW(
                    adapter_name,
                    monitor_index,
                    &mut monitor,
                    EDD_GET_DEVICE_INTERFACE_NAME,
                )
                .as_bool()
                {
                    break;
                }
                if adapter.DeviceName == *target_device_name {
                    return Some(utf16_to_string(&monitor.DeviceID));
                }
                monitor_index += 1;
            }
            adapter_index += 1;
        }
    }
}

pub fn display_count() -> u32 {
    enumerate_monitors().len() as u32
}

pub fn display_info() -> Vec<DisplayInfo> {
    enumerate_monitors()
        .iter()
        .enumerate()
        .map(|(i, m)| DisplayInfo {
            index: i as u32,
            name: utf16_to_string(&m.device_name),
            x: m.rect.left as f64,
            y: m.rect.top as f64,
            width: (m.rect.right - m.rect.left) as f64,
            height: (m.rect.bottom - m.rect.top) as f64,
            // Windows doesn't expose a simple per-monitor scale factor
            // through this API without per-monitor-DPI-aware GDI calls
            // this crate doesn't currently pull in; 1.0 until that's
            // added (TODO).
            scale_factor: 1.0,
            is_main: m.is_primary,
            stable_id: stable_id_for_device_name(&m.device_name),
        })
        .collect()
}

fn create_overlay(class_name: PCWSTR, rect: &RECT, ex_style: WINDOW_EX_STYLE) -> Option<HWND> {
    register_class_once(class_name);
    unsafe {
        let hinstance = GetModuleHandleW(None).ok()?;
        let hwnd = CreateWindowExW(
            ex_style,
            class_name,
            w!(""),
            WS_POPUP,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            None,
            None,
            hinstance,
            None,
        )
        .ok()?;
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_SHOWWINDOW | SET_WINDOW_POS_FLAGS(0x0001 | 0x0002), // SWP_NOSIZE | SWP_NOMOVE
        );
        Some(hwnd)
    }
}

fn destroy_overlay(hwnd: HWND) {
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

pub fn cover_display(index: u32) {
    let monitors = enumerate_monitors();
    let Some(monitor) = monitors.get(index as usize) else { return };

    let mut covers = COVERS.lock().unwrap();
    while covers.len() <= index as usize {
        covers.push(None);
    }
    if covers[index as usize].is_some() {
        return;
    }
    if let Some(hwnd) = create_overlay(COVER_CLASS, &monitor.rect, WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) {
        covers[index as usize] = Some(Handle(hwnd));
    }
}

pub fn uncover_display(index: u32) {
    let mut covers = COVERS.lock().unwrap();
    if let Some(slot) = covers.get_mut(index as usize)
        && let Some(h) = slot.take()
    {
        destroy_overlay(h.0);
    }
}

pub fn cover_all() {
    let count = display_count();
    for i in 0..count {
        cover_display(i);
    }
}

pub fn uncover_all() {
    let mut covers = COVERS.lock().unwrap();
    for slot in covers.iter_mut() {
        if let Some(h) = slot.take() {
            destroy_overlay(h.0);
        }
    }
}

pub fn covered_indices() -> Vec<u32> {
    let covers = COVERS.lock().unwrap();
    covers.iter().enumerate().filter_map(|(i, c)| c.as_ref().map(|_| i as u32)).collect()
}

/// Click-through semi-transparent overlay per display (PRD F2) —
/// `WS_EX_TRANSPARENT` lets clicks fall through to whatever's
/// underneath, `WS_EX_LAYERED` + `SetLayeredWindowAttributes` gives it
/// the alpha blend.
pub fn set_dim(index: u32, percent: u8) {
    let percent = percent.min(100);
    let mut dims = DIMS.lock().unwrap();
    while dims.len() <= index as usize {
        dims.push(None);
    }

    if percent == 0 {
        if let Some(d) = dims[index as usize].take() {
            destroy_overlay(d.hwnd.0);
        }
        return;
    }

    let alpha = ((percent as u32 * 255) / 100) as u8;

    if let Some(existing) = dims[index as usize].as_mut() {
        unsafe {
            let _ = SetLayeredWindowAttributes(
                existing.hwnd.0,
                windows::Win32::Foundation::COLORREF(0),
                alpha,
                LAYERED_WINDOW_ATTRIBUTES_FLAGS(LWA_ALPHA.0),
            );
        }
        existing.percent = percent;
        return;
    }

    let monitors = enumerate_monitors();
    let Some(monitor) = monitors.get(index as usize) else { return };
    let Some(hwnd) = create_overlay(
        DIM_CLASS,
        &monitor.rect,
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TRANSPARENT,
    ) else {
        return;
    };
    unsafe {
        let _ = SetLayeredWindowAttributes(
            hwnd,
            windows::Win32::Foundation::COLORREF(0),
            alpha,
            LAYERED_WINDOW_ATTRIBUTES_FLAGS(LWA_ALPHA.0),
        );
    }
    dims[index as usize] = Some(DimHandle { hwnd: Handle(hwnd), percent });
}

pub fn dim_state() -> Vec<(u32, u8)> {
    let dims = DIMS.lock().unwrap();
    dims.iter().enumerate().filter_map(|(i, d)| d.as_ref().map(|d| (i as u32, d.percent))).collect()
}
