//! Continuous mode: keeps the AI warm, listens to events, and acts proactively.
//!
//! v0.1: starts a heartbeat task that ticks every 60s. Phase 3 will wire
//! it to a richer event bus (file watchers, calendar, security events).

use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};

use crate::state::AppState;

/// Spawn the continuous-mode heartbeat. No-op if already running.
pub async fn start(state: Arc<Mutex<AppState>>, app: AppHandle) {
    tracing::info!("continuous mode started");
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        // Phase 3: pull pending events from the bus and feed them to the AI.
        // For v0.1, just emit a heartbeat event to keep the UI informed.
        let _ = app.emit(
            "mode://heartbeat",
            serde_json::json!({
                "ts_ms": time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000,
                "mode": "continuous",
            }),
        );
        // Activity heartbeat (lightweight — no AI call yet).
        {
            let s = state.lock();
            let _ = s
                .memory
                .activity
                .record("heartbeat", "continuous-mode tick", None);

            // v1.6: tickle the proactive intelligence engine so it can
            // surface new insights to the UI. The engine itself decides
            // whether to do anything based on its internal counter.
            s.proactive.tick(&s.memory, &app);
        }
    }
}
