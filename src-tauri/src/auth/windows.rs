//! Windows Hello unlock — stub for M4 (Mac-first milestone, see PRD M5).
//! Will use `UserConsentVerifier.RequestVerificationAsync` via the
//! `windows` crate.

pub fn authenticate(_reason: &str) -> bool {
    true
}
