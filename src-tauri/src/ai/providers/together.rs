//! Together AI — hosted open-source models (Llama, Qwen, DeepSeek).

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct TogetherProvider;
impl TogetherProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "together",
            "Together AI",
            "Hosted open-source models: Llama, Qwen, DeepSeek, Mixtral.",
            "https://together.ai",
            ProviderCategory::CloudMajor,
            true,
            false,
            Some("https://api.together.xyz/v1"),
            "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            &[
                "meta-llama/Llama-3.3-70B-Instruct-Turbo",
                "Qwen/Qwen2.5-72B-Instruct-Turbo",
                "deepseek-ai/DeepSeek-R1",
            ],
            true,
        ))
    }
}
