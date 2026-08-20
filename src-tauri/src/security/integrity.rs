//! File integrity monitor: computes SHA-256 hashes of critical system files
//! and compares them with stored baselines to detect tampering.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AegisError, Result};

/// An integrity check event (file changed / new / missing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityEvent {
    pub path: String,
    pub event_kind: String, // "changed", "new", "missing"
    pub expected_hash: Option<String>,
    pub actual_hash: Option<String>,
    pub timestamp_ms: u64,
}

/// Baseline hashes stored for comparison.
static BASELINES: LazyLock<parking_lot::Mutex<HashMap<String, String>>> =
    LazyLock::new(|| parking_lot::Mutex::new(HashMap::new()));

/// Returns the list of critical files to monitor based on platform.
pub fn critical_files() -> Vec<String> {
    let mut files = Vec::new();

    #[cfg(unix)]
    {
        if let Some(home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()) {
            files.push(home.join(".bashrc").to_string_lossy().into_owned());
            files.push(home.join(".bash_profile").to_string_lossy().into_owned());
            files.push(home.join(".profile").to_string_lossy().into_owned());
            files.push(
                home.join(".ssh/authorized_keys")
                    .to_string_lossy()
                    .into_owned(),
            );
            files.push(home.join(".ssh/config").to_string_lossy().into_owned());
        }
        files.push("/etc/hosts".into());
        files.push("/etc/passwd".into());
        files.push("/etc/shadow".into());
        files.push("/etc/sudoers".into());
        files.push("/etc/crontab".into());

        // Autostart directory
        if let Some(home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()) {
            let autostart = home.join(".config/autostart");
            if autostart.exists()
                && let Ok(entries) = std::fs::read_dir(&autostart)
            {
                for entry in entries.flatten() {
                    files.push(entry.path().to_string_lossy().into_owned());
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // On Windows, we check key startup locations
        // (registry would require the windows crate, so we check known paths)
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let startup =
                PathBuf::from(&appdata).join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
            if startup.exists() {
                if let Ok(entries) = std::fs::read_dir(&startup) {
                    for entry in entries.flatten() {
                        files.push(entry.path().to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    // Filter to only existing files
    files.retain(|f| PathBuf::from(f).exists());
    files
}

/// Compute SHA-256 hash of a file.
pub fn file_hash(path: &str) -> Result<String> {
    let bytes = std::fs::read(path).map_err(|e| AegisError::Io(format!("reading {path}: {e}")))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

/// Save the current state of critical files as the baseline.
pub fn save_baseline() -> Result<Vec<String>> {
    let files = critical_files();
    let mut baselines = BASELINES.lock();
    baselines.clear();
    let mut saved = Vec::new();
    for path in &files {
        if let Ok(hash) = file_hash(path) {
            baselines.insert(path.clone(), hash);
            saved.push(path.clone());
        }
    }
    tracing::info!("integrity baseline saved: {} files", saved.len());
    Ok(saved)
}

/// Check integrity of critical files against the stored baseline.
/// Returns a list of events for any discrepancies.
pub fn check_integrity() -> Result<Vec<IntegrityEvent>> {
    let files = critical_files();
    let baselines = BASELINES.lock();
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000;
    let mut events = Vec::new();

    for path in &files {
        match file_hash(path) {
            Ok(actual_hash) => {
                match baselines.get(path) {
                    Some(expected_hash) => {
                        if actual_hash != *expected_hash {
                            events.push(IntegrityEvent {
                                path: path.clone(),
                                event_kind: "changed".into(),
                                expected_hash: Some(expected_hash.clone()),
                                actual_hash: Some(actual_hash),
                                timestamp_ms: now_ms,
                            });
                        }
                    }
                    None => {
                        // New file not in baseline
                        events.push(IntegrityEvent {
                            path: path.clone(),
                            event_kind: "new".into(),
                            expected_hash: None,
                            actual_hash: Some(actual_hash),
                            timestamp_ms: now_ms,
                        });
                    }
                }
            }
            Err(_) => {
                // File could not be read — if it was in baseline, it's missing
                if baselines.contains_key(path) {
                    events.push(IntegrityEvent {
                        path: path.clone(),
                        event_kind: "missing".into(),
                        expected_hash: baselines.get(path).cloned(),
                        actual_hash: None,
                        timestamp_ms: now_ms,
                    });
                }
            }
        }
    }

    // Also check for files that were in baseline but no longer in critical_files
    for (path, expected_hash) in baselines.iter() {
        if !files.contains(path) && !PathBuf::from(path).exists() {
            events.push(IntegrityEvent {
                path: path.clone(),
                event_kind: "missing".into(),
                expected_hash: Some(expected_hash.clone()),
                actual_hash: None,
                timestamp_ms: now_ms,
            });
        }
    }

    Ok(events)
}

/// Load baselines from SQLite database (for persistence across restarts).
pub fn load_baselines_from_db(conn: &rusqlite::Connection) -> Result<()> {
    // Create table if needed
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS integrity_baselines (
            path TEXT PRIMARY KEY,
            hash_sha256 TEXT NOT NULL,
            saved_at_ms INTEGER NOT NULL
        );",
    )?;

    let mut stmt = conn.prepare("SELECT path, hash_sha256 FROM integrity_baselines")?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut baselines = BASELINES.lock();
    baselines.clear();
    for (path, hash) in rows.flatten() {
        baselines.insert(path, hash);
    }

    Ok(())
}

/// Save baselines to SQLite database.
pub fn save_baselines_to_db(conn: &rusqlite::Connection) -> Result<()> {
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000;
    let baselines = BASELINES.lock();

    conn.execute("DELETE FROM integrity_baselines", [])?;

    for (path, hash) in baselines.iter() {
        conn.execute(
            "INSERT INTO integrity_baselines (path, hash_sha256, saved_at_ms) VALUES (?1, ?2, ?3)",
            rusqlite::params![path, hash, now_ms as i64],
        )?;
    }

    Ok(())
}
