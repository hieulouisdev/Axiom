//! File system watcher for proactive AI analysis.
//!
//! Watches configured directories for changes and emits events
//! that the AI can analyze proactively.

use std::path::PathBuf;

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
static RECENT_EVENTS: LazyLock<Mutex<Vec<WatchEvent>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Returns recent file system watch events.
pub fn recent_events() -> Vec<WatchEvent> {
    RECENT_EVENTS.lock().clone()
}

/// Start watching directories for changes.
/// This spawns a background task that uses the `notify` crate.
pub async fn start_watching(config: WatchConfig, app_handle: tauri::AppHandle) -> Result<()> {
    // v0.3: notify 6.1.1 requires the `Watcher` trait import to call
    // `watcher.watch(...)` and the `EventHandler` trait to pass an mpsc::Sender
    // directly to `Watcher::new`. We use the callback-based API instead.
    use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc;

    if config.watch_paths.is_empty() {
        tracing::info!("file watcher: no paths configured, skipping");
        return Ok(());
    }

    // Build a watcher that funnels events into an mpsc channel via a closure.
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res: notify::Result<Event>| {
            let _ = tx.send(res);
        })
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

    // Spawn a task to process events.
    tokio::spawn(async move {
        // Pin the watcher so it lives for the duration of the task.
        let _watcher = watcher;
        loop {
            match rx.try_recv() {
                Ok(Ok(event)) => {
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
                            timestamp_ms: time::OffsetDateTime::now_utc().unix_timestamp() as u64
                                * 1000,
                        };

                        // Store recent events
                        {
                            let mut events = RECENT_EVENTS.lock();
                            events.push(watch_event.clone());
                            events.truncate(200);
                        }

                        // Emit to frontend
                        // v0.3 fix: `Emitter` trait must be in scope to call `emit`.
                        use tauri::Emitter as _;
                        let _ = app_handle.emit("watcher://change", &watch_event);
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("file watcher error: {e}");
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
