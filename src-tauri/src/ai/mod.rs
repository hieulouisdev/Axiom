//! AI subsystem: provider trait, registry, router, and 20+ provider implementations.
//!
//! v0.3 adds three new modules:
//! - [`fast_path`] — tuned HTTP client + LRU response cache + dedup layer.
//! - [`tools`] — registry of tools the AI agent can invoke locally.
//! - [`agent`] — tool-use agent loop that lets the AI act as a computer-use
//!   "co-owner" while respecting the safety policy.

pub mod agent;
pub mod fast_path;
pub mod provider;
pub mod router;
pub mod providers;
pub mod tools;

pub use provider::{ChatMessage, ChatRequest, ChatResponse, Provider, ProviderRegistry};
pub use router::AiRouter;
