//! OpenRouter — unified gateway to 200+ models.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct OpenRouterProvider;
impl OpenRouterProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "openrouter",
            "OpenRouter",
            "Unified gateway to 200+ models with pay-as-you-go.",
            "https://openrouter.ai",
            ProviderCategory::CloudMajor,
            true,
            false,
            Some("https://openrouter.ai/api/v1"),
            "openai/gpt-4o-mini",
            &[
                "openai/gpt-4o",
                "anthropic/claude-3.5-sonnet",
                "google/gemini-pro-1.5",
                "meta-llama/llama-3.3-70b-instruct",
            ],
            true,
        ))
    }
}
