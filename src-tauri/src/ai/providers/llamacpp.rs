//! llama.cpp server — OpenAI-compatible server mode.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct LlamaCppProvider;
impl LlamaCppProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "llamacpp",
            "llama.cpp server",
            "llama.cpp built-in HTTP server (server mode).",
            "https://github.com/ggerganov/llama.cpp",
            ProviderCategory::Local,
            false,
            true,
            Some("http://localhost:8080/v1"),
            "local-model",
            &["local-model"],
            true,
        ))
    }
}
