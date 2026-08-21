//! Generic OpenAI-compatible provider — used as a fallback for any
//! provider not explicitly implemented (e.g. custom self-hosted endpoints).

use async_trait::async_trait;
use std::time::Duration;

use crate::ai::provider::{ChatRequest, ChatResponse, Provider, ProviderConfig};

pub struct OpenAiCompatProvider {
    cfg: ProviderConfig,
    client: reqwest::Client,
}

impl OpenAiCompatProvider {
    pub fn new(cfg: ProviderConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest");
        Self { cfg, client }
    }
}

#[async_trait]
impl Provider for OpenAiCompatProvider {
    fn id(&self) -> &str { &self.cfg.id }
    fn display_name(&self) -> &str { &self.cfg.id }
    fn default_model(&self) -> &str { self.cfg.default_model.as_deref().unwrap_or("gpt-3.5-turbo") }

    async fn chat(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let base = self.cfg.base_url.as_deref().ok_or_else(|| anyhow::anyhow!("{}: base_url required", self.cfg.id))?;
        let model = if req.model.is_empty() { self.default_model().to_string() } else { req.model.clone() };
        let payload = super::build_chat_payload(req, &model);
        let url = format!("{}/chat/completions", base.trim_end_matches('/'));
        let mut req_builder = self.client.post(&url).json(&payload);
        if let Some(key) = self.cfg.api_key.as_deref() {
            req_builder = req_builder.bearer_auth(key);
        }
        let resp = req_builder.send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("{}: {} — {}", self.cfg.id, status, body);
        }
        super::parse_chat_response(&body, &self.cfg.id, &model)
    }
}
