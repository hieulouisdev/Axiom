//! SQLite-backed memory store: opens the DB, runs migrations, exposes
//! per-domain stores (conversations / activities / knowledge).

use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;
use rusqlite::Connection;

use crate::error::Result;

use super::activity::ActivityLog;
use super::conversation::ConversationStore;
use super::knowledge::KnowledgeBase;

/// Type alias for the shared connection used by all sub-stores.
pub type SharedConn = Arc<Mutex<Connection>>;

pub struct MemoryStore {
    conn: SharedConn,
    pub conversations: ConversationStore,
    pub activity: ActivityLog,
    pub knowledge: KnowledgeBase,
}

impl MemoryStore {
    /// Opens an in-memory database (used at boot before the persistent DB
    /// is wired up, and for tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory()?));
        let store = Self::from_conn(conn);
        store.migrate()?;
        Ok(store)
    }

    /// Opens a persistent database file.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Arc::new(Mutex::new(Connection::open(path)?));
        Ok(Self::from_conn(conn))
    }

    fn from_conn(conn: SharedConn) -> Self {
        Self {
            conversations: ConversationStore::new(conn.clone()),
            activity: ActivityLog::new(conn.clone()),
            knowledge: KnowledgeBase::new(conn.clone()),
            conn,
        }
    }

    /// Runs schema migrations.
    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS conversations (
                id              TEXT PRIMARY KEY,
                title           TEXT NOT NULL,
                provider_id     TEXT,
                created_at_ms   INTEGER NOT NULL,
                updated_at_ms   INTEGER NOT NULL,
                summary         TEXT
            );

            CREATE TABLE IF NOT EXISTS messages (
                id              TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role            TEXT NOT NULL,
                content         TEXT NOT NULL,
                created_at_ms   INTEGER NOT NULL,
                metadata_json   TEXT,
                FOREIGN KEY(conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);

            CREATE TABLE IF NOT EXISTS activities (
                id              TEXT PRIMARY KEY,
                kind            TEXT NOT NULL,
                summary         TEXT NOT NULL,
                detail_json     TEXT,
                created_at_ms   INTEGER NOT NULL,
                risk            TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_activities_kind ON activities(kind);
            CREATE INDEX IF NOT EXISTS idx_activities_created ON activities(created_at_ms);

            CREATE TABLE IF NOT EXISTS knowledge (
                key             TEXT PRIMARY KEY,
                value           TEXT NOT NULL,
                source          TEXT,
                confidence      REAL NOT NULL DEFAULT 0.5,
                created_at_ms   INTEGER NOT NULL,
                last_used_ms    INTEGER NOT NULL,
                use_count       INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS events (
                id              TEXT PRIMARY KEY,
                kind            TEXT NOT NULL,
                severity        TEXT NOT NULL,
                detail_json     TEXT,
                created_at_ms   INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_events_created ON events(created_at_ms);

            CREATE TABLE IF NOT EXISTS integrity_baselines (
                path            TEXT PRIMARY KEY,
                hash_sha256     TEXT NOT NULL,
                saved_at_ms     INTEGER NOT NULL
            );
            "#,
        )?;

        // v0.3: audit log table — every AI tool call is recorded here.
        crate::computer::audit::migrate(&conn)?;

        Ok(())
    }

    /// Returns a clone of the shared connection handle. Useful for callers
    /// that want to perform transactional work outside the sub-stores.
    pub fn shared_conn(&self) -> SharedConn {
        self.conn.clone()
    }
}
