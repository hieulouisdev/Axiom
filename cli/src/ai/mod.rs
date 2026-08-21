//! AI module for the CLI — provider trait + key providers.

pub mod provider;
pub mod providers;
pub mod router;

pub use provider::{ChatMessage, ChatRequest, ChatResponse, Provider, ProviderRegistry, ProviderConfig, Role, Usage};
pub use router::AiRouter;
