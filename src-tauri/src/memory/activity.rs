//! Activity log: append-only audit trail of every computer-use action.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::store::SharedConn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityRecord {
    pub id: String,
    pub kind: String,
    pub summary: String,
    pub detail_json: Option<String>,
    pub created_at_ms: u64,
    pub risk: Option<String>,
}

pub struct ActivityLog {
    conn: SharedConn,
}

impl ActivityLog {
    pub fn new(conn: SharedConn) -> Self {
        Self { conn }
    }

    pub fn record(&self, kind: &str, summary: &str, risk: Option<&str>) -> Result<ActivityRecord> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO activities (id, kind, summary, detail_json, created_at_ms, risk) VALUES (?1, ?2, ?3, NULL, ?4, ?5)",
            params![id, kind, summary, now as i64, risk],
        )?;
        Ok(ActivityRecord {
            id,
            kind: kind.to_string(),
            summary: summary.to_string(),
            detail_json: None,
            created_at_ms: now,
            risk: risk.map(Into::into),
        })
    }

    pub fn recent(&self, limit: u32) -> Result<Vec<ActivityRecord>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, kind, summary, detail_json, created_at_ms, risk FROM activities ORDER BY created_at_ms DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(ActivityRecord {
                id: row.get(0)?,
                kind: row.get(1)?,
                summary: row.get(2)?,
                detail_json: row.get(3)?,
                created_at_ms: row.get::<_, i64>(4)? as u64,
                risk: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn count(&self) -> Result<u64> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM activities", [], |row| row.get(0))?;
        Ok(count as u64)
    }
}
