//! Cohere — Command R+ / Command R via Cohere Chat API.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct CohereProvider;
impl CohereProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "cohere",
            "Cohere",
            "Command R+ / Command R via the Cohere Chat API.",
            "https://cohere.com",
            ProviderCategory::CloudMajor,
            true,
            false,
            Some("https://api.cohere.ai/v1"),
            "command-r-plus",
            &["command-r-plus", "command-r", "command-r-08-2024"],
            true,
        ))
    }
}
