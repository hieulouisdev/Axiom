//! Security subsystem: passive monitoring, threat detection, active defense,
//! file quarantine, network anomaly detection, file integrity monitoring,
//! and alert notifications.
//!
//! v0.6 adds [`yara`] — a pure-Rust YARA rule loader + stop-gap matcher
//! that lets users drop `.yar` / `.yara` files into their data directory
//! to extend the on-demand scanner with custom indicators of compromise.

pub mod alerts;
pub mod defender;
pub mod integrity;
pub mod monitor;
pub mod network;
pub mod quarantine;
pub mod sandbox;
pub mod scanner;
pub mod telemetry;
pub mod yara;

pub use alerts::{AlertConfig, send_alert};
pub use defender::{start as start_defender, DefenseEvent};
pub use integrity::{IntegrityEvent, check_integrity, save_baseline, critical_files, file_hash};
pub use monitor::{start as start_monitor, ProcessSnapshot, Threat};
pub use network::{NetworkAnomaly, SocketInfo, detect_anomalies};
pub use quarantine::{QuarantineEntry, QuarantineStore};
pub use scanner::{scan_directory, scan_file, ScanResult};
pub use sandbox::{SandboxPolicy};
pub use telemetry::{TelemetryConfig, TelemetryEvent, TelemetrySummary};
pub use yara::{YaraRule, load_all as load_yara_rules, scan_file as yara_scan_file};

/// Severity levels shared across the security subsystem.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}
