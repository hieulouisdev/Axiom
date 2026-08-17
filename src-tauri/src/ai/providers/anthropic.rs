//! Anthropic Claude — Claude 3.5 Sonnet, Haiku, Opus via Messages API.

use std::sync::RwLock;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::ai::provider::{
    ChatMessage, ChatRequest, ChatResponse, Provider, ProviderCategory, ProviderCreds,
    ProviderDescriptor, Role,
};
use crate::error::{AegisError, Result};

pub struct AnthropicProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
    client: Client,
}

impl AnthropicProvider {
    pub fn new() -> std::sync::Arc<Self> {
        let desc = ProviderDescriptor {
            id: "anthropic".into(),
            name: "Anthropic Claude".into(),
            description: "Claude 3.5 Sonnet / Haiku / Opus via the Messages API.".into(),
            homepage: "https://anthropic.com".into(),
            category: ProviderCategory::CloudMajor,
            requires_api_key: true,
            local: false,
            default_base_url: Some("https://api.anthropic.com".into()),
            default_model: "claude-3-5-sonnet-latest".into(),
            known_models: vec![
                "claude-3-5-sonnet-latest".into(),
                "claude-3-5-haiku-latest".into(),
                "claude-3-opus-latest".into(),
            ],
            implemented: true,
        };
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap();
        std::sync::Arc::new(Self {
            descriptor: desc,
            creds: RwLock::new(ProviderCreds::default()),
            client,
        })
    }

    fn creds(&self) -> ProviderCreds {
        self.creds.read().unwrap().clone()
    }

    fn base_url(&self) -> String {
        let c = self.creds();
        c.base_url
            .clone()
            .or_else(|| self.descriptor.default_base_url.clone())
            .unwrap_or_else(|| "https://api.anthropic.com".into())
    }

    fn api_key(&self) -> Result<String> {
        self.creds()
            .api_key
            .clone()
            .ok_or_else(|| AegisError::AiNotConfigured("Anthropic requires an API key".into()))
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    fn set_creds(&self, creds: ProviderCreds) {
        *self.creds.write().unwrap() = creds;
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let creds = self.creds();
        let model = req
            .model
            .clone()
            .or(creds.model.clone())
            .unwrap_or_else(|| self.descriptor.default_model.clone());

        let api_key = self.api_key()?;

        // Anthropic separates system message from the rest.
        let mut system_text = String::new();
        let mut user_messages: Vec<serde_json::Value> = vec![];
        for m in req.messages {
            match m.role {
                Role::System => {
                    if !system_text.is_empty() {
                        system_text.push_str("\n\n");
                    }
                    system_text.push_str(&m.content);
                }
                Role::User => user_messages.push(json!({"role": "user", "content": m.content})),
                Role::Assistant => user_messages.push(json!({"role": "assistant", "content": m.content})),
                Role::Tool => user_messages.push(json!({"role": "user", "content": m.content})),
            }
        }

        let mut body = json!({
            "model": model,
            "messages": user_messages,
            "max_tokens": req.max_tokens.unwrap_or(2048),
            "temperature": req.temperature.unwrap_or(1.0),
        });
        if !system_text.is_empty() {
            body["system"] = json!(system_text);
        }

        let url = format!("{}/v1/messages", self.base_url());
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AegisError::Ai(format!("HTTP {status} from anthropic: {text}")));
        }

        let body: serde_json::Value = resp.json().await?;
        let text = body["content"][0]["text"]
            .as_str()
            .ok_or_else(|| AegisError::Ai("unexpected anthropic response shape".into()))?
            .to_string();
        let model_used = body["model"].as_str().unwrap_or(&model).to_string();

        Ok(ChatResponse {
            message: ChatMessage::assistant(text),
            model: model_used,
            usage: None,
        })
    }

    async fn ping(&self) -> Result<()> {
        let req = ChatRequest {
            messages: vec![ChatMessage::user("ping")],
            model: None,
            temperature: Some(0.0),
            max_tokens: Some(1),
            top_p: None,
            stop: vec![],
            extra: Default::default(),
        };
        self.chat(req).await.map(|_| ())
    }
}
