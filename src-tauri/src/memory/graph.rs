//! v1.6.0 — Knowledge Graph: entity-relation triples with multi-hop queries.
//!
//! The knowledge graph complements the existing `knowledge` (key-value) and
//! `embeddings` (vector) stores by storing **typed relations** between
//! entities the AI has encountered. Where `knowledge` answers "what is X?"
//! and `embeddings` answers "what's similar to X?", the graph answers
//! "how are X and Y connected?" and "what's related to X within N hops?".
//!
//! ## Storage
//!
//! Triples are persisted in the same SQLite database as the rest of the
//! memory store, in a dedicated `knowledge_graph` table. Sharing the
//! connection lets a single transaction atomically insert a knowledge row,
//! its embedding, **and** its graph edges.
//!
//! ## Schema
//!
//! ```sql
//! CREATE TABLE knowledge_graph (
//!     id           INTEGER PRIMARY KEY,
//!     subject      TEXT NOT NULL,
//!     predicate    TEXT NOT NULL,
//!     object       TEXT NOT NULL,
//!     source       TEXT,
//!     confidence   REAL NOT NULL DEFAULT 0.5,
//!     created_at_ms INTEGER NOT NULL,
//!     UNIQUE(subject, predicate, object)
//! );
//! CREATE INDEX idx_kg_subject   ON knowledge_graph(subject);
//! CREATE INDEX idx_kg_predicate ON knowledge_graph(predicate);
//! CREATE INDEX idx_kg_object    ON knowledge_graph(object);
//! ```
//!
//! ## Querying
//!
//! Three query shapes are supported:
//!
//! - **Pattern match** (`?s, p, o`) — return all triples matching a partial
//!   pattern (`(Option<String>, Option<String>, Option<String>)`). Any
//!   `None` is a wildcard. Mirrors SPARQL triple patterns.
//! - **Forward neighbours** (`neighbors(subject, depth)`) — BFS expansion
//!   returning everything reachable within `depth` hops from `subject`.
//! - **Path** between two nodes — BFS until `target` is reached or `depth`
//!   runs out. Useful for "how do these two entities relate?" queries.
//!
//! The query API returns plain `Triple` values; the agent loop can format
//! them into a system-prompt fragment via [`Self::prompt_for_subject`].

use std::collections::{HashSet, VecDeque};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::Result;

use super::store::SharedConn;

/// A single `(subject, predicate, object)` triple with provenance metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Triple {
    pub id: i64,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub source: Option<String>,
    pub confidence: f64,
    pub created_at_ms: i64,
}

/// In-memory knowledge graph backed by the SQLite connection shared with
/// the rest of the memory store.
pub struct KnowledgeGraph {
    conn: SharedConn,
}

impl KnowledgeGraph {
    pub fn new(conn: SharedConn) -> Self {
        Self { conn }
    }

