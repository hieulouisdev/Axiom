//! Persistent memory store.
//!
//! A single SQLite database holds:
//! - `conversations` — chat history (messages, role, timestamp).
//! - `activities`   — audit log of every computer-use action.
//! - `knowledge`     — extracted facts the AI has learned (selectively kept).
//! - `events`        — security events / defense actions.
//! - `knowledge_embeddings` — vector embeddings of every knowledge entry,
//!   used by the v0.5 RAG pipeline (`memory::rag`).
//!
//! v0.6 adds [`entities`] — automatic extraction of emails, URLs, names,
//! locations, and other durable facts from chat history. This closes the
//! RAG loop: every chat contributes to the user's long-term memory without
//! requiring explicit `memory_remember` calls.

pub mod activity;
pub mod conversation;
pub mod embeddings;
pub mod encryption;
pub mod entities;
pub mod knowledge;
pub mod rag;
pub mod store;

pub use activity::{ActivityLog, ActivityRecord};
pub use conversation::{Conversation, ConversationStore, Message};
pub use embeddings::EmbeddingStore;
pub use encryption::{status as encryption_status, EncryptionStatus};
pub use entities::{extract_and_store, extract_from_messages, ExtractedEntity};
pub use knowledge::{KnowledgeBase, KnowledgeEntry};
pub use store::MemoryStore;
