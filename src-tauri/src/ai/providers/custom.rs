//! Custom providers — user-defined endpoints.
//!
//! These providers let the user plug in any AI service that speaks one of
//! the supported protocols:
//! - `CustomOpenAiProvider` — any OpenAI-compatible endpoint.
//! - `CustomAnthropicProvider` — any Anthropic Messages-compatible endpoint.
//! - `CustomOllamaProvider` — any Ollama-native endpoint (different host/port).
//! - `WebhookProvider` — generic HTTP POST webhook that returns a JSON
//!   `{"text": "..."}` field (lets users connect custom backends like n8n,
//!   Zapier, or self-hosted servers).

use std::sync::Arc;
use std::sync::RwLock;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::ai::provider::{
    ChatMessage, ChatRequest, ChatResponse, Provider, ProviderCategory, ProviderCreds,
    ProviderDescriptor, Role,
};
use crate::error::{AegisError, Result};

// ============================================================================
// CustomOpenAiProvider
// ============================================================================

pub struct CustomOpenAiProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
    inner: Arc<crate::ai::providers::openai_compat::OpenAiCompatProvider>,
}

impl CustomOpenAiProvider {
    pub fn new() -> Arc<Self> {
        let desc = crate::ai::providers::openai_compat::descriptor(
            "custom_openai",
            "Custom (OpenAI-compat)",
            "Any OpenAI-compatible endpoint (you set the base URL).",
            "#",
            ProviderCategory::Custom,
            false,
            false,
            None,
            "gpt-3.5-turbo",
            &[],
            true,
        );
        Arc::new(Self {
            descriptor: desc.clone(),
            creds: RwLock::new(ProviderCreds::default()),
            inner: Arc::new(crate::ai::providers::openai_compat::OpenAiCompatProvider::new(desc)),
        })
    }
}

#[async_trait]
impl Provider for CustomOpenAiProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }
    fn set_creds(&self, creds: ProviderCreds) {
        self.inner.set_creds(creds.clone());
        *self.creds.write().unwrap() = creds;
    }
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let base_url = self.creds.read().unwrap().base_url.clone();
        if base_url.is_none() {
            return Err(AegisError::AiNotConfigured(
                "custom OpenAI-compatible provider requires a base URL".into(),
            ));
        }
        self.inner.chat(req).await
    }
    async fn ping(&self) -> Result<()> {
        let base_url = self.creds.read().unwrap().base_url.clone();
        if base_url.is_none() {
            return Err(AegisError::AiNotConfigured(
                "custom OpenAI-compatible provider requires a base URL".into(),
            ));
        }
        self.inner.ping().await
    }
}

// ============================================================================
// CustomAnthropicProvider
// ============================================================================

pub struct CustomAnthropicProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
}

impl CustomAnthropicProvider {
    pub fn new() -> Arc<Self> {
        let desc = ProviderDescriptor {
            id: "custom_anthropic".into(),
            name: "Custom (Anthropic-compat)".into(),
            description: "Any Anthropic Messages-compatible endpoint.".into(),
            homepage: "#".into(),
            category: ProviderCategory::Custom,
            requires_api_key: false,
            local: false,
            default_base_url: None,
            default_model: "claude-3-5-sonnet-latest".into(),
            known_models: vec![],
            implemented: true,
        };
        Arc::new(Self {
            descriptor: desc,
            creds: RwLock::new(ProviderCreds::default()),
        })
    }

    fn creds(&self) -> ProviderCreds {
        self.creds.read().unwrap().clone()
    }
}

#[async_trait]
impl Provider for CustomAnthropicProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }
    fn set_creds(&self, creds: ProviderCreds) {
        *self.creds.write().unwrap() = creds;
    }
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let creds = self.creds();
        let base_url = creds.base_url.clone().ok_or_else(|| {
            AegisError::AiNotConfigured("custom Anthropic provider requires a base URL".into())
        })?;
        let model = req
            .model
            .clone()
            .or(creds.model.clone())
            .unwrap_or_else(|| self.descriptor.default_model.clone());

        let mut system_text = String::new();
        let mut messages: Vec<serde_json::Value> = vec![];
        for m in req.messages {
            match m.role {
                Role::System => {
                    if !system_text.is_empty() {
                        system_text.push_str("\n\n");
                    }
                    system_text.push_str(&m.content);
                }
                Role::User => messages.push(json!({"role": "user", "content": m.content})),
                Role::Assistant => {
                    messages.push(json!({"role": "assistant", "content": m.content}))
                }
                Role::Tool => messages.push(json!({"role": "user", "content": m.content})),
            }
        }

        let mut body = json!({
            "model": model,
            "messages": messages,
            "max_tokens": req.max_tokens.unwrap_or(2048),
        });
        if !system_text.is_empty() {
            body["system"] = json!(system_text);
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        let mut req_b = client.post(format!("{}/v1/messages", base_url)).json(&body);
        if let Some(key) = creds.api_key {
            req_b = req_b
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01");
        }
        let resp = req_b.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AegisError::Ai(format!("HTTP {status}: {text}")));
        }
        let body: serde_json::Value = resp.json().await?;
        let text = body["content"][0]["text"]
            .as_str()
            .ok_or_else(|| AegisError::Ai("unexpected response shape".into()))?
            .to_string();
        Ok(ChatResponse {
            message: ChatMessage::assistant(text),
            model,
            usage: None,
        })
    }
    async fn ping(&self) -> Result<()> {
        let creds = self.creds();
        if creds.base_url.is_none() {
            return Err(AegisError::AiNotConfigured("base URL required".into()));
        }
        Ok(())
    }
}

