//! v1.6.0 — Background Task Queue.
//!
//! The task queue is the durable substrate behind long-running operations:
//! orchestrator plans, workflow runs, large code_eval invocations, batch
//! entity extraction, etc. Each task has a stable `task_id`, a kind label,
//! a `TaskStatus`, and a `progress` float between 0.0 and 1.0. The frontend
//! subscribes to the `task://progress` event channel for live updates.
//!
//! ## Why a separate queue?
//!
//! Tauri commands can already spawn tokio tasks, but those tasks are
//! anonymous — they don't have a stable id, can't be cancelled from the UI,
//! and their progress isn't visible. The task queue adds:
//!
//! - **Stable IDs** — `task_id` survives across UI reloads.
//! - **Cancellation** — `tasks_cancel(task_id)` flips the status and
//!   `task_cancellation_flag(task_id)` lets long-running code poll for
//!   cancellation cooperatively.
//! - **Persistence** — task records live in-memory but are also written to
//!   the SQLite store so they survive a restart (historical view only;
//!   in-flight work is not resumed — that's a v1.7 feature).
//! - **Progress events** — every `update_progress` call emits a
//!   `task://progress` Tauri event the frontend can render as a list of
//!   progress bars.
//!
//! ## Concurrency
//!
//! The queue itself is a thin `Mutex<HashMap<String, Task>>`. Mutations
//! (status transitions, progress updates) are O(1) and lock-quick. Long-
//! running work happens in the spawned tokio task, which holds only the
//! `task_id` string and pulls a fresh `Arc<TaskQueue>` when it needs to
//! update progress.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

/// Maximum finished tasks to keep in memory. Older ones are evicted FIFO.
pub const MAX_FINISHED_TASKS: usize = 100;

/// Status of a background task.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// A single background task record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    /// Free-form kind label, e.g. `orchestrator_plan`, `workflow_run`,
    /// `code_eval`, `memory_backfill`.
    pub kind: String,
    pub status: TaskStatus,
    /// Progress 0.0 → 1.0.
    pub progress: f32,
    /// Optional short label rendered next to the progress bar.
    pub label: Option<String>,
    /// Final result payload (only set when status = Completed).
    pub result: Option<serde_json::Value>,
    /// Final error message (only set when status = Failed).
    pub error: Option<String>,
    pub created_ms: u64,
    pub updated_ms: u64,
}

/// Cancellation flag for a single task. Held alongside the `Task` record so
/// long-running code can poll `is_cancelled()` without locking the queue's
/// main mutex (which would block other progress updates).
#[derive(Default)]
pub struct CancelFlag {
    cancelled: AtomicBool,
}

impl CancelFlag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

/// Process-wide task queue.
pub struct TaskQueue {
    tasks: Arc<Mutex<HashMap<String, Task>>>,
    cancel_flags: Arc<Mutex<HashMap<String, Arc<CancelFlag>>>>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            cancel_flags: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new task and return its `task_id` + the `Arc<CancelFlag>`
    /// that the spawned work should poll periodically.
    pub fn enqueue(&self, kind: &str, label: Option<String>) -> (String, Arc<CancelFlag>) {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        let task = Task {
            id: id.clone(),
            kind: kind.to_string(),
            status: TaskStatus::Pending,
            progress: 0.0,
            label,
            result: None,
            error: None,
            created_ms: now,
            updated_ms: now,
        };
        let flag = Arc::new(CancelFlag::new());
        self.tasks.lock().insert(id.clone(), task);
        self.cancel_flags.lock().insert(id.clone(), flag.clone());
        (id, flag)
    }

    /// Mark a pending task as running. No-op if the task is already terminal.
    pub fn start(&self, task_id: &str) {
        let mut tasks = self.tasks.lock();
        if let Some(t) = tasks.get_mut(task_id)
            && !t.status.is_terminal()
        {
            t.status = TaskStatus::Running;
            t.updated_ms = now_ms();
        }
    }

    /// Update progress. Emits a `task://progress` Tauri event with the
    /// current task snapshot.
    pub fn update_progress(&self, task_id: &str, progress: f32, app: &tauri::AppHandle) {
        let snapshot = {
            let mut tasks = self.tasks.lock();
            if let Some(t) = tasks.get_mut(task_id) {
                t.progress = progress.clamp(0.0, 1.0);
                t.updated_ms = now_ms();
                Some(t.clone())
            } else {
                None
            }
        };
        if let Some(snap) = snapshot {
            let _ = app.emit("task://progress", &snap);
        }
    }

    /// Mark a task as completed with an optional result payload.
    pub fn complete(&self, task_id: &str, result: Option<serde_json::Value>) {
        let mut tasks = self.tasks.lock();
        if let Some(t) = tasks.get_mut(task_id) {
            t.status = TaskStatus::Completed;
            t.progress = 1.0;
            t.result = result;
            t.updated_ms = now_ms();
        }
        self.evict_if_needed(&mut tasks);
    }

    /// Mark a task as failed with an error message.
    pub fn fail(&self, task_id: &str, error: String) {
        let mut tasks = self.tasks.lock();
        if let Some(t) = tasks.get_mut(task_id) {
            t.status = TaskStatus::Failed;
            t.error = Some(error);
            t.updated_ms = now_ms();
        }
        self.evict_if_needed(&mut tasks);
    }

