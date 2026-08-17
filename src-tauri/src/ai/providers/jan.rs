//! Jan — local desktop LLM runner with OpenAI-compatible API.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct JanProvider;
impl JanProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "jan",
            "Jan",
            "Local desktop LLM runner with OpenAI-compatible API.",
            "https://jan.ai",
            ProviderCategory::Local,
            false,
            true,
            Some("http://localhost:1337/v1"),
            "llama3.2",
            &["llama3.2", "mistral-ins7", "qwen2.5"],
            true,
        ))
    }
}