// ============================================================================
// CustomOllamaProvider
// ============================================================================

pub struct CustomOllamaProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
}

impl CustomOllamaProvider {
    fn creds(&self) -> ProviderCreds {
        self.creds.read().unwrap().clone()
    }

    pub fn new() -> Arc<Self> {
        let desc = ProviderDescriptor {
            id: "custom_ollama".into(),
            name: "Custom (Ollama-compat)".into(),
            description: "Any Ollama-native endpoint on a custom host/port.".into(),
            homepage: "#".into(),
            category: ProviderCategory::Custom,
            requires_api_key: false,
            local: false,
            default_base_url: None,
            default_model: "llama3.2".into(),
            known_models: vec![],
            implemented: true,
        };
        Arc::new(Self {
            descriptor: desc,
            creds: RwLock::new(ProviderCreds::default()),
        })
    }
}

#[async_trait]
impl Provider for CustomOllamaProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }
    fn set_creds(&self, creds: ProviderCreds) {
        *self.creds.write().unwrap() = creds;
    }
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let creds = self.creds();
        let base_url = creds.base_url.clone().ok_or_else(|| {
            AegisError::AiNotConfigured("custom Ollama provider requires a base URL".into())
        })?;
        let model = req
            .model
            .clone()
            .or(creds.model.clone())
            .unwrap_or_else(|| self.descriptor.default_model.clone());

        let messages: Vec<serde_json::Value> = req
            .messages
            .iter()
            .map(|m| {
                json!({
                    "role": match m.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant | Role::Tool => "assistant",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let body = json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;
        let resp = client
            .post(format!("{}/api/chat", base_url))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AegisError::Ai(format!("HTTP {status}: {text}")));
        }
        let body: serde_json::Value = resp.json().await?;
        let text = body["message"]["content"]
            .as_str()
            .ok_or_else(|| AegisError::Ai("unexpected response shape".into()))?
            .to_string();
        Ok(ChatResponse {
            message: ChatMessage::assistant(text),
            model,
            usage: None,
        })
    }
    async fn ping(&self) -> Result<()> {
        let creds = self.creds();
        if creds.base_url.is_none() {
            return Err(AegisError::AiNotConfigured("base URL required".into()));
        }
        Ok(())
    }
}

// ============================================================================
// WebhookProvider
// ============================================================================

pub struct WebhookProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
}

impl WebhookProvider {
    fn creds(&self) -> ProviderCreds {
        self.creds.read().unwrap().clone()
    }

    pub fn new() -> Arc<Self> {
        let desc = ProviderDescriptor {
            id: "webhook".into(),
            name: "Webhook (custom HTTP)".into(),
            description: "POSTs messages to a custom URL; expects {\"text\": \"...\"} response."
                .into(),
            homepage: "#".into(),
            category: ProviderCategory::Custom,
            requires_api_key: false,
            local: false,
            default_base_url: None,
            default_model: "webhook".into(),
            known_models: vec![],
            implemented: true,
        };
        Arc::new(Self {
            descriptor: desc,
            creds: RwLock::new(ProviderCreds::default()),
        })
    }
}

#[async_trait]
impl Provider for WebhookProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }
    fn set_creds(&self, creds: ProviderCreds) {
        *self.creds.write().unwrap() = creds;
    }
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let creds = self.creds();
        let url = creds.base_url.clone().ok_or_else(|| {
            AegisError::AiNotConfigured("webhook provider requires a base URL".into())
        })?;
        let last_user = req
            .messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, Role::User))
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let body = json!({"prompt": last_user, "messages": req.messages});
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        let mut req_b = client.post(&url).json(&body);
        if let Some(key) = creds.api_key {
            req_b = req_b.bearer_auth(key);
        }
        let resp = req_b.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AegisError::Ai(format!("HTTP {status}: {text}")));
        }
        let body: serde_json::Value = resp.json().await?;
        let text = body
            .get("text")
            .and_then(|v| v.as_str())
            .or_else(|| body.get("content").and_then(|v| v.as_str()))
            .ok_or_else(|| AegisError::Ai("webhook response missing 'text' field".into()))?
            .to_string();
        Ok(ChatResponse {
            message: ChatMessage::assistant(text),
            model: "webhook".into(),
            usage: None,
        })
    }
    async fn ping(&self) -> Result<()> {
        let creds = self.creds();
        if creds.base_url.is_none() {
            return Err(AegisError::AiNotConfigured("base URL required".into()));
        }
        Ok(())
    }
}
