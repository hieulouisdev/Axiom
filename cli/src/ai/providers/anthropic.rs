//! Anthropic Messages API.

use async_trait::async_trait;
use std::time::Duration;

use crate::ai::provider::{ChatRequest, ChatResponse, Provider, ProviderConfig};

pub struct AnthropicProvider {
    cfg: ProviderConfig,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(cfg: ProviderConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest");
        Self { cfg, client }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn id(&self) -> &str { "anthropic" }
    fn display_name(&self) -> &str { "Anthropic" }
    fn default_model(&self) -> &str { self.cfg.default_model.as_deref().unwrap_or("claude-3-5-sonnet-latest") }

    async fn chat(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let key = self.cfg.api_key.as_deref().ok_or_else(|| anyhow::anyhow!("anthropic: missing api_key"))?;
        let base = self.cfg.base_url.as_deref().unwrap_or("https://api.anthropic.com");
        let model = if req.model.is_empty() { self.default_model().to_string() } else { req.model.clone() };
        // Anthropic: separate system message from user/assistant turns.
        let mut system_text = String::new();
        let mut turns: Vec<serde_json::Value> = Vec::new();
        for m in &req.messages {
            match m.role {
                crate::ai::Role::System => {
                    if !system_text.is_empty() { system_text.push('\n'); }
                    system_text.push_str(&m.content);
                }
                crate::ai::Role::User => turns.push(serde_json::json!({"role": "user", "content": m.content})),
                crate::ai::Role::Assistant => turns.push(serde_json::json!({"role": "assistant", "content": m.content})),
            }
        }
        let mut payload = serde_json::json!({
            "model": model,
            "messages": turns,
            "max_tokens": req.max_tokens.unwrap_or(2048),
        });
        if !system_text.is_empty() {
            payload["system"] = serde_json::json!(system_text);
        }
        if let Some(t) = req.temperature {
            payload["temperature"] = serde_json::json!(t);
        }
        let url = format!("{}/v1/messages", base.trim_end_matches('/'));
        let resp = self.client
            .post(&url)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("anthropic: {} — {}", status, body);
        }
        let v: serde_json::Value = serde_json::from_str(&body)?;
        let content = v["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("anthropic: missing content[0].text"))?
            .to_string();
        let usage = if v.get("usage").is_some() {
            Some(crate::ai::Usage {
                prompt_tokens: v["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: v["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: v["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32
                    + v["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
            })
        } else { None };
        Ok(ChatResponse { content, model, provider: "anthropic".into(), usage })
    }
}
