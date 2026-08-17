//! Hugging Face Inference API — hosted models on HF.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct HuggingFaceProvider;
impl HuggingFaceProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "huggingface",
            "Hugging Face",
            "Hosted models on Hugging Face Inference Endpoints.",
            "https://huggingface.co",
            ProviderCategory::CloudOther,
            true,
            false,
            Some("https://api-inference.huggingface.co/v1"),
            "meta-llama/Llama-3.3-70B-Instruct",
            &["meta-llama/Llama-3.3-70B-Instruct", "mistralai/Mistral-7B-Instruct-v0.3"],
            true,
        ))
    }
}
