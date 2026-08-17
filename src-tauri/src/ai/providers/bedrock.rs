//! AWS Bedrock — managed Anthropic / Meta / Mistral models on AWS.
//!
//! Stub for v0.1: requires AWS SigV4 request signing, scheduled for Phase 2.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, StubProvider};

pub struct BedrockProvider;
impl BedrockProvider {
    pub fn new() -> Arc<dyn Provider> {
        Arc::new(StubProvider::new(descriptor(
            "bedrock",
            "AWS Bedrock",
            "Managed Anthropic / Meta / Mistral models on AWS (SigV4 auth).",
            "https://aws.amazon.com/bedrock",
            ProviderCategory::CloudOther,
            true,
            false,
            None,
            "anthropic.claude-3-5-sonnet-20240620-v1:0",
            &["anthropic.claude-3-5-sonnet-20240620-v1:0", "meta.llama3-1-70b-instruct-v1:0"],
            false,
        )))
    }
}
