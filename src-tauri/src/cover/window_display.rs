//! Maps a process to the display its frontmost window lives on (PRD
//! F13 "uncover-on-alert": lift the cover only where the waiting
//! agent's terminal actually is). Uses `CGWindowListCopyWindowInfo`,
//! which needs no special entitlement or permission to read window
//! geometry/owner (only window *titles* are permission-gated).

use core_foundation::array::CFArrayGetCount;
use core_foundation::array::CFArrayGetValueAtIndex;
use core_foundation::base::CFRelease;
use core_foundation::dictionary::{CFDictionaryGetValueIfPresent, CFDictionaryRef};
use core_foundation::number::{kCFNumberSInt64Type, CFNumberGetValue, CFNumberRef};
use core_foundation::string::CFString;
use std::os::raw::c_void;

type CGWindowID = u32;
type CFArrayRef = core_foundation::array::CFArrayRef;

#[repr(C)]
#[derive(Default)]
struct CGPoint {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Default)]
struct CGSize {
    width: f64,
    height: f64,
}
#[repr(C)]
#[derive(Default)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: CGWindowID) -> CFArrayRef;
    fn CGRectMakeWithDictionaryRepresentation(dict: CFDictionaryRef, rect: *mut CGRect) -> u8;
}

const OPTION_ON_SCREEN_ONLY: u32 = 1;
const NULL_WINDOW_ID: CGWindowID = 0;

unsafe fn dict_i64(dict: CFDictionaryRef, key: &CFString) -> Option<i64> {
    unsafe {
        let mut value: *const c_void = std::ptr::null();
        let key_ptr = key.as_concrete_TypeRef() as *const c_void;
        if CFDictionaryGetValueIfPresent(dict, key_ptr, &mut value) == 0 || value.is_null() {
            return None;
        }
        let mut out: i64 = 0;
        let ok = CFNumberGetValue(value as CFNumberRef, kCFNumberSInt64Type, &mut out as *mut i64 as *mut c_void);
        if ok {
            Some(out)
        } else {
            None
        }
    }
}

unsafe fn dict_rect(dict: CFDictionaryRef, key: &CFString) -> Option<(f64, f64, f64, f64)> {
    unsafe {
        let mut value: *const c_void = std::ptr::null();
        let key_ptr = key.as_concrete_TypeRef() as *const c_void;
        if CFDictionaryGetValueIfPresent(dict, key_ptr, &mut value) == 0 || value.is_null() {
            return None;
        }
        let mut rect = CGRect::default();
        let ok = CGRectMakeWithDictionaryRepresentation(value as CFDictionaryRef, &mut rect);
        if ok != 0 {
            Some((rect.origin.x, rect.origin.y, rect.size.width, rect.size.height))
        } else {
            None
        }
    }
}

use core_foundation::base::TCFType;

/// Bounding rect (global display coords, origin top-left, Y-down — the
/// same space `cover::display_info()` reports in) of `pid`'s frontmost
/// normal on-screen window, if any.
fn frontmost_window_rect(pid: u32) -> Option<(f64, f64, f64, f64)> {
    let array_ref = unsafe { CGWindowListCopyWindowInfo(OPTION_ON_SCREEN_ONLY, NULL_WINDOW_ID) };
    if array_ref.is_null() {
        return None;
    }

    let owner_key = CFString::new("kCGWindowOwnerPID");
    let layer_key = CFString::new("kCGWindowLayer");
    let bounds_key = CFString::new("kCGWindowBounds");

    let result = unsafe {
        let count = CFArrayGetCount(array_ref);
        let mut found = None;
        for i in 0..count {
            let dict = CFArrayGetValueAtIndex(array_ref, i) as CFDictionaryRef;
            if dict.is_null() {
                continue;
            }
            let Some(owner_pid) = dict_i64(dict, &owner_key) else { continue };
            if owner_pid as u32 != pid {
                continue;
            }
            // Layer 0 is a normal window; skip menu-bar extras/overlays.
            if dict_i64(dict, &layer_key) != Some(0) {
                continue;
            }
            if let Some(rect) = dict_rect(dict, &bounds_key) {
                found = Some(rect);
                break;
            }
        }
        found
    };

    unsafe { CFRelease(array_ref as *const c_void) };
    result
}

/// Which display index (matching `cover::display_info()`'s order)
/// contains the center of `pid`'s frontmost window, if resolvable.
pub fn display_index_for_pid<R: tauri::Runtime>(app: &tauri::AppHandle<R>, pid: u32) -> Option<u32> {
    let (x, y, w, h) = frontmost_window_rect(pid)?;
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    super::get_displays(app.clone())
        .into_iter()
        .find(|d| cx >= d.x && cx < d.x + d.width && cy >= d.y && cy < d.y + d.height)
        .map(|d| d.index)
}
