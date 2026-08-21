//! Z.AI (GLM) provider — OpenAI-compatible, includes a zero-key public route
//! for the GLM-4.6 public preview.

use async_trait::async_trait;
use std::time::Duration;

use crate::ai::provider::{ChatRequest, ChatResponse, Provider, ProviderConfig};

pub struct ZaiProvider {
    cfg: ProviderConfig,
    client: reqwest::Client,
}

impl ZaiProvider {
    pub fn new(cfg: ProviderConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest");
        Self { cfg, client }
    }
}

#[async_trait]
impl Provider for ZaiProvider {
    fn id(&self) -> &str { "zai" }
    fn display_name(&self) -> &str { "Z.AI (GLM)" }
    fn default_model(&self) -> &str { self.cfg.default_model.as_deref().unwrap_or("glm-4.6") }

    async fn chat(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let base = self.cfg.base_url.as_deref().unwrap_or("https://api.z.ai/api/paas/v4");
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
            anyhow::bail!("zai: {} — {}", status, body);
        }
        super::parse_chat_response(&body, "zai", &model)
    }
}
