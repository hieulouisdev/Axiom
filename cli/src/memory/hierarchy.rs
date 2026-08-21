//! v1.7 — Hierarchical memory (L0→L3) for the CLI.
//!
//! Same model as the desktop app's `memory::hierarchy` module:
//! L0 Conversation → L1 Atom → L2 Scenario → L3 Persona.

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::store::SharedConn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAtom {
    pub id: i64,
    pub kind: AtomKind,
    pub summary: String,
    pub source_quote: Option<String>,
    pub source_conversation_id: Option<String>,
    pub scenario_id: Option<i64>,
    pub confidence: f64,
    pub created_at_ms: i64,
    pub last_recalled_ms: i64,
    pub recall_count: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AtomKind {
    Preference,
    Fact,
    Decision,
    Instruction,
    Goal,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: i64,
    pub title: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub atom_count: i64,
}

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

pub struct HierarchicalMemory {
    conn: SharedConn,
}

impl HierarchicalMemory {
    pub fn new(conn: SharedConn) -> Self {
        Self { conn }
    }

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

    pub fn add_atom(
        &self,
        kind: AtomKind,
        summary: &str,
        source_quote: Option<&str>,
        source_conversation_id: Option<&str>,
        scenario_id: Option<i64>,
        confidence: f64,
    ) -> Result<i64> {
        let now = chrono::Utc::now().timestamp_millis();
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO memory_atoms
                 (kind, summary, source_quote, source_conversation_id, scenario_id, confidence,
                  created_at_ms, last_recalled_ms, recall_count)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, 0)"#,
            params![kind.as_str(), summary, source_quote, source_conversation_id, scenario_id, confidence, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_atoms(&self, limit: i64) -> Result<Vec<MemoryAtom>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT id, kind, summary, source_quote, source_conversation_id,
                      scenario_id, confidence, created_at_ms, last_recalled_ms, recall_count
                 FROM memory_atoms ORDER BY created_at_ms DESC LIMIT ?1"#,
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

    pub fn forget_atom(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM memory_atoms WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn upsert_scenario(&self, title: &str, summary: Option<&str>, tags: &[String]) -> Result<i64> {
        let now = chrono::Utc::now().timestamp_millis();
        let tags_csv = tags.join(",");
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO memory_scenarios (title, summary, tags, created_at_ms, updated_at_ms)
               VALUES (?1, ?2, ?3, ?4, ?4)
               ON CONFLICT(title) DO UPDATE SET
                   summary = COALESCE(excluded.summary, memory_scenarios.summary),
                   tags = COALESCE(excluded.tags, memory_scenarios.tags),
                   updated_at_ms = excluded.updated_at_ms"#,
            params![title, summary, tags_csv, now],
        )?;
        let id: i64 = conn.query_row(
            "SELECT id FROM memory_scenarios WHERE title = ?1",
            params![title],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn list_scenarios(&self) -> Result<Vec<Scenario>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT s.id, s.title, s.summary, s.tags, s.created_at_ms, s.updated_at_ms,
                      (SELECT COUNT(*) FROM memory_atoms a WHERE a.scenario_id = s.id) AS atom_count
                 FROM memory_scenarios s ORDER BY s.updated_at_ms DESC"#,
        )?;
        let rows = stmt.query_map([], |row| {
            let tags_csv: Option<String> = row.get(3)?;
            let tags = tags_csv.unwrap_or_default().split(',').filter(|s| !s.is_empty()).map(str::to_string).collect();
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

    pub fn set_persona_trait(&self, user_id: &str, key: &str, value: &str, confidence: f64, source: Option<&str>) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO memory_persona (user_id, key, value, confidence, source, updated_at_ms)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)
               ON CONFLICT(user_id, key) DO UPDATE SET
                   value = excluded.value,
                   confidence = excluded.confidence,
                   source = excluded.source,
                   updated_at_ms = excluded.updated_at_ms"#,
            params![user_id, key, value, confidence, source, now],
        )?;
        Ok(())
    }

    pub fn load_persona(&self, user_id: &str) -> Result<Persona> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT key, value, confidence, source, updated_at_ms
                 FROM memory_persona WHERE user_id = ?1 ORDER BY key ASC"#,
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
        Ok(Persona { user_id: user_id.to_string(), traits, updated_at_ms })
    }

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
                let tag_str = if s.tags.is_empty() { String::new() } else { format!(" [{}]", s.tags.join(", ")) };
                out.push_str(&format!("- **{}**{} — {} atoms\n", s.title, tag_str, s.atom_count));
            }
            out.push('\n');
        }
        if !atoms.is_empty() {
            out.push_str("## Recent memory atoms\n");
            for a in &atoms {
                out.push_str(&format!("- ({}) {} — conf={:.2}\n", a.kind.as_str(), a.summary, a.confidence));
            }
        }
        Ok(out)
    }
}

/// Deterministic atom extractor — same heuristic as the desktop app.
pub fn deterministic_extract(message: &str) -> Vec<(AtomKind, String)> {
    let mut out = Vec::new();
    let lower = message.to_lowercase();
    let decision_markers = ["let's", "lets", "we should", "we'll", "from now on", "remember:"];
    let pref_markers = ["i prefer", "i like", "i hate", "i love", "always use", "never use"];
    let goal_markers = ["i want to", "the goal is", "we need to ship", "i'm trying to"];
    for line in message.lines() {
        let line_lower = line.to_lowercase();
        if decision_markers.iter().any(|m| line_lower.contains(m)) {
            let t = line.trim();
            if t.len() > 8 && t.len() < 240 {
                out.push((AtomKind::Decision, t.to_string()));
            }
        } else if pref_markers.iter().any(|m| line_lower.contains(m)) {
            let t = line.trim();
            if t.len() > 8 && t.len() < 240 {
                out.push((AtomKind::Preference, t.to_string()));
            }
        } else if goal_markers.iter().any(|m| line_lower.contains(m)) {
            let t = line.trim();
            if t.len() > 8 && t.len() < 240 {
                out.push((AtomKind::Goal, t.to_string()));
            }
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out.dedup_by(|a, b| a.1 == b.1);
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
