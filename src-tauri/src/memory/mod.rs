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
//!
//! v1.6 adds [`graph`] — a typed entity-relation store complementing the
//! key-value `knowledge` table. Triples `(subject, predicate, object)` let
//! the agent answer multi-hop questions ("how do X and Y relate?") that
//! flat-key lookups can't.

pub mod activity;
pub mod conversation;
pub mod embeddings;
pub mod encryption;
pub mod entities;
pub mod graph;
pub mod knowledge;
pub mod rag;
pub mod store;

pub use activity::{ActivityLog, ActivityRecord};
pub use conversation::{Conversation, ConversationStore, Message};
pub use embeddings::EmbeddingStore;
pub use encryption::{EncryptionStatus, status as encryption_status};
pub use entities::{ExtractedEntity, extract_and_store, extract_from_messages};
pub use graph::{KnowledgeGraph, Triple};
pub use knowledge::{KnowledgeBase, KnowledgeEntry};
pub use store::MemoryStore;
