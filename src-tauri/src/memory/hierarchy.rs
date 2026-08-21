//! v1.7.0 — Hierarchical Memory (inspired by TencentDB Agent Memory).
//!
//! Raw chat logs are distilled layer-by-layer into increasingly compact,
//! durable representations:
//!
//! ```text
//! L0 Conversation  ─ verbatim chat messages (already in `memory::conversation`)
//!        ↓ extract
//! L1 Atom           ─ a single distilled fact / preference / decision
//!        ↓ cluster
//! L2 Scenario       ─ a themed cluster of atoms ("project X", "auth module")
//!        ↓ abstract
//! L3 Persona        ─ long-term user traits ("prefers concise answers",
//!                     "codes in Rust", "reviewer mindset")
//! ```
//!
//! Each layer is persisted in its own SQLite table and contributes a
//! differently-weighted fragment to the agent system prompt:
//!
//! | Layer | TTL        | Prompt weight | Source                       |
//! |-------|------------|---------------|------------------------------|
//! | L0    | 90 days    | last 8 turns  | `conversations` table        |
//! | L1    | 365 days   | top-K by recency | `memory_atoms`           |
//! | L2    | ∞          | all           | `memory_scenarios`           |
//! | L3    | ∞          | all           | `memory_persona`             |
//!
//! The distillation pipeline is intentionally LLM-optional: when no
//! provider is available, atoms are still extracted via a deterministic
//! regex/keyword pass (emails, URLs, code blocks, decisions marked by
//! "remember:"/"let's"/"from now on"). When a provider IS available the
//! orchestrator can call `distill_with_llm()` to refine.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::store::SharedConn;

// ────────────────────────────────────────────────────────────────────────────
// L1 — Atom
// ────────────────────────────────────────────────────────────────────────────

/// A single distilled memory atom.
///
/// Atoms are the smallest unit of memory: one fact, one preference, one
/// decision. They are extracted from conversations (L0) and clustered
/// into scenarios (L2) when themes emerge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAtom {
    pub id: i64,
    pub kind: AtomKind,
    /// Short human-readable summary, e.g. "user prefers Rust over C++".
    pub summary: String,
    /// Optional verbatim source quote.
    pub source_quote: Option<String>,
    /// Conversation id the atom was extracted from, if any.
    pub source_conversation_id: Option<String>,
    /// Scenario id this atom belongs to, once clustered.
    pub scenario_id: Option<i64>,
    pub confidence: f64,
    pub created_at_ms: i64,
    pub last_recalled_ms: i64,
    pub recall_count: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AtomKind {
    /// A user preference ("I like dark mode").
    Preference,
    /// A durable fact about the world or user ("user lives in Hanoi").
    Fact,
    /// A decision the user made ("we'll use Postgres not MySQL").
    Decision,
    /// A standing instruction ("always run clippy before commit").
    Instruction,
    /// A goal or intent ("ship v1.7 by August").
    Goal,
    /// A piece of context the AI should know but isn't durable.
    Context,
}

impl AtomKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AtomKind::Preference => "preference",
            AtomKind::Fact => "fact",
            AtomKind::Decision => "decision",
            AtomKind::Instruction => "instruction",
            AtomKind::Goal => "goal",
            AtomKind::Context => "context",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "preference" => AtomKind::Preference,
            "fact" => AtomKind::Fact,
            "decision" => AtomKind::Decision,
            "instruction" => AtomKind::Instruction,
            "goal" => AtomKind::Goal,
            "context" => AtomKind::Context,
            _ => return None,
        })
    }
}

// ────────────────────────────────────────────────────────────────────────────
// L2 — Scenario
// ────────────────────────────────────────────────────────────────────────────

/// A themed cluster of atoms.
///
/// Scenarios group atoms that share a topic, project, or temporal arc.
/// For example: "Rust migration Q3", "auth-module refactor", "Vietnamese
/// i18n". An atom belongs to at most one scenario; unassigned atoms are
/// shown under the "default" scenario bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: i64,
    pub title: String,
    pub summary: Option<String>,
    /// Optional tags for cross-scenario linking (CSV in DB).
    pub tags: Vec<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub atom_count: i64,
}

