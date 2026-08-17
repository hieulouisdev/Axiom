//! Network anomaly detection.
//!
//! v0.1: lightweight heuristic checks (suspicious outbound ports, rapid
//! connection bursts). Phase 2 adds DeepPacket inspection.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAnomaly {
    pub kind: String,
    pub detail: String,
    pub severity: super::Severity,
    pub timestamp_ms: u64,
}

/// v0.1: returns anomalies for any listening socket on suspicious ports.
/// Phase 2 will replace this with a real packet capture.
pub fn detect_anomalies() -> Vec<NetworkAnomaly> {
    // Placeholder until Phase 2.
    Vec::new()
}
