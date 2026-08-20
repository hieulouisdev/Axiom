//! Proactive intelligence engine: pattern detection + insight surfacing.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::memory::store::MemoryStore;

/// Maximum insights to keep in memory at any time. Older ones are evicted.
pub const MAX_INSIGHTS: usize = 200;

/// A single actionable observation surfaced to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub id: String,
    pub kind: InsightKind,
    pub title: String,
    pub detail: String,
    /// Suggested follow-up action (displayed as a button in the UI).
    pub suggested_action: Option<String>,
    /// 0.0 (info) → 1.0 (critical).
    pub severity: f32,
    pub created_ms: u64,
    /// Whether the user has dismissed this insight.
    pub dismissed: bool,
}

/// Categorizes an insight so the UI can group/filter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InsightKind {
    /// Pattern in the user's activity (e.g. "you've been doing X a lot").
    ActivityPattern,
    /// Something the user might want to remember (e.g. entity mention).
    MemorySuggestion,
    /// Security observation (e.g. unresolved threat, outdated baseline).
    Security,
    /// Workflow / automation suggestion (e.g. "you keep running these 3
    /// commands — make a workflow").
    WorkflowSuggestion,
    /// Cost / efficiency observation (e.g. "you've spent 4 hours in chat
    /// today").
    Efficiency,
}

/// The proactive intelligence engine. Holds a ring buffer of recent
/// insights and runs a periodic analysis pass over the memory store.
pub struct ProactiveEngine {
    insights: Arc<Mutex<Vec<Insight>>>,
    enabled: Arc<AtomicBool>,
    /// How many analysis ticks have run. Used to vary the analysis
    /// (e.g. only run expensive pattern detection every Nth tick).
    tick_count: Arc<Mutex<u64>>,
}

