//! DeepInfra — hosted open-source models.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct DeepInfraProvider;
impl DeepInfraProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "deepinfra",
            "DeepInfra",
            "Hosted open-source models with OpenAI-compatible API.",
            "https://deepinfra.com",
            ProviderCategory::CloudOther,
            true,
            false,
            Some("https://api.deepinfra.com/v1/openai"),
            "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            &["meta-llama/Llama-3.3-70B-Instruct-Turbo", "deepseek-ai/DeepSeek-R1"],
            true,
        ))
    }
}
