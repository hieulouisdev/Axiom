//! Replicate — hosted open-source models with prediction API.
//!
//! Stub for v0.1: Replicate uses a different async prediction model,
//! scheduled for Phase 2.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, StubProvider};

pub struct ReplicateProvider;
impl ReplicateProvider {
    pub fn new() -> Arc<dyn Provider> {
        Arc::new(StubProvider::new(descriptor(
            "replicate",
            "Replicate",
            "Hosted open-source models via Prediction API (async).",
            "https://replicate.com",
            ProviderCategory::CloudOther,
            true,
            false,
            None,
            "meta/llama-3.3-70b-instruct",
            &["meta/llama-3.3-70b-instruct", "mistralai/mistral-7b-instruct-v0.3"],
            false,
        )))
    }
}