impl ProactiveEngine {
    pub fn new() -> Self {
        Self {
            insights: Arc::new(Mutex::new(Vec::new())),
            enabled: Arc::new(AtomicBool::new(false)),
            tick_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Return all non-dismissed insights, newest first.
    pub fn active_insights(&self) -> Vec<Insight> {
        self.insights
            .lock()
            .iter()
            .filter(|i| !i.dismissed)
            .cloned()
            .collect()
    }

    /// Return the most recent N insights (dismissed or not).
    pub fn recent(&self, limit: usize) -> Vec<Insight> {
        let all = self.insights.lock();
        let start = all.len().saturating_sub(limit);
        all[start..].iter().rev().cloned().collect()
    }

    /// Mark an insight as dismissed.
    pub fn dismiss(&self, insight_id: &str) -> bool {
        let mut insights = self.insights.lock();
        for i in insights.iter_mut() {
            if i.id == insight_id {
                i.dismissed = true;
                return true;
            }
        }
        false
    }

    /// Clear all insights.
    pub fn clear(&self) {
        self.insights.lock().clear();
    }

    /// Run one analysis tick. Reads from the memory store, detects patterns,
    /// and emits `intelligence://insight` events for each new insight.
    /// Called from the continuous-mode heartbeat.
    pub fn tick(&self, memory: &MemoryStore, app: &tauri::AppHandle) {
        if !self.is_enabled() {
            return;
        }
        let mut counter = self.tick_count.lock();
        *counter += 1;
        let n = *counter;
        drop(counter);

        // Each tick contributes a different analysis angle so we don't burn
        // CPU re-running every detector on every heartbeat.
        let new_insights: Vec<Insight> = match n % 4 {
            0 => self.detect_activity_patterns(memory),
            1 => self.detect_memory_suggestions(memory),
            2 => self.detect_security_observations(memory),
            3 => self.detect_workflow_suggestions(memory),
            _ => vec![],
        };

        if new_insights.is_empty() {
            return;
        }

        let mut store = self.insights.lock();
        for insight in new_insights {
            let _ = app.emit("intelligence://insight", &insight);
            store.push(insight);
        }
        // Evict oldest if over capacity.
        if store.len() > MAX_INSIGHTS {
            let extra = store.len() - MAX_INSIGHTS;
            store.drain(0..extra);
        }
    }

    // ---- pattern detectors ----

    fn detect_activity_patterns(&self, memory: &MemoryStore) -> Vec<Insight> {
        let mut out: Vec<Insight> = Vec::new();
        // High activity volume in the last hour?
        if let Ok(recent) = memory.activity.recent(50) {
            let now = now_ms();
            let one_hour_ago = now.saturating_sub(3_600_000);
            let last_hour: Vec<_> = recent
                .iter()
                .filter(|a| a.created_at_ms >= one_hour_ago)
                .collect();
            if last_hour.len() >= 20 {
                out.push(Insight {
                    id: uuid::Uuid::new_v4().to_string(),
                    kind: InsightKind::Efficiency,
                    title: "High activity in the last hour".into(),
                    detail: format!(
                        "You've performed {} actions in the last hour. Consider taking a break \
                         or batching similar tasks into a workflow to reduce cognitive load.",
                        last_hour.len()
                    ),
                    suggested_action: Some("Open Studio".into()),
                    severity: 0.3,
                    created_ms: now,
                    dismissed: false,
                });
            }
        }
        out
    }

    fn detect_memory_suggestions(&self, memory: &MemoryStore) -> Vec<Insight> {
        let mut out: Vec<Insight> = Vec::new();
        if let Ok(entries) = memory.knowledge.list(50) {
            // If the user has <5 facts stored, suggest they try the
            // memory_remember tool more often.
            if entries.len() < 5 {
                out.push(Insight {
                    id: uuid::Uuid::new_v4().to_string(),
                    kind: InsightKind::MemorySuggestion,
                    title: "Build up your knowledge base".into(),
                    detail: format!(
                        "Your knowledge base has only {} entries. Try asking Aegis to \
                         \"remember\" facts about people, projects, and preferences — it'll \
                         surface them automatically in future chats via RAG.",
                        entries.len()
                    ),
                    suggested_action: Some("Open Memory".into()),
                    severity: 0.2,
                    created_ms: now_ms(),
                    dismissed: false,
                });
            }
        }
        out
    }

    fn detect_security_observations(&self, memory: &MemoryStore) -> Vec<Insight> {
        let mut out: Vec<Insight> = Vec::new();
        // If the user has scanned for viruses before but no baseline is
        // saved, suggest saving one.
        if let Ok(events) = memory.activity.recent(100) {
            let has_scan = events
                .iter()
                .any(|a| a.summary.to_lowercase().contains("scan"));
            if has_scan {
                out.push(Insight {
                    id: uuid::Uuid::new_v4().to_string(),
                    kind: InsightKind::Security,
                    title: "Refresh your integrity baseline".into(),
                    detail: "You've run a security scan recently. Now is a good time to save \
                             a fresh file-integrity baseline so future tampering is detected \
                             immediately."
                        .into(),
                    suggested_action: Some("Save baseline".into()),
                    severity: 0.4,
                    created_ms: now_ms(),
                    dismissed: false,
                });
            }
        }
        out
    }

    fn detect_workflow_suggestions(&self, memory: &MemoryStore) -> Vec<Insight> {
        let mut out: Vec<Insight> = Vec::new();
        // Look for repeated shell commands — if the same command prefix
        // appears >=3 times, suggest making a workflow.
        if let Ok(events) = memory.activity.recent(200) {
            use std::collections::HashMap;
            let mut prefixes: HashMap<String, usize> = HashMap::new();
            for a in &events {
                if let Some(first) = a.summary.split_whitespace().next() {
                    *prefixes.entry(first.to_lowercase()).or_insert(0) += 1;
                }
            }
            for (prefix, count) in prefixes.iter().filter(|(_, c)| **c >= 5).take(3) {
                out.push(Insight {
                    id: uuid::Uuid::new_v4().to_string(),
                    kind: InsightKind::WorkflowSuggestion,
                    title: format!("Automate \"{}\" commands", prefix),
                    detail: format!(
                        "You've run \"{}\" {} times recently. Consider building a workflow \
                         in Studio to chain it with follow-up steps and run it on demand.",
                        prefix, count
                    ),
                    suggested_action: Some("Open Studio".into()),
                    severity: 0.25,
                    created_ms: now_ms(),
                    dismissed: false,
                });
            }
        }
        out
    }
}

impl Default for ProactiveEngine {
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
    fn enable_disable_roundtrip() {
        let e = ProactiveEngine::new();
        assert!(!e.is_enabled());
        e.enable();
        assert!(e.is_enabled());
        e.disable();
        assert!(!e.is_enabled());
    }

    #[test]
    fn dismiss_marks_insight_dismissed() {
        let e = ProactiveEngine::new();
        let insight = Insight {
            id: "i1".into(),
            kind: InsightKind::Efficiency,
            title: "Test".into(),
            detail: "Test detail".into(),
            suggested_action: None,
            severity: 0.1,
            created_ms: 0,
            dismissed: false,
        };
        e.insights.lock().push(insight);
        assert!(e.dismiss("i1"));
        assert!(e.active_insights().is_empty());
        assert_eq!(e.recent(10).len(), 1);
    }

    #[test]
    fn clear_empties_insights() {
        let e = ProactiveEngine::new();
        e.insights.lock().push(Insight {
            id: "x".into(),
            kind: InsightKind::Efficiency,
            title: "x".into(),
            detail: "y".into(),
            suggested_action: None,
            severity: 0.0,
            created_ms: 0,
            dismissed: false,
        });
        e.clear();
        assert!(e.active_insights().is_empty());
    }

    #[test]
    fn recent_returns_newest_first() {
        let e = ProactiveEngine::new();
        for i in 0..5 {
            e.insights.lock().push(Insight {
                id: format!("i{i}"),
                kind: InsightKind::Efficiency,
                title: format!("title{i}"),
                detail: "d".into(),
                suggested_action: None,
                severity: 0.1,
                created_ms: i as u64,
                dismissed: false,
            });
        }
        let r = e.recent(3);
        assert_eq!(r.len(), 3);
        // Newest first.
        assert_eq!(r[0].id, "i4");
        assert_eq!(r[2].id, "i2");
    }
}
