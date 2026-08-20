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
//!
//! v0.6 adds:
//! - [`web`] — real web search (DuckDuckGo HTML endpoint) and HTML readability
//!   extraction. The `web_search` tool is no longer a stub.
//!
//! v1.6 adds:
//! - [`orchestrator`] — multi-agent DAG planner/executor that decomposes a
//!   high-level goal into topologically-sorted steps, dispatches independent
//!   branches concurrently up to `orchestrator_max_parallel`, and emits
//!   per-step Tauri events for live UI progress.

pub mod agent;
pub mod catalog;
pub mod fast_path;
pub mod orchestrator;
pub mod provider;
pub mod providers;
pub mod router;
pub mod skills;
pub mod tools;
pub mod web;

pub use provider::{ChatMessage, ChatRequest, ChatResponse, Provider, ProviderRegistry};
pub use router::AiRouter;
