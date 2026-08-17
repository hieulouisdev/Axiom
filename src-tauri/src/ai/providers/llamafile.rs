//! Llamafile — single-file distributable local LLM server.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct LlamafileProvider;
impl LlamafileProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "llamafile",
            "Llamafile",
            "Single-file distributable local LLM server (Mozilla+NODE)",
            "https://github.com/Mozilla-Ocho/llamafile",
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
