//! Native per-display blackout covers for macOS.
//!
//! Each covered display gets its own borderless, pure-black `NSWindow`
//! sitting at `NSScreenSaverWindowLevel` so it sits above the menu bar,
//! Dock, notifications and full-screen apps. All AppKit calls here must
//! run on the main thread; callers are responsible for dispatching onto it
//! (see `cover::dispatch_main`).

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSCursor, NSScreen, NSScreenSaverWindowLevel, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use serde::Serialize;
use std::sync::Mutex;

use super::display_id;

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
    /// Stable across reconnects/port swaps (PRD F7); `None` only for
    /// unusual virtual displays that don't resolve one.
    pub stable_id: Option<String>,
}

/// Wraps a retained NSWindow so it can live in a global static.
/// SAFETY: every access to the inner window happens on the main thread,
/// serialized by `COVERS`'s mutex, so this is sound even though AppKit
/// objects are not thread-safe in general.
struct CoverWindow(Retained<NSWindow>);
unsafe impl Send for CoverWindow {}

static COVERS: Mutex<Vec<Option<CoverWindow>>> = Mutex::new(Vec::new());

fn main_thread() -> MainThreadMarker {
    MainThreadMarker::new().expect("cover operations must run on the main thread")
}

/// Number of connected displays, in the same order `cover_display` indexes into.
pub fn display_count() -> u32 {
    let mtm = main_thread();
    NSScreen::screens(mtm).len() as u32
}

fn ensure_capacity(covers: &mut Vec<Option<CoverWindow>>, len: usize) {
    while covers.len() < len {
        covers.push(None);
    }
}

/// Cover a single display by index (0-based, matches `NSScreen::screens` order).
/// Idempotent: covering an already-covered display is a no-op.
pub fn cover_display(index: u32) {
    let mtm = main_thread();
    let screens = NSScreen::screens(mtm);
    let Some(screen) = screens.iter().nth(index as usize) else {
        return;
    };

    let mut covers = COVERS.lock().unwrap();
    ensure_capacity(&mut covers, index as usize + 1);
    if covers[index as usize].is_some() {
        return;
    }

    let frame = screen.frame();
    let window: Retained<NSWindow> = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer_screen(
            NSWindow::alloc(mtm),
            frame,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
            Some(&screen),
        )
    };

    window.setOpaque(true);
    window.setBackgroundColor(Some(&NSColor::blackColor()));
    window.setHasShadow(false);
    window.setLevel(NSScreenSaverWindowLevel);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    window.setIgnoresMouseEvents(false);
    window.orderFrontRegardless();

    covers[index as usize] = Some(CoverWindow(window));
    NSCursor::hide();
}

/// Remove the cover from a single display, if present.
pub fn uncover_display(index: u32) {
    let mut covers = COVERS.lock().unwrap();
    if let Some(slot) = covers.get_mut(index as usize)
        && let Some(cw) = slot.take()
    {
        cw.0.orderOut(None);
    }
    if covers.iter().all(|c| c.is_none()) {
        NSCursor::unhide();
    }
}

/// Cover every connected display.
pub fn cover_all() {
    let count = display_count();
    for i in 0..count {
        cover_display(i);
    }
}

/// Remove every cover. This is the emergency-restore path: it must never fail.
pub fn uncover_all() {
    let mut covers = COVERS.lock().unwrap();
    for slot in covers.iter_mut() {
        if let Some(cw) = slot.take() {
            cw.0.orderOut(None);
        }
    }
    NSCursor::unhide();
}

/// Real display arrangement for the PRD's "to-scale map". AppKit's
/// coordinate space has Y increasing *upward* with origin at the main
/// display's bottom-left; we flip it here so callers get ordinary
/// top-left-origin, Y-down coordinates matching the OS's visual
/// arrangement panel.
pub fn display_info() -> Vec<DisplayInfo> {
    let mtm = main_thread();
    let screens = NSScreen::screens(mtm);

    let max_top = screens
        .iter()
        .map(|s| {
            let f = s.frame();
            f.origin.y + f.size.height
        })
        .fold(f64::MIN, f64::max);

    screens
        .iter()
        .enumerate()
        .map(|(i, screen)| {
            let f = screen.frame();
            DisplayInfo {
                index: i as u32,
                name: screen.localizedName().to_string(),
                x: f.origin.x,
                y: max_top - (f.origin.y + f.size.height),
                width: f.size.width,
                height: f.size.height,
                scale_factor: screen.backingScaleFactor(),
                is_main: i == 0,
                stable_id: display_id::stable_id_for_screen(&screen),
            }
        })
        .collect()
}

/// Indices of currently covered displays.
pub fn covered_indices() -> Vec<u32> {
    let covers = COVERS.lock().unwrap();
    covers
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.as_ref().map(|_| i as u32))
        .collect()
}

/// Click-through semi-transparent overlay per display (PRD F2). Sits
/// just below the blackout cover's level so a covered display can't be
/// "un-dimmed" by seeing through it, but above ordinary app windows.
struct DimWindow {
    window: Retained<NSWindow>,
    percent: u8,
}
unsafe impl Send for DimWindow {}

static DIMS: Mutex<Vec<Option<DimWindow>>> = Mutex::new(Vec::new());
const DIM_WINDOW_LEVEL: isize = NSScreenSaverWindowLevel - 1;

/// Set a display's dim level (0-100). 0 removes the overlay entirely.
pub fn set_dim(index: u32, percent: u8) {
    let percent = percent.min(100);
    let mtm = main_thread();

    let mut dims = DIMS.lock().unwrap();
    while dims.len() <= index as usize {
        dims.push(None);
    }

    if percent == 0 {
        if let Some(d) = dims[index as usize].take() {
            d.window.orderOut(None);
        }
        return;
    }

    let alpha = percent as f64 / 100.0;

    if let Some(existing) = dims[index as usize].as_mut() {
        existing
            .window
            .setBackgroundColor(Some(&NSColor::colorWithWhite_alpha(0.0, alpha)));
        existing.percent = percent;
        return;
    }

    let screens = NSScreen::screens(mtm);
    let Some(screen) = screens.iter().nth(index as usize) else {
        return;
    };
    let frame = screen.frame();
    let window: Retained<NSWindow> = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer_screen(
            NSWindow::alloc(mtm),
            frame,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
            Some(&screen),
        )
    };
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::colorWithWhite_alpha(0.0, alpha)));
    window.setHasShadow(false);
    window.setLevel(DIM_WINDOW_LEVEL);
    window.setIgnoresMouseEvents(true);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    window.orderFrontRegardless();

    dims[index as usize] = Some(DimWindow { window, percent });
}

/// Current dim percentage per display index (displays with no overlay are omitted).
pub fn dim_state() -> Vec<(u32, u8)> {
    let dims = DIMS.lock().unwrap();
    dims.iter()
        .enumerate()
        .filter_map(|(i, d)| d.as_ref().map(|d| (i as u32, d.percent)))
        .collect()
}
