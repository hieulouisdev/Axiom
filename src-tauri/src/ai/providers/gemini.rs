//! Google Gemini — Gemini 1.5 Pro / Flash via generateContent API.

use parking_lot::RwLock;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::ai::provider::{
    ChatMessage, ChatRequest, ChatResponse, Provider, ProviderCategory, ProviderCreds,
    ProviderDescriptor, Role,
};
use crate::error::{AegisError, Result};

pub struct GeminiProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
    client: Client,
}

impl GeminiProvider {
    pub fn new() -> std::sync::Arc<Self> {
        let desc = ProviderDescriptor {
            id: "gemini".into(),
            name: "Google Gemini".into(),
            description: "Gemini 1.5 Pro / Flash via Google AI Studio.".into(),
            homepage: "https://ai.google.dev".into(),
            category: ProviderCategory::CloudMajor,
            requires_api_key: true,
            local: false,
            default_base_url: Some("https://generativelanguage.googleapis.com".into()),
            default_model: "gemini-1.5-flash".into(),
            known_models: vec![
                "gemini-1.5-pro".into(),
                "gemini-1.5-flash".into(),
                "gemini-1.5-flash-8b".into(),
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
        self.creds.read().clone()
    }

    fn base_url(&self) -> String {
        let c = self.creds();
        c.base_url
            .clone()
            .or_else(|| self.descriptor.default_base_url.clone())
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com".into())
    }

    fn api_key(&self) -> Result<String> {
        self.creds()
            .api_key
            .clone()
            .ok_or_else(|| AegisError::AiNotConfigured("Gemini requires an API key".into()))
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }
    fn set_creds(&self, creds: ProviderCreds) {
        *self.creds.write() = creds;
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let creds = self.creds();
        let model = req
            .model
            .clone()
            .or(creds.model.clone())
            .unwrap_or_else(|| self.descriptor.default_model.clone());

        let api_key = self.api_key()?;

        let mut system_text = String::new();
        let mut contents: Vec<serde_json::Value> = vec![];
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
                Role::Assistant | Role::Tool => "model",
            };
            contents.push(json!({
                "role": role,
                "parts": [{"text": m.content}],
            }));
        }

        let mut body = json!({
            "contents": contents,
            "generationConfig": {
                "temperature": req.temperature.unwrap_or(1.0),
                "maxOutputTokens": req.max_tokens.unwrap_or(2048),
                "topP": req.top_p.unwrap_or(1.0),
            }
        });
        if !system_text.is_empty() {
            body["systemInstruction"] = json!({"parts": [{"text": system_text}]});
        }

        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url(),
            model,
            api_key
        );

        let resp = self.client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AegisError::Ai(format!("HTTP {status} from gemini: {text}")));
        }
        let body: serde_json::Value = resp.json().await?;
        let text = body["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| AegisError::Ai("unexpected gemini response shape".into()))?
            .to_string();

        Ok(ChatResponse {
            message: ChatMessage::assistant(text),
            model,
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
