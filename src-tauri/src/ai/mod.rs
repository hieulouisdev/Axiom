//! AI subsystem: provider trait, registry, router, and 20+ provider implementations.
//!
//! v0.3 adds three new modules:
//! - [`fast_path`] — tuned HTTP client + LRU response cache + dedup layer.
//! - [`tools`] — registry of tools the AI agent can invoke locally.
//! - [`agent`] — tool-use agent loop that lets the AI act as a computer-use
//!   "co-owner" while respecting the safety policy.
//!
//! v0.4 adds:
//! - [`catalog`] — compile-time catalog of every known AI model (10k+ entries)
//!   merged from `ai-model-directory` and `models-dev` reference repos.
//! - [`skills`] — declarative skill packs the AI can load to specialize its
//!   behavior (code_writer, code_reviewer, refactor, test_writer, …).

pub mod agent;
pub mod catalog;
pub mod fast_path;
pub mod provider;
pub mod providers;
pub mod router;
pub mod skills;
pub mod tools;

pub use provider::{ChatMessage, ChatRequest, ChatResponse, Provider, ProviderRegistry};
pub use router::AiRouter;
