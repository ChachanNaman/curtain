//! Touch ID / device-password unlock (PRD F8), macOS implementation via
//! LocalAuthentication. `LAPolicy::DeviceOwnerAuthentication` falls back
//! to the device password automatically if biometrics aren't available
//! or fail, so a user can never be locked out.

use block2::RcBlock;
use objc2::rc::Retained;
use objc2_foundation::NSString;
use objc2_local_authentication::{LAContext, LAPolicy};
use std::sync::mpsc;
use std::time::Duration;

/// Blocks the calling thread until the user authenticates or cancels.
/// Safe to call from any thread (LocalAuthentication is not main-thread-only).
pub fn authenticate(reason: &str) -> bool {
    let context: Retained<LAContext> = unsafe { LAContext::new() };
    let reason = NSString::from_str(reason);
    let (tx, rx) = mpsc::channel::<bool>();

    let block = RcBlock::new(move |success: objc2::runtime::Bool, _error: *mut objc2_foundation::NSError| {
        let _ = tx.send(success.as_bool());
    });

    unsafe {
        context.evaluatePolicy_localizedReason_reply(
            LAPolicy::DeviceOwnerAuthentication,
            &reason,
            &block,
        );
    }

    // The OS prompt has no hard timeout, but if something goes wrong we
    // shouldn't hang the restore flow forever: fail closed after 2 minutes.
    rx.recv_timeout(Duration::from_secs(120)).unwrap_or(false)
}
