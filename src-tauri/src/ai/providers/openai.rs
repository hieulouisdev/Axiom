//! OpenAI — GPT-4o, GPT-4 Turbo, GPT-3.5 Turbo.

use std::sync::Arc;

use super::openai_compat::{OpenAiCompatProvider, descriptor};
use crate::ai::provider::{Provider, ProviderCategory};

pub struct OpenAiProvider(Arc<OpenAiCompatProvider>);

impl OpenAiProvider {
    pub fn new() -> Arc<Self> {
        let desc = descriptor(
            "openai",
            "OpenAI",
            "GPT-4o, GPT-4 Turbo, GPT-3.5 Turbo. Cloud-hosted, requires API key.",
            "https://openai.com",
            ProviderCategory::CloudMajor,
            true,
            false,
            Some("https://api.openai.com/v1"),
            "gpt-4o-mini",
            &["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo"],
            true,
        );
        Arc::new(Self(Arc::new(OpenAiCompatProvider::new(desc))))
    }
}

// Manual Provider impl that delegates to the inner client.
#[async_trait::async_trait]
impl Provider for OpenAiProvider {
    fn descriptor(&self) -> &crate::ai::provider::ProviderDescriptor {
        self.0.descriptor()
    }
    fn set_creds(&self, creds: crate::ai::provider::ProviderCreds) {
        self.0.set_creds(creds)
    }
    async fn chat(
        &self,
        req: crate::ai::provider::ChatRequest,
    ) -> crate::error::Result<crate::ai::provider::ChatResponse> {
        self.0.chat(req).await
    }
    async fn chat_stream(
        &self,
        req: crate::ai::provider::ChatRequest,
        on_chunk: Box<dyn Fn(crate::ai::provider::ChatStreamChunk) + Send + Sync>,
    ) -> crate::error::Result<crate::ai::provider::ChatResponse> {
        self.0.chat_stream(req, on_chunk).await
    }
    async fn ping(&self) -> crate::error::Result<()> {
        self.0.ping().await
    }
}
