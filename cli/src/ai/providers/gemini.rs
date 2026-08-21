//! Google Gemini API.

use async_trait::async_trait;
use std::time::Duration;

use crate::ai::provider::{ChatRequest, ChatResponse, Provider, ProviderConfig};

pub struct GeminiProvider {
    cfg: ProviderConfig,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(cfg: ProviderConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest");
        Self { cfg, client }
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn id(&self) -> &str { "gemini" }
    fn display_name(&self) -> &str { "Google Gemini" }
    fn default_model(&self) -> &str { self.cfg.default_model.as_deref().unwrap_or("gemini-1.5-flash") }

    async fn chat(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let key = self.cfg.api_key.as_deref().ok_or_else(|| anyhow::anyhow!("gemini: missing api_key"))?;
        let base = self.cfg.base_url.as_deref().unwrap_or("https://generativelanguage.googleapis.com");
        let model = if req.model.is_empty() { self.default_model().to_string() } else { req.model.clone() };
        let url = format!("{}/v1beta/models/{}:generateContent?key={}",
            base.trim_end_matches('/'), model, key);

        // Gemini: separate system_instruction from contents.
        let mut system_text = String::new();
        let mut contents: Vec<serde_json::Value> = Vec::new();
        for m in &req.messages {
            match m.role {
                crate::ai::Role::System => {
                    if !system_text.is_empty() { system_text.push('\n'); }
                    system_text.push_str(&m.content);
                }
                crate::ai::Role::User => contents.push(serde_json::json!({"role": "user", "parts": [{"text": m.content}]})),
                crate::ai::Role::Assistant => contents.push(serde_json::json!({"role": "model", "parts": [{"text": m.content}]})),
            }
        }
        let mut payload = serde_json::json!({"contents": contents});
        if !system_text.is_empty() {
            payload["system_instruction"] = serde_json::json!({"parts": [{"text": system_text}]});
        }
        if let Some(t) = req.temperature {
            payload["generationConfig"] = serde_json::json!({"temperature": t});
        }
        if let Some(m) = req.max_tokens {
            payload["generationConfig"] = serde_json::json!({"maxOutputTokens": m});
        }
        let resp = self.client.post(&url).json(&payload).send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("gemini: {} — {}", status, body);
        }
        let v: serde_json::Value = serde_json::from_str(&body)?;
        let content = v["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("gemini: missing candidates[0].content.parts[0].text"))?
            .to_string();
        Ok(ChatResponse { content, model, provider: "gemini".into(), usage: None })
    }
}
