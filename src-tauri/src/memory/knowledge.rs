//! Knowledge base: selectively-stored facts the AI has learned.
//!
//! The AI is encouraged (via system prompt) to call `remember` only for
//! information that is:
//! - durable (won't change in minutes)
//! - personal (about the user, not generic trivia)
//! - actionable (will be useful in future sessions)

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::store::SharedConn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub key: String,
    pub value: String,
    pub source: Option<String>,
    pub confidence: f64,
    pub created_at_ms: u64,
    pub last_used_ms: u64,
    pub use_count: u64,
}

pub struct KnowledgeBase {
    conn: SharedConn,
}

impl KnowledgeBase {
    pub fn new(conn: SharedConn) -> Self {
        Self { conn }
    }

    pub fn remember(
        &self,
        key: &str,
        value: &str,
        source: Option<&str>,
        confidence: f64,
    ) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO knowledge (key, value, source, confidence, created_at_ms, last_used_ms, use_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0)
             ON CONFLICT(key) DO UPDATE SET value=?2, source=?3, confidence=?4, last_used_ms=?5",
            params![key, value, source, confidence, now as i64],
        )?;
        Ok(())
    }

    pub fn forget(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM knowledge WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn lookup(&self, key: &str) -> Result<Option<KnowledgeEntry>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT key, value, source, confidence, created_at_ms, last_used_ms, use_count FROM knowledge WHERE key = ?1",
        )?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            // Bump use count.
            conn.execute(
                "UPDATE knowledge SET use_count = use_count + 1, last_used_ms = ?1 WHERE key = ?2",
                params![time::OffsetDateTime::now_utc().unix_timestamp() * 1000, key],
            )?;
            return Ok(Some(KnowledgeEntry {
                key: row.get(0)?,
                value: row.get(1)?,
                source: row.get(2)?,
                confidence: row.get(3)?,
                created_at_ms: row.get::<_, i64>(4)? as u64,
                last_used_ms: row.get::<_, i64>(5)? as u64,
                use_count: row.get::<_, i64>(6)? as u64,
            }));
        }
        Ok(None)
    }

    pub fn list(&self, limit: u32) -> Result<Vec<KnowledgeEntry>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT key, value, source, confidence, created_at_ms, last_used_ms, use_count FROM knowledge ORDER BY last_used_ms DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(KnowledgeEntry {
                key: row.get(0)?,
                value: row.get(1)?,
                source: row.get(2)?,
                confidence: row.get(3)?,
                created_at_ms: row.get::<_, i64>(4)? as u64,
                last_used_ms: row.get::<_, i64>(5)? as u64,
                use_count: row.get::<_, i64>(6)? as u64,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn clear_all(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM knowledge", [])?;
        Ok(())
    }

    /// v0.4: semantic-ish search over the knowledge base.
    ///
    /// Returns the top `limit` entries ranked by a simple token-overlap score
    /// (Jaccard similarity on whitespace-tokenized lowercase tokens). This is
    /// a placeholder for a future vector-embedding-based search; it's good
    /// enough for short factual queries and avoids pulling in a heavy ML
    /// dependency.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<KnowledgeEntry>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT key, value, source, confidence, created_at_ms, last_used_ms, use_count FROM knowledge",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((KnowledgeEntry {
                key: row.get(0)?,
                value: row.get(1)?,
                source: row.get(2)?,
                confidence: row.get(3)?,
                created_at_ms: row.get::<_, i64>(4)? as u64,
                last_used_ms: row.get::<_, i64>(5)? as u64,
                use_count: row.get::<_, i64>(6)? as u64,
            },))
        })?;

        let q_tokens: std::collections::HashSet<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|s| !s.is_empty() && s.len() > 1)
            .collect();

        let mut scored: Vec<(f64, KnowledgeEntry)> = Vec::new();
        for r in rows {
            let entry = r?.0;
            // Combine key + value into a single bag of tokens.
            let combined = format!("{} {}", entry.key, entry.value).to_lowercase();
            let entry_tokens: std::collections::HashSet<String> = combined
                .split_whitespace()
                .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                .filter(|s| !s.is_empty() && s.len() > 1)
                .collect();
            // Jaccard similarity.
            let inter = q_tokens.intersection(&entry_tokens).count() as f64;
            let union = q_tokens.union(&entry_tokens).count() as f64;
            let score = if union > 0.0 { inter / union } else { 0.0 };
            // Bonus: substring match on the value.
            let bonus = if entry.value.to_lowercase().contains(&query.to_lowercase()) {
                0.25
            } else {
                0.0
            };
            let final_score = score + bonus + entry.confidence * 0.05;
            if final_score > 0.0 {
                scored.push((final_score, entry));
            }
        }
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().take(limit).map(|(_, e)| e).collect())
    }
}
