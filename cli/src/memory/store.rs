//! SQLite store + migrations for the CLI memory module.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::Connection;

pub type SharedConn = Arc<Mutex<Connection>>;

pub struct MemoryStore {
    pub conn: SharedConn,
    pub conversations: super::conversation::ConversationStore,
    pub hierarchy: super::hierarchy::HierarchicalMemory,
    pub skills: super::skill_lib::SkillLibrary,
    pub wiki: super::wiki::Wiki,
    pub code_graph: super::codegraph::CodeGraph,
}

impl MemoryStore {
    pub fn open_in_memory() -> Result<Self> {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory()?));
        let store = Self::from_conn(conn);
        store.migrate()?;
        Ok(store)
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Arc::new(Mutex::new(Connection::open(path)?));
        let store = Self::from_conn(conn);
        store.migrate()?;
        Ok(store)
    }

    fn from_conn(conn: SharedConn) -> Self {
        Self {
            conversations: super::conversation::ConversationStore::new(conn.clone()),
            hierarchy: super::hierarchy::HierarchicalMemory::new(conn.clone()),
            skills: super::skill_lib::SkillLibrary::new(conn.clone()),
            wiki: super::wiki::Wiki::new(conn.clone()),
            code_graph: super::codegraph::CodeGraph::new(conn.clone()),
            conn,
        }
    }

    pub fn migrate(&self) -> Result<()> {
        self.conversations.migrate()?;
        self.hierarchy.migrate()?;
        self.skills.migrate()?;
        self.wiki.migrate()?;
        self.code_graph.migrate()?;
        Ok(())
    }
}
