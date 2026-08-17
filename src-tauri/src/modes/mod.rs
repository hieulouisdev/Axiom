//! Operational modes: Continuous vs On-Demand.
//!
//! - **Continuous**: the AI is always active. It listens to events (file
//!   changes, schedule ticks, security events) and acts proactively. Higher
//!   cost, lower latency.
//! - **OnDemand**: the AI is dormant until the user explicitly invokes it.
//!   Lowest cost. The security monitor still runs (it's not AI-dependent).

pub mod continuous;
pub mod ondemand;
pub mod watcher;

use serde::{Deserialize, Serialize};

use crate::config::OperatingMode;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Continuous,
    OnDemand,
}

impl From<OperatingMode> for Mode {
    fn from(m: OperatingMode) -> Self {
        match m {
            OperatingMode::Continuous => Mode::Continuous,
            OperatingMode::OnDemand => Mode::OnDemand,
        }
    }
}

impl From<Mode> for OperatingMode {
    fn from(m: Mode) -> Self {
        match m {
            Mode::Continuous => OperatingMode::Continuous,
            Mode::OnDemand => OperatingMode::OnDemand,
        }
    }
}
