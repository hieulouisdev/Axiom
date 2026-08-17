//! LM Studio — local OpenAI-compatible server.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct LmStudioProvider;
impl LmStudioProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "lmstudio",
            "LM Studio",
            "Local OpenAI-compatible server. No API key required.",
            "https://lmstudio.ai",
            ProviderCategory::Local,
            false,
            true,
            Some("http://localhost:1234/v1"),
            "local-model",
            &["local-model"],
            true,
        ))
    }
}
