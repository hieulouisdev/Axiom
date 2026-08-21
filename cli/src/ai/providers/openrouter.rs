//! OpenRouter — OpenAI-compatible aggregator with hundreds of models.

use async_trait::async_trait;
use std::time::Duration;

use crate::ai::provider::{ChatRequest, ChatResponse, Provider, ProviderConfig};

pub struct OpenRouterProvider {
    cfg: ProviderConfig,
    client: reqwest::Client,
}

impl OpenRouterProvider {
    pub fn new(cfg: ProviderConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest");
        Self { cfg, client }
    }
}

#[async_trait]
impl Provider for OpenRouterProvider {
    fn id(&self) -> &str { "openrouter" }
    fn display_name(&self) -> &str { "OpenRouter" }
    fn default_model(&self) -> &str { self.cfg.default_model.as_deref().unwrap_or("openai/gpt-4o-mini") }

    async fn chat(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let key = self.cfg.api_key.as_deref().ok_or_else(|| anyhow::anyhow!("openrouter: missing api_key"))?;
        let base = self.cfg.base_url.as_deref().unwrap_or("https://openrouter.ai/api/v1");
        let model = if req.model.is_empty() { self.default_model().to_string() } else { req.model.clone() };
        let payload = super::build_chat_payload(req, &model);
        let url = format!("{}/chat/completions", base.trim_end_matches('/'));
        let resp = self.client
            .post(&url)
            .bearer_auth(key)
            .header("HTTP-Referer", "https://github.com/hieulouisdev/Axiom")
            .header("X-Title", "Aegis AI CLI")
            .json(&payload)
            .send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("openrouter: {} — {}", status, body);
        }
        super::parse_chat_response(&body, "openrouter", &model)
    }
}
