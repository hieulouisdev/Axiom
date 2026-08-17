//! Action audit log: every tool call the AI makes is appended to an
//! append-only audit trail stored in SQLite alongside the existing activity
//! log. This gives the user (and any future security reviewer) a tamper-evident
//! record of what the AI did, when, with what arguments, and what came back.
//!
//! Unlike the activity log (which records user-facing events), the audit log
//! records the *internal* AI tool calls — including ones that were denied by
//! the safety policy or aborted by the kill switch. This makes it possible
//! to reconstruct the AI's decision tree after the fact.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::Result;
use crate::memory::store::MemoryStore;

/// A single audit record. Stored in the `audit_log` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub ts_ms: i64,
    pub conversation_id: Option<String>,
    pub agent_run_id: Option<String>,
    pub tool_name: String,
    /// Arguments as a JSON string (truncated to 4 KB).
    pub arguments_json: String,
    /// Result content (truncated to 16 KB).
    pub result_json: String,
    /// "ok" | "error" | "denied" | "confirmation_required" | "rate_limited"
    pub outcome: String,
    pub duration_ms: u64,
}

const MAX_ARGS_LEN: usize = 4_096;
const MAX_RESULT_LEN: usize = 16_384;

/// Migrate the `audit_log` table. Called from `MemoryStore::migrate()`.
pub fn migrate(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS audit_log (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            ts_ms         INTEGER NOT NULL,
            conversation_id TEXT,
            agent_run_id  TEXT,
            tool_name     TEXT NOT NULL,
            arguments_json TEXT NOT NULL,
            result_json   TEXT NOT NULL,
            outcome       TEXT NOT NULL,
            duration_ms   INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS audit_log_ts_idx ON audit_log(ts_ms DESC);
        CREATE INDEX IF NOT EXISTS audit_log_conversation_idx ON audit_log(conversation_id);
        CREATE INDEX IF NOT EXISTS audit_log_tool_idx ON audit_log(tool_name);
        ",
    )?;
    Ok(())
}

/// Append a new audit entry. Returns the row id.
pub fn append(
    conn: &rusqlite::Connection,
    conversation_id: Option<&str>,
    agent_run_id: Option<&str>,
    tool_name: &str,
    arguments: &serde_json::Value,
    result: &str,
    outcome: &str,
    duration_ms: u64,
) -> Result<i64> {
    let now_ms = OffsetDateTime::now_utc().unix_timestamp() * 1000;
    let args_str = serde_json::to_string(arguments).unwrap_or_default();
    let args_str = truncate(args_str, MAX_ARGS_LEN);
    let result_trunc = truncate(result.to_string(), MAX_RESULT_LEN);

    conn.execute(
        "INSERT INTO audit_log (ts_ms, conversation_id, agent_run_id, tool_name, arguments_json, result_json, outcome, duration_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            now_ms,
            conversation_id,
            agent_run_id,
            tool_name,
            args_str,
            result_trunc,
            outcome,
            duration_ms as i64,
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(id)
}

/// Return the last N audit entries (newest first).
pub fn recent(conn: &rusqlite::Connection, limit: u32) -> Result<Vec<AuditEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, ts_ms, conversation_id, agent_run_id, tool_name, arguments_json, result_json, outcome, duration_ms
         FROM audit_log
         ORDER BY id DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(rusqlite::params![limit as i64], |row| {
        Ok(AuditEntry {
            id: row.get(0)?,
            ts_ms: row.get(1)?,
            conversation_id: row.get(2)?,
            agent_run_id: row.get(3)?,
            tool_name: row.get(4)?,
            arguments_json: row.get(5)?,
            result_json: row.get(6)?,
            outcome: row.get(7)?,
            duration_ms: row.get::<_, i64>(8)? as u64,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Total audit log size (used for stats).
pub fn count(conn: &rusqlite::Connection) -> Result<u64> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0))?;
    Ok(n as u64)
}

/// Helper: truncate a string at a UTF-8 char boundary near `max_len`.
fn truncate(s: String, max_len: usize) -> String {
    if s.len() <= max_len {
        return s;
    }
    let mut cut = max_len;
    // Step back to a UTF-8 boundary.
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut head = s[..cut].to_string();
    head.push_str("...[truncated]");
    head
}

/// Drop the audit log entirely (used by `aegis forget` / GDPR wipe).
pub fn wipe(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute("DELETE FROM audit_log", [])?;
    Ok(())
}

/// Convenience wrapper for the rest of the crate.
pub fn append_via(store: &MemoryStore, conv: Option<&str>, run: Option<&str>, tool: &str, args: &serde_json::Value, result: &str, outcome: &str, dur_ms: u64) -> Result<i64> {
    let conn = store.shared_conn();
    let conn = conn.lock();
    append(&conn, conv, run, tool, args, result, outcome, dur_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_handles_multibyte() {
        // Vietnamese: "Xin chào"
        let s = "Xin chào, đây là một chuỗi rất dài để kiểm tra truncate";
        let t = truncate(s.to_string(), 10);
        assert!(t.ends_with("...[truncated]"));
        // Should be valid UTF-8 (would panic otherwise).
        let _ = t.chars().count();
    }

    #[test]
    fn truncate_short_returns_input() {
        let s = "short".to_string();
        assert_eq!(truncate(s, 100), "short");
    }
}
