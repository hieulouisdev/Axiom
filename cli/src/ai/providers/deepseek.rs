//! DeepSeek — OpenAI-compatible.

use async_trait::async_trait;
use std::time::Duration;

use crate::ai::provider::{ChatRequest, ChatResponse, Provider, ProviderConfig};

pub struct DeepSeekProvider {
    cfg: ProviderConfig,
    client: reqwest::Client,
}

impl DeepSeekProvider {
    pub fn new(cfg: ProviderConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest");
        Self { cfg, client }
    }
}

#[async_trait]
impl Provider for DeepSeekProvider {
    fn id(&self) -> &str { "deepseek" }
    fn display_name(&self) -> &str { "DeepSeek" }
    fn default_model(&self) -> &str { self.cfg.default_model.as_deref().unwrap_or("deepseek-chat") }

    async fn chat(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let key = self.cfg.api_key.as_deref().ok_or_else(|| anyhow::anyhow!("deepseek: missing api_key"))?;
        let base = self.cfg.base_url.as_deref().unwrap_or("https://api.deepseek.com/v1");
        let model = if req.model.is_empty() { self.default_model().to_string() } else { req.model.clone() };
        let payload = super::build_chat_payload(req, &model);
        let url = format!("{}/chat/completions", base.trim_end_matches('/'));
        let resp = self.client.post(&url).bearer_auth(key).json(&payload).send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("deepseek: {} — {}", status, body);
        }
        super::parse_chat_response(&body, "deepseek", &model)
    }
}
