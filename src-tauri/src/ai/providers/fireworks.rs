//! Fireworks AI — fast hosted inference for open-source models.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct FireworksProvider;
impl FireworksProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "fireworks",
            "Fireworks AI",
            "Fast hosted inference for open-source models.",
            "https://fireworks.ai",
            ProviderCategory::CloudOther,
            true,
            false,
            Some("https://api.fireworks.ai/inference/v1"),
            "accounts/fireworks/models/llama-v3p3-70b-instruct",
            &[
                "accounts/fireworks/models/llama-v3p3-70b-instruct",
                "accounts/fireworks/models/qwen2p5-72b-instruct",
            ],
            true,
        ))
    }
}
