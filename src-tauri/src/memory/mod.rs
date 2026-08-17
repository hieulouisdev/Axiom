//! Persistent memory store.
//!
//! A single SQLite database holds:
//! - `conversations` — chat history (messages, role, timestamp).
//! - `activities`   — audit log of every computer-use action.
//! - `knowledge`     — extracted facts the AI has learned (selectively kept).
//! - `events`        — security events / defense actions.
//! - `knowledge_embeddings` — vector embeddings of every knowledge entry,
//!   used by the v0.5 RAG pipeline (`memory::rag`).

pub mod activity;
pub mod conversation;
pub mod embeddings;
pub mod knowledge;
pub mod rag;
pub mod store;

pub use activity::{ActivityLog, ActivityRecord};
pub use conversation::{Conversation, ConversationStore, Message};
pub use embeddings::EmbeddingStore;
pub use knowledge::{KnowledgeBase, KnowledgeEntry};
pub use store::MemoryStore;
