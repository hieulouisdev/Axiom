//! Telemetry layer — opt-in only, never on by default.
//!
//! Phase 4.3 — collects anonymous usage metrics when the user explicitly
//! opts in. All data stays local until the user reviews and approves
//! sending. No PII is ever collected.

use serde::{Deserialize, Serialize};

/// Telemetry configuration and state.
///
/// **IMPORTANT**: `enabled` is **always** `false` by default. The user must
/// explicitly opt in. There is no code path that silently enables telemetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Whether telemetry is enabled (always false by default).
    pub enabled: bool,
    /// Whether the user has seen the opt-in prompt.
    pub prompted: bool,
    /// Anonymous installation ID (random UUID, not traceable to user).
    pub install_id: String,
    /// Metrics collected since last send.
    pub pending_metrics: Vec<TelemetryEvent>,
}

/// A single telemetry event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Event kind (e.g. "app_start", "chat_sent", "tool_invoked").
    pub kind: String,
    /// Timestamp in milliseconds since Unix epoch.
    pub timestamp_ms: u64,
    /// Event payload — arbitrary JSON, but must never contain PII.
    pub data: serde_json::Value,
}

/// Summary DTO returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySummary {
    pub enabled: bool,
    pub prompted: bool,
    pub pending_count: usize,
    pub install_id: String,
}

impl TelemetryConfig {
    /// Create a new telemetry config with everything disabled.
    ///
    /// A random UUID is generated for `install_id` so that aggregate
    /// analytics can deduplicate events without identifying the user.
    pub fn new() -> Self {
        Self {
            enabled: false,
            prompted: false,
            install_id: uuid::Uuid::new_v4().to_string(),
            pending_metrics: Vec::new(),
        }
    }

    /// Opt in to telemetry collection.
    ///
    /// After this call, [`record_event`](Self::record_event) will actually
    /// buffer events.
    pub fn opt_in(&mut self) {
        if !self.enabled {
            tracing::info!("telemetry: user opted in");
        }
        self.enabled = true;
        self.prompted = true;
    }

    /// Opt out of telemetry collection.
    ///
    /// Pending metrics are **drained and discarded** immediately so no
    /// stale data is accidentally sent later.
    pub fn opt_out(&mut self) {
        let discarded = self.pending_metrics.len();
        self.enabled = false;
        self.prompted = true;
        self.pending_metrics.clear();
        if discarded > 0 {
            tracing::info!("telemetry: user opted out; discarded {discarded} pending events");
        } else {
            tracing::info!("telemetry: user opted out");
        }
    }

    /// Record a telemetry event.
    ///
    /// If telemetry is disabled the event is silently dropped.
    /// The `data` value must never contain PII — callers are responsible
    /// for sanitization.
    pub fn record_event(&mut self, kind: &str, data: serde_json::Value) {
        if !self.enabled {
            return;
        }
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.pending_metrics.push(TelemetryEvent {
            kind: kind.to_string(),
            timestamp_ms,
            data,
        });

        // Cap at 1000 pending events to prevent unbounded memory growth.
        if self.pending_metrics.len() > 1000 {
            let excess = self.pending_metrics.len() - 1000;
            self.pending_metrics.drain(..excess);
            tracing::warn!("telemetry: dropped {excess} oldest events (cap=1000)");
        }
    }

    /// Return a summary for the frontend.
    pub fn summary(&self) -> TelemetrySummary {
        TelemetrySummary {
            enabled: self.enabled,
            prompted: self.prompted,
            pending_count: self.pending_metrics.len(),
            install_id: self.install_id.clone(),
        }
    }

    /// Drain all pending events, returning them for transmission.
    ///
    /// After this call the internal buffer is empty.
    pub fn drain_pending(&mut self) -> Vec<TelemetryEvent> {
        std::mem::take(&mut self.pending_metrics)
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_config_is_disabled() {
        let cfg = TelemetryConfig::new();
        assert!(!cfg.enabled);
        assert!(!cfg.prompted);
        assert!(!cfg.install_id.is_empty());
    }

    #[test]
    fn opt_in_enables() {
        let mut cfg = TelemetryConfig::new();
        cfg.opt_in();
        assert!(cfg.enabled);
        assert!(cfg.prompted);
    }

    #[test]
    fn opt_out_disables_and_clears() {
        let mut cfg = TelemetryConfig::new();
        cfg.opt_in();
        cfg.record_event("test", serde_json::json!({}));
        assert_eq!(cfg.pending_metrics.len(), 1);

        cfg.opt_out();
        assert!(!cfg.enabled);
        assert!(cfg.prompted);
        assert!(cfg.pending_metrics.is_empty());
    }

    #[test]
    fn record_event_drops_when_disabled() {
        let mut cfg = TelemetryConfig::new();
        cfg.record_event("test", serde_json::json!({}));
        assert!(cfg.pending_metrics.is_empty());
    }

    #[test]
    fn record_event_buffers_when_enabled() {
        let mut cfg = TelemetryConfig::new();
        cfg.opt_in();
        cfg.record_event("app_start", serde_json::json!({"version": "0.7.0"}));
        assert_eq!(cfg.pending_metrics.len(), 1);
        assert_eq!(cfg.pending_metrics[0].kind, "app_start");
    }

    #[test]
    fn cap_at_1000_events() {
        let mut cfg = TelemetryConfig::new();
        cfg.opt_in();
        for i in 0..1005 {
            cfg.record_event("test", serde_json::json!({"i": i}));
        }
        assert_eq!(cfg.pending_metrics.len(), 1000);
    }

    #[test]
    fn drain_empties_buffer() {
        let mut cfg = TelemetryConfig::new();
        cfg.opt_in();
        cfg.record_event("a", serde_json::json!({}));
        cfg.record_event("b", serde_json::json!({}));
        let events = cfg.drain_pending();
        assert_eq!(events.len(), 2);
        assert!(cfg.pending_metrics.is_empty());
    }
}
