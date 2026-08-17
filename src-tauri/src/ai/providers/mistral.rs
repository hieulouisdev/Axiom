//! Mistral AI — Mistral Large / Codestral / Pixtral.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct MistralProvider;
impl MistralProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "mistral",
            "Mistral AI",
            "Mistral Large, Codestral, Pixtral via la Plateforme.",
            "https://mistral.ai",
            ProviderCategory::CloudMajor,
            true,
            false,
            Some("https://api.mistral.ai/v1"),
            "mistral-small-latest",
            &["mistral-large-latest", "mistral-small-latest", "codestral-latest", "pixtral-12b-2409"],
            true,
        ))
    }
}
