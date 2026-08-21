//! Conversation store — L0 of the hierarchical memory.

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::store::SharedConn;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl MessageRole {
    pub fn as_str(self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
            MessageRole::Tool => "tool",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            "tool" => MessageRole::Tool,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub provider_id: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

pub struct ConversationStore {
    conn: SharedConn,
}

impl ConversationStore {
    pub fn new(conn: SharedConn) -> Self {
        Self { conn }
    }

    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS conversations (
                id              TEXT PRIMARY KEY,
                title           TEXT NOT NULL,
                provider_id     TEXT,
                created_at_ms   INTEGER NOT NULL,
                updated_at_ms   INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS messages (
                id              TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role            TEXT NOT NULL,
                content         TEXT NOT NULL,
                created_at_ms   INTEGER NOT NULL,
                FOREIGN KEY(conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);
            "#,
        )?;
        Ok(())
    }

    pub fn create_conversation(&self, id: &str, title: &str, provider_id: Option<&str>) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO conversations (id, title, provider_id, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![id, title, provider_id, now],
        )?;
        Ok(())
    }

    pub fn append_message(&self, conversation_id: &str, role: MessageRole, content: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content, created_at_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, conversation_id, role.as_str(), content, now],
        )?;
        conn.execute(
            "UPDATE conversations SET updated_at_ms = ?1 WHERE id = ?2",
            params![now, conversation_id],
        )?;
        Ok(id)
    }

    pub fn list_messages(&self, conversation_id: &str, limit: i64) -> Result<Vec<Message>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, created_at_ms FROM messages WHERE conversation_id = ?1 ORDER BY created_at_ms ASC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![conversation_id, limit], |row| {
            let role_str: String = row.get(2)?;
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: MessageRole::parse(&role_str).unwrap_or(MessageRole::User),
                content: row.get(3)?,
                created_at_ms: row.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn list_conversations(&self, limit: i64) -> Result<Vec<Conversation>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, provider_id, created_at_ms, updated_at_ms FROM conversations ORDER BY updated_at_ms DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                provider_id: row.get(2)?,
                created_at_ms: row.get(3)?,
                updated_at_ms: row.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn delete_conversation(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
        Ok(())
    }
}
