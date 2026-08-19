//! Conversation history store.
//!
//! Phase 2: Added summarization support.

use std::sync::Arc;

use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::store::SharedConn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub provider_id: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    /// Optional summary of the conversation (for context pruning).
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at_ms: u64,
    pub metadata_json: Option<String>,
}

pub struct ConversationStore {
    conn: SharedConn,
}

impl ConversationStore {
    pub fn new(conn: SharedConn) -> Self {
        Self { conn }
    }

    pub fn create(&self, title: &str, provider_id: Option<&str>) -> Result<Conversation> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO conversations (id, title, provider_id, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![id, title, provider_id, now as i64],
        )?;
        Ok(Conversation {
            id,
            title: title.to_string(),
            provider_id: provider_id.map(Into::into),
            created_at_ms: now,
            updated_at_ms: now,
            summary: None,
        })
    }

    pub fn list(&self, limit: u32) -> Result<Vec<Conversation>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, provider_id, created_at_ms, updated_at_ms FROM conversations ORDER BY updated_at_ms DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                provider_id: row.get(2)?,
                created_at_ms: row.get::<_, i64>(3)? as u64,
                updated_at_ms: row.get::<_, i64>(4)? as u64,
                summary: None,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn add_message(&self, conversation_id: &str, role: &str, content: &str) -> Result<Message> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content, created_at_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, conversation_id, role, content, now as i64],
        )?;
        conn.execute(
            "UPDATE conversations SET updated_at_ms = ?1 WHERE id = ?2",
            params![now as i64, conversation_id],
        )?;
        Ok(Message {
            id,
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            created_at_ms: now,
            metadata_json: None,
        })
    }

    pub fn messages(&self, conversation_id: &str) -> Result<Vec<Message>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, created_at_ms, metadata_json FROM messages WHERE conversation_id = ?1 ORDER BY created_at_ms ASC",
        )?;
        let rows = stmt.query_map(params![conversation_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at_ms: row.get::<_, i64>(4)? as u64,
                metadata_json: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn delete(&self, conversation_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM messages WHERE conversation_id = ?1",
            params![conversation_id],
        )?;
        conn.execute(
            "DELETE FROM conversations WHERE id = ?1",
            params![conversation_id],
        )?;
        Ok(())
    }

    pub fn clear_all(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM messages", [])?;
        conn.execute("DELETE FROM conversations", [])?;
        Ok(())
    }

    pub fn search(&self, query: &str, limit: u32) -> Result<Vec<Message>> {
        let pattern = format!("%{query}%");
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, created_at_ms, metadata_json FROM messages WHERE content LIKE ?1 ORDER BY created_at_ms DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![pattern, limit as i64], |row| {
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at_ms: row.get::<_, i64>(4)? as u64,
                metadata_json: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Store a summary for a conversation.
    pub fn set_summary(&self, conversation_id: &str, summary: &str) -> Result<()> {
        let conn = self.conn.lock();
        // Add summary column if it doesn't exist
        let _ = conn.execute_batch("ALTER TABLE conversations ADD COLUMN summary TEXT");
        conn.execute(
            "UPDATE conversations SET summary = ?1 WHERE id = ?2",
            params![summary, conversation_id],
        )?;
        Ok(())
    }

    /// Get the summary for a conversation.
    pub fn get_summary(&self, conversation_id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let _ = conn.execute_batch("ALTER TABLE conversations ADD COLUMN summary TEXT");
        let mut stmt = conn.prepare("SELECT summary FROM conversations WHERE id = ?1")?;
        let mut rows = stmt.query(params![conversation_id])?;
        if let Some(row) = rows.next()? {
            Ok(row.get(0)?)
        } else {
            Ok(None)
        }
    }

    /// Prune old messages from a conversation, keeping only the N most recent.
    /// Returns the number of messages pruned.
    pub fn prune_old_messages(&self, conversation_id: &str, keep_count: u32) -> Result<u32> {
        let conn = self.conn.lock();
        // Delete all but the most recent `keep_count` messages
        let result = conn.execute(
            "DELETE FROM messages WHERE conversation_id = ?1 AND id NOT IN (
                SELECT id FROM messages WHERE conversation_id = ?1
                ORDER BY created_at_ms DESC LIMIT ?2
            )",
            params![conversation_id, keep_count as i64],
        )?;
        Ok(result as u32)
    }

    /// Get the number of messages in a conversation.
    pub fn message_count(&self, conversation_id: &str) -> Result<u32> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = ?1",
            params![conversation_id],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }
}

// Suppress unused warning for the Mutex/Connection imports — kept for API stability.
#[allow(dead_code)]
fn _force_use(_m: Mutex<Connection>, _a: Arc<()>) {}
