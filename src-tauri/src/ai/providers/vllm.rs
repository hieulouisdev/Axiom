//! vLLM — high-throughput local inference engine.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct VllmProvider;
impl VllmProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "vllm",
            "vLLM",
            "High-throughput local inference engine (OpenAI-compatible).",
            "https://github.com/vllm-project/vllm",
            ProviderCategory::Local,
            false,
            true,
            Some("http://localhost:8000/v1"),
            "meta-llama/Llama-3.2-1B-Instruct",
            &[
                "meta-llama/Llama-3.2-1B-Instruct",
                "Qwen/Qwen2.5-7B-Instruct",
            ],
            true,
        ))
    }
}
