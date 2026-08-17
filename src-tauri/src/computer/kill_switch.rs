//! Kill switch: a process-wide boolean that, when tripped, immediately halts
//! every running agent loop. The frontend can flip it via the
//! `safety_trip_kill_switch` Tauri command (e.g. from a big red "STOP" button).
//!
//! Once tripped, the switch stays tripped until `reset()` is called. This
//! prevents the AI from re-launching itself immediately after being stopped.

use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

static TRIPPED: AtomicBool = AtomicBool::new(false);

/// Returns true once the kill switch has been tripped. Stays true until
/// [`reset`] is called.
pub fn is_tripped() -> bool {
    TRIPPED.load(Ordering::SeqCst)
}

/// Trip the kill switch. All running agent loops will abort on their next
/// iteration check.
pub fn trip() {
    TRIPPED.store(true, Ordering::SeqCst);
    tracing::warn!("KILL SWITCH TRIPPED — all agent loops will halt");
}

/// Reset the kill switch. Allows new agent runs to start.
pub fn reset() {
    TRIPPED.store(false, Ordering::SeqCst);
    tracing::info!("kill switch reset — agent runs permitted again");
}

/// Helper for tests.
#[cfg(test)]
pub fn _force_state(v: bool) {
    TRIPPED.store(v, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trip_and_reset_round_trip() {
        // Make sure we leave the switch in the un-tripped state.
        let original = is_tripped();
        reset();
        assert!(!is_tripped());
        trip();
        assert!(is_tripped());
        trip();
        assert!(is_tripped());
        reset();
        assert!(!is_tripped());
        // Restore (in case another test relied on the original state).
        if original {
            trip();
        }
    }
}
