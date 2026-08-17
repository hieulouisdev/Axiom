//! Moonshot AI (Kimi) — Chinese cloud LLM provider.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct MoonshotProvider;
impl MoonshotProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "moonshot",
            "Moonshot AI (Kimi)",
            "Chinese cloud LLM provider (Kimi) with OpenAI-compatible API.",
            "https://platform.moonshot.cn",
            ProviderCategory::CloudOther,
            true,
            false,
            Some("https://api.moonshot.cn/v1"),
            "moonshot-v1-8k",
            &["moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k"],
            true,
        ))
    }
}
