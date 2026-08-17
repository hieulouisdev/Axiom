//! Persistent memory store.
//!
//! A single SQLite database holds:
//! - `conversations` — chat history (messages, role, timestamp).
//! - `activities`   — audit log of every computer-use action.
//! - `knowledge`     — extracted facts the AI has learned (selectively kept).
//! - `events`        — security events / defense actions.

pub mod activity;
pub mod conversation;
pub mod knowledge;
pub mod store;

pub use activity::{ActivityLog, ActivityRecord};
pub use conversation::{Conversation, ConversationStore, Message};
pub use knowledge::{KnowledgeBase, KnowledgeEntry};
pub use store::MemoryStore;
