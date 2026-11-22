//! Windows Hello unlock (PRD F8) via `UserConsentVerifier`. This is the
//! same API Windows itself uses for "Windows Hello" prompts anywhere in
//! the OS — it already falls back to the device PIN/password on its
//! own if biometrics aren't set up or fail, so a user can never be
//! locked out, matching the macOS side's LocalAuthentication behavior.

use windows::core::HSTRING;
use windows::Security::Credentials::UI::{UserConsentVerificationResult, UserConsentVerifier};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

/// Blocks the calling thread until the user authenticates or cancels.
/// WinRT calls need the calling thread COM-initialized; Tauri command
/// handlers don't guarantee a persistent apartment across calls, so
/// this initializes on every call — idempotent and cheap if the
/// thread is already initialized.
pub fn authenticate(reason: &str) -> bool {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let message = HSTRING::from(reason);
    let Ok(op) = UserConsentVerifier::RequestVerificationAsync(&message) else {
        return false;
    };
    // `.get()` blocks the calling thread until the async operation
    // completes — there's no OS-level timeout on the Windows Hello
    // prompt either, same as the macOS side's LAContext call.
    match op.get() {
        Ok(result) => result == UserConsentVerificationResult::Verified,
        Err(_) => false,
    }
}
