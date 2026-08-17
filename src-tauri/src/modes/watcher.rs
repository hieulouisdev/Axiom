//! File system watcher for proactive AI analysis.
//!
//! Watches configured directories for changes and emits events
//! that the AI can analyze proactively.

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// A file system change event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEvent {
    pub kind: String,
    pub path: String,
    pub timestamp_ms: u64,
}

/// Configuration for the file watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Directories to watch.
    pub watch_paths: Vec<String>,
    /// File patterns to ignore (glob patterns).
    pub ignore_patterns: Vec<String>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            watch_paths: Vec::new(),
            ignore_patterns: vec![
                "*.tmp".into(),
                "*.log".into(),
                ".git/*".into(),
                "node_modules/*".into(),
                "target/*".into(),
            ],
        }
    }
}

/// Recent file system events.
static RECENT_EVENTS: once_cell::sync::Lazy<Mutex<Vec<WatchEvent>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(Vec::new()));

/// Returns recent file system watch events.
pub fn recent_events() -> Vec<WatchEvent> {
    RECENT_EVENTS.lock().clone()
}

/// Start watching directories for changes.
/// This spawns a background task that uses the `notify` crate.
pub async fn start_watching(
    config: WatchConfig,
    app_handle: tauri::AppHandle,
) -> Result<()> {
    use notify::{RecommendedWatcher, RecursiveMode, Event, EventKind};
    use std::sync::mpsc;

    if config.watch_paths.is_empty() {
        tracing::info!("file watcher: no paths configured, skipping");
        return Ok(());
    }

    let (tx, rx) = mpsc::channel::<Event>();

    let mut watcher: RecommendedWatcher =
        notify::Watcher::new(tx, std::time::Duration::from_millis(200))
            .map_err(|e| crate::error::AegisError::Internal(format!("watcher init: {e}")))?;

    for path in &config.watch_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            if let Err(e) = watcher.watch(&p, RecursiveMode::Recursive) {
                tracing::warn!("failed to watch {}: {e}", path);
            } else {
                tracing::info!("watching directory: {}", path);
            }
        } else {
            tracing::warn!("watch path does not exist: {}", path);
        }
    }

    // Spawn a task to process events
    tokio::spawn(async move {
        loop {
            // Check for events without blocking the async runtime
            match rx.try_recv() {
                Ok(event) => {
                    let kind_str = match event.kind {
                        EventKind::Create(_) => "create",
                        EventKind::Modify(_) => "modify",
                        EventKind::Remove(_) => "remove",
                        EventKind::Access(_) => "access",
                        EventKind::Other => "other",
                        _ => "unknown",
                    };

                    for path in &event.paths {
                        let watch_event = WatchEvent {
                            kind: kind_str.to_string(),
                            path: path.to_string_lossy().to_string(),
                            timestamp_ms: time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000,
                        };

                        // Store recent events
                        {
                            let mut events = RECENT_EVENTS.lock();
                            events.push(watch_event.clone());
                            events.truncate(200);
                        }

                        // Emit to frontend
                        let _ = app_handle.emit("watcher://change", &watch_event);
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    tracing::info!("file watcher channel disconnected");
                    break;
                }
            }
        }
    });

    Ok(())
}
