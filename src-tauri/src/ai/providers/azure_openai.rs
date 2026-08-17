//! Azure OpenAI Service — enterprise OpenAI deployment.
//!
//! Stub for v0.1: requires Azure-specific auth (AAD token or API key with
//! deployment-id), to be implemented in Phase 2 (see ROADMAP).

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, StubProvider};

pub struct AzureOpenAiProvider;
impl AzureOpenAiProvider {
    pub fn new() -> Arc<dyn Provider> {
        Arc::new(StubProvider::new(descriptor(
            "azure_openai",
            "Azure OpenAI",
            "Enterprise OpenAI deployment via Azure (AAD / API key).",
            "https://azure.microsoft.com/products/ai-services/openai-service",
            ProviderCategory::CloudOther,
            true,
            false,
            None,
            "gpt-4o",
            &["gpt-4o", "gpt-4", "gpt-35-turbo"],
            false,
        )))
    }
}
