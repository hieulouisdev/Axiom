//! Anyscale — hosted Llama 3.1 / 3.2 endpoint service.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct AnyscaleProvider;
impl AnyscaleProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "anyscale",
            "Anyscale",
            "Hosted Llama 3.1 / 3.2 endpoint service.",
            "https://anyscale.com",
            ProviderCategory::CloudMajor,
            true,
            false,
            Some("https://api.endpoints.anyscale.com/v1"),
            "meta-llama/Llama-3.1-70B-Instruct",
            &[
                "meta-llama/Llama-3.1-70B-Instruct",
                "meta-llama/Llama-3.1-8B-Instruct",
            ],
            true,
        ))
    }
}
