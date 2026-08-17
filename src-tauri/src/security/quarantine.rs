//! File quarantine: moves suspected-malicious files into a sandboxed
//! directory under the user's data dir, with a manifest so they can be
//! restored later.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineEntry {
    pub id: String,
    pub original_path: String,
    pub quarantined_path: String,
    pub quarantined_at_ms: u64,
    pub signature_name: Option<String>,
    pub size_bytes: u64,
}

/// In-memory store for v0.1. Phase 2 persists to SQLite.
pub struct QuarantineStore {
    entries: Vec<QuarantineEntry>,
}

impl QuarantineStore {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn list(&self) -> &[QuarantineEntry] {
        &self.entries
    }

    /// Moves a file into the quarantine directory. Returns the quarantined path.
    pub fn quarantine(&mut self, source: &str) -> Result<String> {
        let src = Path::new(source);
        if !src.exists() {
            return Err(crate::error::AegisError::Io(format!(
                "source file does not exist: {source}"
            )));
        }

        let q_dir = AppConfig::data_dir().join("quarantine");
        fs::create_dir_all(&q_dir).ok();

        let id = uuid::Uuid::new_v4().simple().to_string();
        let file_name = src
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into());
        let dest = q_dir.join(format!("{id}-{file_name}"));

        // Copy then delete original (more portable than `rename` across filesystems).
        let metadata = fs::metadata(src)?;
        fs::copy(src, &dest)?;
        fs::remove_file(src)?;

        let entry = QuarantineEntry {
            id: id.clone(),
            original_path: source.into(),
            quarantined_path: dest.to_string_lossy().into_owned(),
            quarantined_at_ms: time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000,
            signature_name: None,
            size_bytes: metadata.len(),
        };
        let q_path = entry.quarantined_path.clone();
        self.entries.push(entry);
        Ok(q_path)
    }

    /// Restores a quarantined file by id.
    pub fn restore(&mut self, id: &str) -> Result<()> {
        let idx = self
            .entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| crate::error::AegisError::Security(format!("no quarantine entry {id}")))?;
        let entry = self.entries.remove(idx);
        let dest = PathBuf::from(&entry.original_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::copy(&entry.quarantined_path, &dest)?;
        fs::remove_file(&entry.quarantined_path)?;
        Ok(())
    }

    /// Permanently deletes a quarantined file.
    pub fn delete(&mut self, id: &str) -> Result<()> {
        let idx = self
            .entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| crate::error::AegisError::Security(format!("no quarantine entry {id}")))?;
        let entry = self.entries.remove(idx);
        let _ = fs::remove_file(&entry.quarantined_path);
        Ok(())
    }
}
