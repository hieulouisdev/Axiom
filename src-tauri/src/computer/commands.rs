//! Shell command execution with safety gating.

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::{AegisError, Result};

use super::safety::{SafetyDecision, SafetyPolicy};

/// Result of a successful command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// Execute a shell command after running it through the safety policy.
///
/// - If the policy returns `Allow`, the command runs immediately.
/// - If the policy returns `Deny`, returns an error.
/// - If the policy returns `RequireConfirmation`, returns a
///   [`AegisError::SafetyConfirmation`] with a token. The frontend must
///   display the summary/rationale and call back with the token to authorize.
pub fn exec_command(policy: &SafetyPolicy, command: &str) -> Result<ExecResult> {
    match policy.check_command(command) {
        SafetyDecision::Allow => {}
        SafetyDecision::Deny { reason } => {
            return Err(AegisError::SafetyDenial(reason));
        }
        SafetyDecision::RequireConfirmation { token, summary, .. } => {
            return Err(AegisError::SafetyConfirmation { token, summary });
        }
    }
    run_command(command)
}

/// Like [`exec_command`] but skips the safety policy. Used internally by
/// the confirmation flow once the user has explicitly authorized the action.
pub fn exec_command_authorized(command: &str) -> Result<ExecResult> {
    run_command(command)
}

fn run_command(command: &str) -> Result<ExecResult> {
    let start = std::time::Instant::now();

    let (mut cmd, is_unix) = build_command(command);

    let output = cmd
        .output()
        .map_err(|e| AegisError::Io(format!("failed to spawn command '{command}': {e}")))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let exit_code = output.status.code().unwrap_or(if is_unix { 143 } else { 1 });

    Ok(ExecResult {
        command: command.to_string(),
        stdout,
        stderr,
        exit_code,
        duration_ms,
    })
}

fn build_command(command: &str) -> (Command, bool) {
    #[cfg(unix)]
    {
        let mut c = Command::new("sh");
        c.arg("-c").arg(command);
        (c, true)
    }
    #[cfg(not(unix))]
    {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(command);
        (c, false)
    }
}
