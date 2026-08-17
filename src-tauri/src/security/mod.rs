//! Security subsystem: passive monitoring, threat detection, active defense,
//! and file quarantine.
//!
//! Design principles:
//! - Detection is conservative: false positives are acceptable, false
//!   negatives are not.
//! - Active defense never escalates beyond blocking/quarantining; we never
//!   retaliate against the attacker's host (we just protect the user's).
//! - Every defensive action is logged to the memory store and surfaced
//!   to the user via a notification + the security panel.
//! - The user can always disable auto-defense from Settings → Security.

pub mod defender;
pub mod monitor;
pub mod network;
pub mod quarantine;
pub mod scanner;

pub use defender::{start as start_defender, DefenseEvent};
pub use monitor::{start as start_monitor, ProcessSnapshot, Threat};
pub use quarantine::{QuarantineEntry, QuarantineStore};
pub use scanner::{scan_directory, scan_file, ScanResult};

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
