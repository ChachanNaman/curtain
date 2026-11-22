//! Stable per-display identifiers (PRD F7), macOS implementation.
//!
//! `NSScreen` order shifts when monitors are plugged into different
//! ports or in a different order, so display-specific hotkey bindings
//! can't key off index. `CGDisplayCreateUUIDFromDisplayID` returns a
//! UUID tied to the physical panel/EDID, stable across reconnects and
//! port swaps — this is what System Settings' arrangement panel uses
//! internally too.

use core_foundation::base::TCFType;
use core_foundation::uuid::{CFUUID, CFUUIDGetUUIDBytes, CFUUIDRef};
use objc2_app_kit::NSScreen;
use objc2_foundation::{NSNumber, NSString};

type CGDirectDisplayID = u32;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGDisplayCreateUUIDFromDisplayID(display: CGDirectDisplayID) -> CFUUIDRef;
}

fn format_uuid(bytes: [u8; 16]) -> String {
    let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn direct_display_id(screen: &NSScreen) -> Option<CGDirectDisplayID> {
    let desc = screen.deviceDescription();
    let key = NSString::from_str("NSScreenNumber");
    let value = desc.objectForKey(&key)?;
    let number = value.downcast::<NSNumber>().ok()?;
    Some(number.as_u32())
}

/// Stable UUID string for a display, or `None` if it couldn't be
/// resolved (headless/virtual displays without a UUID).
pub fn stable_id_for_screen(screen: &NSScreen) -> Option<String> {
    let direct_id = direct_display_id(screen)?;
    let uuid_ref = unsafe { CGDisplayCreateUUIDFromDisplayID(direct_id) };
    if uuid_ref.is_null() {
        return None;
    }
    let uuid: CFUUID = unsafe { TCFType::wrap_under_create_rule(uuid_ref) };
    let b = unsafe { CFUUIDGetUUIDBytes(uuid.as_concrete_TypeRef()) };
    let bytes = [
        b.byte0, b.byte1, b.byte2, b.byte3, b.byte4, b.byte5, b.byte6, b.byte7, b.byte8, b.byte9,
        b.byte10, b.byte11, b.byte12, b.byte13, b.byte14, b.byte15,
    ];
    Some(format_uuid(bytes))
}
