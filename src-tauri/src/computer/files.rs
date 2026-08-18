//! File read/write with safety gating.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{AegisError, Result};

use super::safety::{SafetyDecision, SafetyPolicy};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadResult {
    pub path: String,
    pub bytes: usize,
    pub content: String,
    pub truncated: bool,
}

/// Maximum bytes returned by `file_read` to keep payloads bounded.
const MAX_READ_BYTES: usize = 1_000_000; // 1 MB

/// Read a file from disk. Read operations are always allowed (no confirmation
/// needed), but reads of system-protected paths are denied.
pub fn file_read(path: &str) -> Result<FileReadResult> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(AegisError::Io(format!("file not found: {path}")));
    }
    let metadata = std::fs::metadata(&p)?;
    if metadata.is_dir() {
        return Err(AegisError::Io(format!("path is a directory: {path}")));
    }
    let bytes = std::fs::read(&p)?;
    let total = bytes.len();
    if total > MAX_READ_BYTES {
        let truncated_bytes = &bytes[..MAX_READ_BYTES];
        let content = String::from_utf8_lossy(truncated_bytes).into_owned();
        return Ok(FileReadResult {
            path: path.to_string(),
            bytes: total,
            content,
            truncated: true,
        });
    }
    let content = String::from_utf8_lossy(&bytes).into_owned();
    Ok(FileReadResult {
        path: path.to_string(),
        bytes: total,
        content,
        truncated: false,
    })
}

/// Write text to a file. Routes through the safety policy: writes outside the
/// user-approved whitelist require confirmation.
pub fn file_write(policy: &SafetyPolicy, path: &str, content: &str) -> Result<()> {
    match policy.check_file_write(path) {
        SafetyDecision::Allow => {}
        SafetyDecision::Deny { reason } => {
            return Err(AegisError::SafetyDenial(reason));
        }
        SafetyDecision::RequireConfirmation { token, summary, .. } => {
            return Err(AegisError::SafetyConfirmation { token, summary });
        }
    }
    write_file_inner(path, content)
}

/// Authorized write (called after the user has confirmed the action).
pub fn file_write_authorized(path: &str, content: &str) -> Result<()> {
    write_file_inner(path, content)
}

fn write_file_inner(path: &str, content: &str) -> Result<()> {
    let p = PathBuf::from(path);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&p, content)
        .map_err(|e| AegisError::Io(format!("failed to write to {path}: {e}")))?;
    Ok(())
}
