//! LocalAI — self-hosted OpenAI-compatible inference.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct LocalAiProvider;
impl LocalAiProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "localai",
            "LocalAI",
            "Self-hosted OpenAI-compatible inference (CPU/GPU).",
            "https://localai.io",
            ProviderCategory::Local,
            false,
            true,
            Some("http://localhost:8080/v1"),
            "gpt-3.5-turbo",
            &["gpt-3.5-turbo", "llama-3-8b-instruct"],
            true,
        ))
    }
}
