//! DeepSeek — DeepSeek-V3 / DeepSeek-R1 reasoning models.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct DeepSeekProvider;
impl DeepSeekProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "deepseek",
            "DeepSeek",
            "DeepSeek-V3 chat and DeepSeek-R1 reasoning models.",
            "https://platform.deepseek.com",
            ProviderCategory::CloudMajor,
            true,
            false,
            Some("https://api.deepseek.com/v1"),
            "deepseek-chat",
            &["deepseek-chat", "deepseek-reasoner"],
            true,
        ))
    }
}
