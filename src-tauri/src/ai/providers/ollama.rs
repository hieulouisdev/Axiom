//! Ollama — local LLM runtime (Llama, Mistral, Qwen, etc.).
//!
//! Uses Ollama's native `/api/chat` endpoint. Also supports the
//! OpenAI-compatible `/v1/chat/completions` endpoint exposed since 0.1.x.

use std::sync::RwLock;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::ai::provider::{
    ChatMessage, ChatRequest, ChatResponse, Provider, ProviderCategory, ProviderCreds,
    ProviderDescriptor, Role,
};
use crate::error::{AegisError, Result};

pub struct OllamaProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
    client: Client,
}

impl OllamaProvider {
    pub fn new() -> std::sync::Arc<Self> {
        let desc = ProviderDescriptor {
            id: "ollama".into(),
            name: "Ollama".into(),
            description: "Local LLM runtime (Llama, Mistral, Qwen, Gemma). No API key required."
                .into(),
            homepage: "https://ollama.com".into(),
            category: ProviderCategory::Local,
            requires_api_key: false,
            local: true,
            default_base_url: Some("http://localhost:11434".into()),
            default_model: "llama3.2".into(),
            known_models: vec![
                "llama3.2".into(),
                "llama3.1".into(),
                "mistral".into(),
                "qwen2.5".into(),
                "gemma2".into(),
                "deepseek-r1".into(),
            ],
            implemented: true,
        };
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
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
            .unwrap_or_else(|| "http://localhost:11434".into())
    }
}

#[async_trait]
impl Provider for OllamaProvider {
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

        let mut system_text = String::new();
        let mut messages: Vec<serde_json::Value> = vec![];
        for m in req.messages {
            let role = match m.role {
                Role::System => {
                    if !system_text.is_empty() {
                        system_text.push_str("\n\n");
                    }
                    system_text.push_str(&m.content);
                    continue;
                }
                Role::User => "user",
                Role::Assistant | Role::Tool => "assistant",
            };
            messages.push(json!({"role": role, "content": m.content}));
        }

        let mut body = json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": req.temperature.unwrap_or(0.7),
                "num_predict": req.max_tokens.unwrap_or(2048),
                "top_p": req.top_p.unwrap_or(0.9),
            }
        });
        if !system_text.is_empty() {
            if let Some(opts) = body.get_mut("options") {
                opts["system"] = json!(system_text);
            }
        }

        let url = format!("{}/api/chat", self.base_url());
        let resp = self.client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AegisError::Ai(format!("HTTP {status} from ollama: {text}")));
        }

        let body: serde_json::Value = resp.json().await?;
        let text = body["message"]["content"]
            .as_str()
            .ok_or_else(|| AegisError::Ai("unexpected ollama response shape".into()))?
            .to_string();

        Ok(ChatResponse {
            message: ChatMessage::assistant(text),
            model,
            usage: None,
        })
    }

    async fn ping(&self) -> Result<()> {
        let url = format!("{}/api/tags", self.base_url());
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(AegisError::Ai(format!(
                "ollama not reachable at {} (HTTP {})",
                self.base_url(),
                resp.status()
            )));
        }
        Ok(())
    }
}
