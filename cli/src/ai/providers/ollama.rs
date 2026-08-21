//! Ollama (local LLMs) — OpenAI-compatible.

use async_trait::async_trait;
use std::time::Duration;

use crate::ai::provider::{ChatRequest, ChatResponse, Provider, ProviderConfig};

pub struct OllamaProvider {
    cfg: ProviderConfig,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(cfg: ProviderConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(600))  // local models can be slow on first call
            .build()
            .expect("reqwest");
        Self { cfg, client }
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn id(&self) -> &str { "ollama" }
    fn display_name(&self) -> &str { "Ollama (local)" }
    fn default_model(&self) -> &str { self.cfg.default_model.as_deref().unwrap_or("llama3.2") }

    async fn chat(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let base = self.cfg.base_url.as_deref().unwrap_or("http://localhost:11434/v1");
        let model = if req.model.is_empty() { self.default_model().to_string() } else { req.model.clone() };
        let payload = super::build_chat_payload(req, &model);
        let url = format!("{}/chat/completions", base.trim_end_matches('/'));
        let resp = self.client.post(&url).json(&payload).send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("ollama: {} — {}", status, body);
        }
        super::parse_chat_response(&body, "ollama", &model)
    }
}