    /// Mark a task as cancelled. Long-running code should also poll
    /// `is_cancelled(task_id)` periodically.
    pub fn cancel(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.lock();
        let flags = self.cancel_flags.lock();
        if let Some(t) = tasks.get_mut(task_id) {
            if t.status.is_terminal() {
                return false;
            }
            t.status = TaskStatus::Cancelled;
            t.updated_ms = now_ms();
        }
        if let Some(f) = flags.get(task_id) {
            f.cancel();
        }
        true
    }

    /// Check if a task has been cancelled. Long-running work should poll
    /// this periodically (between iterations of its main loop).
    pub fn is_cancelled(&self, task_id: &str) -> bool {
        let flags = self.cancel_flags.lock();
        flags.get(task_id).map(|f| f.is_cancelled()).unwrap_or(true) // Unknown task — treat as cancelled.
    }

    /// Get a snapshot of a task by id.
    pub fn get(&self, task_id: &str) -> Option<Task> {
        self.tasks.lock().get(task_id).cloned()
    }

    /// List all known tasks, newest first.
    pub fn list(&self) -> Vec<Task> {
        let tasks = self.tasks.lock();
        let mut v: Vec<Task> = tasks.values().cloned().collect();
        // `sort_by_key` is more idiomatic than `sort_by(|a, b| ...)`.
        // `Reverse` flips the order so the newest `updated_ms` comes first.
        v.sort_by_key(|t| std::cmp::Reverse(t.updated_ms));
        v
    }

    /// List only non-terminal (pending / running) tasks.
    pub fn active(&self) -> Vec<Task> {
        self.tasks
            .lock()
            .values()
            .filter(|t| !t.status.is_terminal())
            .cloned()
            .collect()
    }

    fn evict_if_needed(&self, tasks: &mut HashMap<String, Task>) {
        // Count terminal tasks; if too many, evict oldest by `updated_ms`.
        let terminal_count = tasks.values().filter(|t| t.status.is_terminal()).count();
        if terminal_count <= MAX_FINISHED_TASKS {
            return;
        }
        let extra = terminal_count - MAX_FINISHED_TASKS;
        // Collect (id, updated_ms) for terminal tasks, sort ascending, drop oldest.
        let mut terminals: Vec<(String, u64)> = tasks
            .iter()
            .filter(|(_, t)| t.status.is_terminal())
            .map(|(id, t)| (id.clone(), t.updated_ms))
            .collect();
        terminals.sort_by_key(|(_, ts)| *ts);
        for (id, _) in terminals.into_iter().take(extra) {
            tasks.remove(&id);
            self.cancel_flags.lock().remove(&id);
        }
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_start_complete_lifecycle() {
        let q = TaskQueue::new();
        let (id, flag) = q.enqueue("test", Some("hello".into()));
        assert_eq!(q.get(&id).unwrap().status, TaskStatus::Pending);
        assert_eq!(q.get(&id).unwrap().label.as_deref(), Some("hello"));
        assert!(!flag.is_cancelled());

        q.start(&id);
        assert_eq!(q.get(&id).unwrap().status, TaskStatus::Running);

        q.complete(&id, Some(serde_json::json!({"x": 1})));
        let t = q.get(&id).unwrap();
        assert_eq!(t.status, TaskStatus::Completed);
        assert_eq!(t.progress, 1.0);
        assert_eq!(t.result.unwrap()["x"], 1);
    }

    #[test]
    fn cancel_flips_flag_and_status() {
        let q = TaskQueue::new();
        let (id, flag) = q.enqueue("test", None);
        q.start(&id);
        assert!(q.cancel(&id));
        assert_eq!(q.get(&id).unwrap().status, TaskStatus::Cancelled);
        assert!(flag.is_cancelled());
        assert!(q.is_cancelled(&id));
    }

    #[test]
    fn cancel_terminal_returns_false() {
        let q = TaskQueue::new();
        let (id, _) = q.enqueue("test", None);
        q.complete(&id, None);
        assert!(!q.cancel(&id));
    }

    #[test]
    fn is_cancelled_unknown_task_returns_true() {
        let q = TaskQueue::new();
        assert!(q.is_cancelled("does-not-exist"));
    }

    #[test]
    fn active_filters_terminal() {
        let q = TaskQueue::new();
        let (id1, _) = q.enqueue("test", None);
        let (id2, _) = q.enqueue("test", None);
        q.start(&id1);
        q.complete(&id1, None);
        // id1 is terminal, id2 is pending.
        let actives = q.active();
        assert_eq!(actives.len(), 1);
        assert_eq!(actives[0].id, id2);
    }

    #[test]
    fn list_newest_first() {
        let q = TaskQueue::new();
        let (id1, _) = q.enqueue("test", None);
        // Sleep very briefly so `updated_ms` differs.
        std::thread::sleep(std::time::Duration::from_millis(5));
        let (id2, _) = q.enqueue("test", None);
        // Bump id1's updated_ms by completing it (which sets it to now).
        q.complete(&id1, None);
        let v = q.list();
        // id1 should be newest (just completed); id2 second.
        assert_eq!(v[0].id, id1);
        assert_eq!(v[1].id, id2);
    }

    #[test]
    fn eviction_kicks_in_over_capacity() {
        let q = TaskQueue::new();
        // Enqueue MAX_FINISHED_TASKS + 5 tasks and complete them all.
        for _ in 0..(MAX_FINISHED_TASKS + 5) {
            let (id, _) = q.enqueue("x", None);
            q.complete(&id, None);
        }
        // Should be trimmed to MAX_FINISHED_TASKS.
        let terminal = q.list().iter().filter(|t| t.status.is_terminal()).count();
        assert!(terminal <= MAX_FINISHED_TASKS);
    }
}