    /// Create the `knowledge_graph` table and indexes. Idempotent.
    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS knowledge_graph (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                subject       TEXT NOT NULL,
                predicate     TEXT NOT NULL,
                object        TEXT NOT NULL,
                source        TEXT,
                confidence    REAL NOT NULL DEFAULT 0.5,
                created_at_ms INTEGER NOT NULL,
                UNIQUE(subject, predicate, object)
            );
            CREATE INDEX IF NOT EXISTS idx_kg_subject   ON knowledge_graph(subject);
            CREATE INDEX IF NOT EXISTS idx_kg_predicate ON knowledge_graph(predicate);
            CREATE INDEX IF NOT EXISTS idx_kg_object    ON knowledge_graph(object);
            "#,
        )?;
        Ok(())
    }

    /// Upsert a triple. If `(subject, predicate, object)` already exists,
    /// the source/confidence are updated. Returns the row id.
    pub fn upsert(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        source: Option<&str>,
        confidence: f64,
    ) -> Result<i64> {
        let conn = self.conn.lock();
        let now = OffsetDateTime::now_utc().unix_timestamp() * 1000;
        // Try insert first; on conflict, update source/confidence.
        let res = conn.execute(
            "INSERT INTO knowledge_graph (subject, predicate, object, source, confidence, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(subject, predicate, object) DO UPDATE SET
                 source = excluded.source,
                 confidence = excluded.confidence",
            params![subject, predicate, object, source, confidence, now],
        );
        match res {
            Ok(_) => Ok(conn.last_insert_rowid()),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete all triples matching the given pattern. Any `None` is a
    /// wildcard. Returns the number of rows deleted.
    pub fn delete_pattern(
        &self,
        subject: Option<&str>,
        predicate: Option<&str>,
        object: Option<&str>,
    ) -> Result<usize> {
        let conn = self.conn.lock();
        let mut sql = String::from("DELETE FROM knowledge_graph WHERE 1=1");
        let mut args: Vec<&str> = Vec::new();
        if let Some(s) = subject {
            sql.push_str(" AND subject = ?");
            args.push(s);
        }
        if let Some(p) = predicate {
            sql.push_str(" AND predicate = ?");
            args.push(p);
        }
        if let Some(o) = object {
            sql.push_str(" AND object = ?");
            args.push(o);
        }
        let rows = conn.execute(&sql, rusqlite::params_from_iter(args.iter().copied()))?;
        Ok(rows)
    }

    /// Query triples matching a partial pattern. Any `None` is a wildcard.
    /// Mirrors SPARQL triple patterns. Results are sorted by `confidence`
    /// descending then `created_at_ms` ascending (oldest first for ties).
    pub fn query(
        &self,
        subject: Option<&str>,
        predicate: Option<&str>,
        object: Option<&str>,
    ) -> Result<Vec<Triple>> {
        let conn = self.conn.lock();
        let mut sql = String::from(
            "SELECT id, subject, predicate, object, source, confidence, created_at_ms
             FROM knowledge_graph WHERE 1=1",
        );
        let mut args: Vec<&str> = Vec::new();
        if let Some(s) = subject {
            sql.push_str(" AND subject = ?");
            args.push(s);
        }
        if let Some(p) = predicate {
            sql.push_str(" AND predicate = ?");
            args.push(p);
        }
        if let Some(o) = object {
            sql.push_str(" AND object = ?");
            args.push(o);
        }
        sql.push_str(" ORDER BY confidence DESC, created_at_ms ASC LIMIT 500");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(args.iter().copied()), |row| {
            Ok(Triple {
                id: row.get(0)?,
                subject: row.get(1)?,
                predicate: row.get(2)?,
                object: row.get(3)?,
                source: row.get(4)?,
                confidence: row.get(5)?,
                created_at_ms: row.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// BFS expansion: return all triples reachable from `subject` within
    /// `depth` hops. The returned triples include the seed's first hop and
    /// follow `object` links recursively. The returned vec is in
    /// breadth-first order with no duplicates.
    pub fn neighbors(&self, subject: &str, depth: usize) -> Result<Vec<Triple>> {
        if depth == 0 {
            return Ok(vec![]);
        }
        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(subject.to_string());
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        queue.push_back((subject.to_string(), 0));
        let mut out: Vec<Triple> = Vec::new();
        let mut seen_triple_ids: HashSet<i64> = HashSet::new();

        while let Some((node, hops)) = queue.pop_front() {
            if hops >= depth {
                continue;
            }
            // Outgoing edges from `node`.
            for triple in self.query(Some(&node), None, None)? {
                let object = triple.object.clone();
                if seen_triple_ids.insert(triple.id) {
                    out.push(triple.clone());
                }
                if visited.insert(object.clone()) {
                    queue.push_back((object, hops + 1));
                }
            }
            // Also pull incoming edges (`node` as object) — useful for
            // "what points at this entity?" navigation.
            for triple in self.query(None, None, Some(&node))? {
                let subject = triple.subject.clone();
                if seen_triple_ids.insert(triple.id) {
                    out.push(triple.clone());
                }
                if visited.insert(subject.clone()) {
                    queue.push_back((subject, hops + 1));
                }
            }
        }
        Ok(out)
    }

    /// Shortest path between two entities. Returns the sequence of triples
    /// traversed from `start` to `target`, or an empty vec if no path within
    /// `max_depth` hops. The path alternates direction freely — out-edges
    /// (`subject` → `object`) and in-edges (`object` → `subject`).
    pub fn path(&self, start: &str, target: &str, max_depth: usize) -> Result<Vec<Triple>> {
        if start == target || max_depth == 0 {
            return Ok(vec![]);
        }
        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(start.to_string());
        let mut queue: VecDeque<(String, Vec<Triple>)> = VecDeque::new();
        queue.push_back((start.to_string(), vec![]));

        while let Some((node, path_so_far)) = queue.pop_front() {
            if path_so_far.len() >= max_depth {
                continue;
            }
            for triple in self.query(Some(&node), None, None)? {
                // Clone the field we need to use after `triple` is moved
                // into `path_so_far.push(triple)`.
                let object = triple.object.clone();
                if object == target {
                    let mut final_path = path_so_far.clone();
                    final_path.push(triple);
                    return Ok(final_path);
                }
                if visited.insert(object.clone()) {
                    let mut p = path_so_far.clone();
                    p.push(triple);
                    queue.push_back((object, p));
                }
            }
            for triple in self.query(None, None, Some(&node))? {
                let subject = triple.subject.clone();
                if subject == target {
                    let mut final_path = path_so_far.clone();
                    final_path.push(triple);
                    return Ok(final_path);
                }
                if visited.insert(subject.clone()) {
                    let mut p = path_so_far.clone();
                    p.push(triple);
                    queue.push_back((subject, p));
                }
            }
        }
        Ok(vec![])
    }

    /// Render a human-readable paragraph describing the immediate
    /// neighborhood of `subject`. Used by the agent loop's RAG pipeline
    /// to inject graph context into the system prompt.
    pub fn prompt_for_subject(&self, subject: &str, depth: usize) -> Result<String> {
        let triples = self.neighbors(subject, depth)?;
        if triples.is_empty() {
            return Ok(String::new());
        }
        let mut lines = Vec::with_capacity(triples.len() + 2);
        lines.push(format!(
            "--- Knowledge graph: neighborhood of '{subject}' (depth={depth}) ---"
        ));
        for t in triples {
            lines.push(format!(
                "  {} --{}--> {} (conf={:.2})",
                t.subject, t.predicate, t.object, t.confidence
            ));
        }
        Ok(lines.join("\n"))
    }

    /// Total triple count.
    pub fn count(&self) -> Result<i64> {
        let conn = self.conn.lock();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM knowledge_graph", [], |r| r.get(0))?;
        Ok(n)
    }

    /// Distinct subjects (entities the graph knows about). Useful for the
    /// Studio UI's "entities" list.
    pub fn subjects(&self, limit: usize) -> Result<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT subject FROM knowledge_graph
             ORDER BY subject ASC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Distinct predicate types (relations in use). Useful for the UI's
    /// "available relations" picker.
    pub fn predicates(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt =
            conn.prepare("SELECT DISTINCT predicate FROM knowledge_graph ORDER BY predicate ASC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Clear the entire graph.
    pub fn clear(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch("DELETE FROM knowledge_graph")?;
        Ok(())
    }
}

/// Helper: extract entity name from a knowledge key. Knowledge keys are
/// typically prefixed like `name:John` or `org:Acme` — the entity name is
/// the part after the colon. This is used by the agent loop to auto-link
/// new knowledge entries into the graph.
pub fn entity_name_from_key(key: &str) -> &str {
    if let Some(idx) = key.find(':') {
        &key[idx + 1..]
    } else {
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;

    fn open() -> KnowledgeGraph {
        let conn = Arc::new(Mutex::new(rusqlite::Connection::open_in_memory().unwrap()));
        let g = KnowledgeGraph::new(conn);
        g.migrate().unwrap();
        g
    }

    #[test]
    fn upsert_idempotent() {
        let g = open();
        let id1 = g
            .upsert("alice", "knows", "bob", Some("chat"), 0.9)
            .unwrap();
        let id2 = g
            .upsert("alice", "knows", "bob", Some("chat2"), 0.8)
            .unwrap();
        // ON CONFLICT path returns the existing id (sqlite's behavior with
        // last_insert_rowid is implementation-defined for UPDATEs, so we just
        // verify there's exactly one row with the latest confidence).
        let _ = (id1, id2);
        let rows = g.query(Some("alice"), None, None).unwrap();
        assert_eq!(rows.len(), 1);
        assert!((rows[0].confidence - 0.8).abs() < 1e-6);
    }

    #[test]
    fn query_wildcards() {
        let g = open();
        g.upsert("alice", "knows", "bob", None, 0.5).unwrap();
        g.upsert("alice", "works_at", "acme", None, 0.5).unwrap();
        g.upsert("bob", "knows", "carol", None, 0.5).unwrap();

        // All triples.
        assert_eq!(g.query(None, None, None).unwrap().len(), 3);
        // Subject = alice.
        assert_eq!(g.query(Some("alice"), None, None).unwrap().len(), 2);
        // Object = bob.
        assert_eq!(g.query(None, None, Some("bob")).unwrap().len(), 1);
        // Predicate = knows.
        assert_eq!(g.query(None, Some("knows"), None).unwrap().len(), 2);
    }

    #[test]
    fn neighbors_bfs() {
        let g = open();
        // alice -> bob -> carol
        g.upsert("alice", "knows", "bob", None, 0.5).unwrap();
        g.upsert("bob", "knows", "carol", None, 0.5).unwrap();

        let depth1 = g.neighbors("alice", 1).unwrap();
        assert_eq!(depth1.len(), 1); // alice -> bob
        let depth2 = g.neighbors("alice", 2).unwrap();
        assert!(depth2.len() >= 2); // alice -> bob, bob -> carol
    }

    #[test]
    fn path_finds_shortest() {
        let g = open();
        g.upsert("alice", "knows", "bob", None, 0.5).unwrap();
        g.upsert("bob", "knows", "carol", None, 0.5).unwrap();

        let path = g.path("alice", "carol", 5).unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].subject, "alice");
        assert_eq!(path[1].subject, "bob");
    }

    #[test]
    fn path_returns_empty_when_no_connection() {
        let g = open();
        g.upsert("alice", "knows", "bob", None, 0.5).unwrap();
        g.upsert("zoe", "knows", "yan", None, 0.5).unwrap();
        let path = g.path("alice", "yan", 5).unwrap();
        assert!(path.is_empty());
    }

    #[test]
    fn count_and_subjects() {
        let g = open();
        g.upsert("alice", "knows", "bob", None, 0.5).unwrap();
        g.upsert("alice", "works_at", "acme", None, 0.5).unwrap();
        assert_eq!(g.count().unwrap(), 2);
        let subs = g.subjects(10).unwrap();
        assert!(subs.contains(&"alice".to_string()));
    }

    #[test]
    fn prompt_for_subject_empty_when_no_triples() {
        let g = open();
        let p = g.prompt_for_subject("nobody", 2).unwrap();
        assert!(p.is_empty());
    }

    #[test]
    fn prompt_for_subject_renders_triples() {
        let g = open();
        g.upsert("alice", "knows", "bob", None, 0.7).unwrap();
        let p = g.prompt_for_subject("alice", 1).unwrap();
        assert!(p.contains("alice"));
        assert!(p.contains("knows"));
        assert!(p.contains("bob"));
    }

    #[test]
    fn clear_removes_everything() {
        let g = open();
        g.upsert("a", "x", "b", None, 0.5).unwrap();
        g.upsert("c", "y", "d", None, 0.5).unwrap();
        assert_eq!(g.count().unwrap(), 2);
        g.clear().unwrap();
        assert_eq!(g.count().unwrap(), 0);
    }

    #[test]
    fn entity_name_from_key_strips_prefix() {
        assert_eq!(entity_name_from_key("name:John"), "John");
        assert_eq!(entity_name_from_key("plain"), "plain");
        assert_eq!(entity_name_from_key("org:Acme Inc"), "Acme Inc");
    }
}
