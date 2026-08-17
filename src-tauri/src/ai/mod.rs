//! AI subsystem: provider trait, registry, router, and 20+ provider implementations.

pub mod provider;
pub mod router;
pub mod providers;

pub use provider::{ChatMessage, ChatRequest, ChatResponse, Provider, ProviderRegistry};
pub use router::AiRouter;
