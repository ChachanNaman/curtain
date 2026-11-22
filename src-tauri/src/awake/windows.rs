//! Windows keep-awake — stub for M1 (Mac-first milestone, see PRD M5).
//! Will use `SetThreadExecutionState(ES_SYSTEM_REQUIRED | ES_CONTINUOUS)`.

pub fn enable(_reason: &str) {}

pub fn disable() {}

pub fn is_enabled() -> bool {
    false
}
