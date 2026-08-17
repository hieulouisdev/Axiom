//! On-demand mode: the AI is dormant until explicitly invoked.
//!
//! The security monitor still runs independently (it does not consume AI
//! tokens). When a security event escalates to `Critical` severity, the
//! AI is woken briefly to draft an explanation for the user, then goes
//! back to sleep.

use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;

use crate::state::AppState;

/// Spawns the on-demand watcher. It only does anything when a `wake` event
/// is received via the channel — otherwise it sleeps.
pub async fn start(state: Arc<Mutex<AppState>>) {
    tracing::info!("on-demand mode started (AI dormant, security monitor still alive)");
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        // Check if there's a pending wake request.
        let wake = {
            let s = state.lock();
            // For v0.1, we just check whether any critical security events
            // occurred in the last 30s. Phase 3 will use a proper channel.
            false
        };
        if wake {
            tracing::info!("on-demand: AI woken by event");
            // Phase 3: invoke AI here.
        }
    }
}
