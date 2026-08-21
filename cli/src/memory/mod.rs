//! Memory module for the CLI — SQLite-backed hierarchical memory.
//!
//! Vendors the v1.7 hierarchical memory model (L0→L3) directly into the
//! CLI so the binary has zero runtime deps on the desktop app.

pub mod store;
pub mod hierarchy;
pub mod skill_lib;
pub mod wiki;
pub mod codegraph;
pub mod conversation;

pub use store::{MemoryStore, SharedConn};
pub use hierarchy::{HierarchicalMemory, MemoryAtom, AtomKind, Scenario, Persona, PersonaTrait, deterministic_extract};
pub use skill_lib::{SkillLibrary, Skill, SkillVersion, SkillStatus, Visibility, SkillTrigger, SkillStep};
pub use wiki::{Wiki, WikiPage, WikiLink};
pub use codegraph::{CodeGraph, Symbol, SymbolKind, Repo};
pub use conversation::{Conversation, ConversationStore, Message, MessageRole};
