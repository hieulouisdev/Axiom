//! KoboldCpp — local fantasy-leaning LLM server with OpenAI-compatible API.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct KoboldCppProvider;
impl KoboldCppProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "koboldcpp",
            "KoboldCpp",
            "Local llama.cpp-based server with OpenAI-compatible API.",
            "https://github.com/LostRuins/koboldcpp",
            ProviderCategory::Local,
            false,
            true,
            Some("http://localhost:5001/v1"),
            "local-model",
            &["local-model"],
            true,
        ))
    }
}