// ────────────────────────────────────────────────────────────────────────────
// L3 — Persona
// ────────────────────────────────────────────────────────────────────────────

/// Long-term user traits. There is exactly one persona row per user
/// (identified by `user_id`, default `"default"`).
///
/// Persona entries are intentionally a free-form key/value map so the
/// agent can store arbitrary durable traits ("prefers terse answers",
/// "writes Rust + TypeScript", "uses Neovim", "timezone Asia/Saigon").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub user_id: String,
    pub traits: Vec<PersonaTrait>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaTrait {
    pub key: String,
    pub value: String,
    pub confidence: f64,
    pub source: Option<String>,
    pub updated_at_ms: i64,
}

// ────────────────────────────────────────────────────────────────────────────
// Store
// ────────────────────────────────────────────────────────────────────────────

/// Hierarchical memory store.
///
/// All layers share the [`SharedConn`] used by the rest of the memory
/// module so a single SQLite transaction can span L0→L3.
pub struct HierarchicalMemory {
    conn: SharedConn,
}

impl HierarchicalMemory {
    pub fn new(conn: SharedConn) -> Self {
        Self { conn }
    }

    /// Run schema migrations for the hierarchical tables.
    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memory_atoms (
                id                     INTEGER PRIMARY KEY AUTOINCREMENT,
                kind                   TEXT NOT NULL,
                summary                TEXT NOT NULL,
                source_quote           TEXT,
                source_conversation_id TEXT,
                scenario_id            INTEGER,
                confidence             REAL NOT NULL DEFAULT 0.6,
                created_at_ms          INTEGER NOT NULL,
                last_recalled_ms       INTEGER NOT NULL,
                recall_count           INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (scenario_id) REFERENCES memory_scenarios(id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_atoms_kind     ON memory_atoms(kind);
            CREATE INDEX IF NOT EXISTS idx_atoms_scenario ON memory_atoms(scenario_id);
            CREATE INDEX IF NOT EXISTS idx_atoms_recent   ON memory_atoms(last_recalled_ms DESC);

            CREATE TABLE IF NOT EXISTS memory_scenarios (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                title        TEXT NOT NULL UNIQUE,
                summary      TEXT,
                tags         TEXT,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS memory_persona (
                user_id      TEXT NOT NULL,
                key          TEXT NOT NULL,
                value        TEXT NOT NULL,
                confidence   REAL NOT NULL DEFAULT 0.6,
                source       TEXT,
                updated_at_ms INTEGER NOT NULL,
                PRIMARY KEY (user_id, key)
            );
            "#,
        )?;
        Ok(())
    }

    // ─── L1 Atom ops ──────────────────────────────────────────────────

    pub fn add_atom(
        &self,
        kind: AtomKind,
        summary: &str,
        source_quote: Option<&str>,
        source_conversation_id: Option<&str>,
        scenario_id: Option<i64>,
        confidence: f64,
    ) -> Result<i64> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO memory_atoms
                 (kind, summary, source_quote, source_conversation_id, scenario_id, confidence,
                  created_at_ms, last_recalled_ms, recall_count)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, 0)"#,
            params![
                kind.as_str(),
                summary,
                source_quote,
                source_conversation_id,
                scenario_id,
                confidence,
                now as i64,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_atoms(&self, limit: i64) -> Result<Vec<MemoryAtom>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT id, kind, summary, source_quote, source_conversation_id,
                      scenario_id, confidence, created_at_ms, last_recalled_ms, recall_count
                 FROM memory_atoms
                ORDER BY created_at_ms DESC
                LIMIT ?1"#,
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            let kind_str: String = row.get(1)?;
            let kind = AtomKind::parse(&kind_str).unwrap_or(AtomKind::Context);
            Ok(MemoryAtom {
                id: row.get(0)?,
                kind,
                summary: row.get(2)?,
                source_quote: row.get(3)?,
                source_conversation_id: row.get(4)?,
                scenario_id: row.get(5)?,
                confidence: row.get(6)?,
                created_at_ms: row.get(7)?,
                last_recalled_ms: row.get(8)?,
                recall_count: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn atoms_for_scenario(&self, scenario_id: i64) -> Result<Vec<MemoryAtom>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT id, kind, summary, source_quote, source_conversation_id,
                      scenario_id, confidence, created_at_ms, last_recalled_ms, recall_count
                 FROM memory_atoms
                WHERE scenario_id = ?1
                ORDER BY created_at_ms ASC"#,
        )?;
        let rows = stmt.query_map(params![scenario_id], |row| {
            let kind_str: String = row.get(1)?;
            let kind = AtomKind::parse(&kind_str).unwrap_or(AtomKind::Context);
            Ok(MemoryAtom {
                id: row.get(0)?,
                kind,
                summary: row.get(2)?,
                source_quote: row.get(3)?,
                source_conversation_id: row.get(4)?,
                scenario_id: row.get(5)?,
                confidence: row.get(6)?,
                created_at_ms: row.get(7)?,
                last_recalled_ms: row.get(8)?,
                recall_count: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn forget_atom(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM memory_atoms WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn touch_atom(&self, id: i64) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let conn = self.conn.lock();
        conn.execute(
            r#"UPDATE memory_atoms
                  SET last_recalled_ms = ?1,
                      recall_count = recall_count + 1
                WHERE id = ?2"#,
            params![now as i64, id],
        )?;
        Ok(())
    }

    // ─── L2 Scenario ops ──────────────────────────────────────────────

    pub fn upsert_scenario(&self, title: &str, summary: Option<&str>, tags: &[String]) -> Result<i64> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let tags_csv = tags.join(",");
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO memory_scenarios (title, summary, tags, created_at_ms, updated_at_ms)
               VALUES (?1, ?2, ?3, ?4, ?4)
               ON CONFLICT(title) DO UPDATE SET
                   summary  = COALESCE(excluded.summary, memory_scenarios.summary),
                   tags     = COALESCE(excluded.tags,    memory_scenarios.tags),
                   updated_at_ms = excluded.updated_at_ms"#,
            params![title, summary, tags_csv, now as i64],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_scenarios(&self) -> Result<Vec<Scenario>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT s.id, s.title, s.summary, s.tags, s.created_at_ms, s.updated_at_ms,
                      (SELECT COUNT(*) FROM memory_atoms a WHERE a.scenario_id = s.id) AS atom_count
                 FROM memory_scenarios s
                ORDER BY s.updated_at_ms DESC"#,
        )?;
        let rows = stmt.query_map([], |row| {
            let tags_csv: Option<String> = row.get(3)?;
            let tags = tags_csv
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            Ok(Scenario {
                id: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                tags,
                created_at_ms: row.get(4)?,
                updated_at_ms: row.get(5)?,
                atom_count: row.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn delete_scenario(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        // detach atoms first so they survive as ungrouped
        conn.execute(
            "UPDATE memory_atoms SET scenario_id = NULL WHERE scenario_id = ?1",
            params![id],
        )?;
        conn.execute("DELETE FROM memory_scenarios WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn assign_atom_to_scenario(&self, atom_id: i64, scenario_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE memory_atoms SET scenario_id = ?1 WHERE id = ?2",
            params![scenario_id, atom_id],
        )?;
        Ok(())
    }

    // ─── L3 Persona ops ──────────────────────────────────────────────

    pub fn set_persona_trait(
        &self,
        user_id: &str,
        key: &str,
        value: &str,
        confidence: f64,
        source: Option<&str>,
    ) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO memory_persona (user_id, key, value, confidence, source, updated_at_ms)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)
               ON CONFLICT(user_id, key) DO UPDATE SET
                   value      = excluded.value,
                   confidence = excluded.confidence,
                   source     = excluded.source,
                   updated_at_ms = excluded.updated_at_ms"#,
            params![user_id, key, value, confidence, source, now as i64],
        )?;
        Ok(())
    }

    pub fn forget_persona_trait(&self, user_id: &str, key: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM memory_persona WHERE user_id = ?1 AND key = ?2",
            params![user_id, key],
        )?;
        Ok(())
    }

    pub fn load_persona(&self, user_id: &str) -> Result<Persona> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT key, value, confidence, source, updated_at_ms
                 FROM memory_persona
                WHERE user_id = ?1
                ORDER BY key ASC"#,
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(PersonaTrait {
                key: row.get(0)?,
                value: row.get(1)?,
                confidence: row.get(2)?,
                source: row.get(3)?,
                updated_at_ms: row.get(4)?,
            })
        })?;
        let mut traits = Vec::new();
        for r in rows {
            traits.push(r?);
        }
        let updated_at_ms = traits.iter().map(|t| t.updated_at_ms).max().unwrap_or(0);
        Ok(Persona {
            user_id: user_id.to_string(),
            traits,
            updated_at_ms,
        })
    }

    // ─── Prompt rendering ────────────────────────────────────────────

    /// Build a system-prompt fragment that summarizes the user's durable
    /// memory: persona traits, all scenarios, and the K most-recent atoms.
    pub fn render_prompt_fragment(&self, user_id: &str, recent_atom_limit: i64) -> Result<String> {
        let persona = self.load_persona(user_id)?;
        let scenarios = self.list_scenarios()?;
        let atoms = self.list_atoms(recent_atom_limit)?;
        let mut out = String::new();
        if !persona.traits.is_empty() {
            out.push_str("## Long-term persona\n");
            for t in &persona.traits {
                out.push_str(&format!("- **{}**: {} (conf={:.2})\n", t.key, t.value, t.confidence));
            }
            out.push('\n');
        }
        if !scenarios.is_empty() {
            out.push_str("## Active scenarios\n");
            for s in &scenarios {
                let tag_str = if s.tags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", s.tags.join(", "))
                };
                out.push_str(&format!(
                    "- **{}**{} — {} atoms — updated {}\n",
                    s.title,
                    tag_str,
                    s.atom_count,
                    s.updated_at_ms
                ));
            }
            out.push('\n');
        }
        if !atoms.is_empty() {
            out.push_str("## Recent memory atoms\n");
            for a in &atoms {
                out.push_str(&format!(
                    "- ({}) {} — conf={:.2}\n",
                    a.kind.as_str(),
                    a.summary,
                    a.confidence
                ));
            }
        }
        Ok(out)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Deterministic extraction (LLM-optional)
// ────────────────────────────────────────────────────────────────────────────

/// Lightweight deterministic extractor that pulls atoms out of a chat
/// message without needing an LLM round-trip. Looks for:
/// - decision markers: "let's", "we should", "from now on", "remember:"
/// - preferences: "I prefer", "I like", "I hate", "always", "never"
/// - goals: "I want to", "the goal is", "we need to ship"
///
/// Returns a list of (kind, summary) pairs ready to insert as atoms.
pub fn deterministic_extract(message: &str) -> Vec<(AtomKind, String)> {
    let mut out = Vec::new();
    let lower = message.to_lowercase();
    let lines = message.lines();

    let decision_markers = ["let's", "lets", "we should", "we'll", "from now on", "remember:"];
    let pref_markers = ["i prefer", "i like", "i hate", "i love", "always use", "never use"];
    let goal_markers = ["i want to", "the goal is", "we need to ship", "i'm trying to"];

    for line in lines {
        let line_lower = line.to_lowercase();
        if decision_markers.iter().any(|m| line_lower.contains(m)) {
            let trimmed = line.trim();
            if trimmed.len() > 8 && trimmed.len() < 240 {
                out.push((AtomKind::Decision, trimmed.to_string()));
            }
        } else if pref_markers.iter().any(|m| line_lower.contains(m)) {
            let trimmed = line.trim();
            if trimmed.len() > 8 && trimmed.len() < 240 {
                out.push((AtomKind::Preference, trimmed.to_string()));
            }
        } else if goal_markers.iter().any(|m| line_lower.contains(m)) {
            let trimmed = line.trim();
            if trimmed.len() > 8 && trimmed.len() < 240 {
                out.push((AtomKind::Goal, trimmed.to_string()));
            }
        }
    }
    // de-dup
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out.dedup_by(|a, b| a.1 == b.1);
    // also surface a fact if message contains an email (very common case)
    if lower.contains('@') {
        for word in message.split_whitespace() {
            if word.contains('@') && word.contains('.') && word.len() < 80 {
                out.push((AtomKind::Fact, format!("contact: {}", word.trim_matches(|c: char| !c.is_alphanumeric() && c != '@' && c != '.'))));
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open() -> HierarchicalMemory {
        let conn = Arc::new(Mutex::new(rusqlite::Connection::open_in_memory().unwrap()));
        let h = HierarchicalMemory::new(conn);
        h.migrate().unwrap();
        h
    }

    #[test]
    fn atom_roundtrip() {
        let h = open();
        let id = h
            .add_atom(
                AtomKind::Preference,
                "prefers dark mode",
                Some("I prefer dark mode"),
                None,
                None,
                0.8,
            )
            .unwrap();
        let atoms = h.list_atoms(10).unwrap();
        assert_eq!(atoms.len(), 1);
        assert_eq!(atoms[0].id, id);
        assert_eq!(atoms[0].kind, AtomKind::Preference);
        h.touch_atom(id).unwrap();
        let atoms2 = h.list_atoms(10).unwrap();
        assert_eq!(atoms2[0].recall_count, 1);
    }

    #[test]
    fn scenario_assign() {
        let h = open();
        let sid = h.upsert_scenario("Rust migration", Some("Q3 effort"), &["rust".into()]).unwrap();
        let aid = h
            .add_atom(AtomKind::Decision, "use Tokio for runtime", None, None, None, 0.7)
            .unwrap();
        h.assign_atom_to_scenario(aid, sid).unwrap();
        let atoms = h.atoms_for_scenario(sid).unwrap();
        assert_eq!(atoms.len(), 1);
        let scens = h.list_scenarios().unwrap();
        assert_eq!(scens[0].atom_count, 1);
    }

    #[test]
    fn persona_upsert() {
        let h = open();
        h.set_persona_trait("default", "language", "Vietnamese", 0.9, Some("chat")).unwrap();
        h.set_persona_trait("default", "language", "English", 0.95, Some("correction")).unwrap();
        let p = h.load_persona("default").unwrap();
        assert_eq!(p.traits.len(), 1);
        assert_eq!(p.traits[0].value, "English");
        assert!((p.traits[0].confidence - 0.95).abs() < 1e-6);
    }

    #[test]
    fn deterministic_extractor_finds_decisions() {
        let msg = "let's switch to Rust for the next service. I prefer vim. The goal is to ship by August.";
        let atoms = deterministic_extract(msg);
        assert!(atoms.iter().any(|(_, s)| s.contains("let's switch to Rust")));
        assert!(atoms.iter().any(|(_, s)| s.contains("I prefer vim")));
        assert!(atoms.iter().any(|(_, s)| s.contains("The goal is")));
    }

    #[test]
    fn prompt_fragment_renders() {
        let h = open();
        h.set_persona_trait("default", "tz", "Asia/Saigon", 1.0, None).unwrap();
        h.upsert_scenario("Project A", None, &[]).unwrap();
        h.add_atom(AtomKind::Fact, "user lives in Hanoi", None, None, None, 0.6).unwrap();
        let f = h.render_prompt_fragment("default", 10).unwrap();
        assert!(f.contains("Long-term persona"));
        assert!(f.contains("Active scenarios"));
        assert!(f.contains("Recent memory atoms"));
    }
}

use std::sync::Arc;
