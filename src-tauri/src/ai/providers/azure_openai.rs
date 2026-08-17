//! Azure OpenAI Service — enterprise OpenAI deployment.
//!
//! Phase 2: Full implementation with Azure-specific auth (API key with
//! `api-key` header), deployment-id URL routing, and api-version query param.

use std::collections::BTreeMap;
use std::sync::RwLock;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::error::{AegisError, Result};
use crate::ai::provider::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, Provider,
    ProviderCategory, ProviderCreds, ProviderDescriptor, Role, Usage,
};

pub struct AzureOpenAiProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
    client: Client,
}

impl AzureOpenAiProvider {
    pub fn new() -> std::sync::Arc<dyn Provider> {
        let desc = ProviderDescriptor {
            id: "azure_openai".into(),
            name: "Azure OpenAI".into(),
            description: "Enterprise OpenAI deployment via Azure (AAD / API key).".into(),
            homepage: "https://azure.microsoft.com/products/ai-services/openai-service".into(),
            category: ProviderCategory::CloudOther,
            requires_api_key: true,
            local: false,
            default_base_url: None,
            default_model: "gpt-4o".into(),
            known_models: vec!["gpt-4o".into(), "gpt-4".into(), "gpt-35-turbo".into()],
            implemented: true,
        };
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("reqwest client");
        std::sync::Arc::new(Self {
            descriptor: desc,
            creds: RwLock::new(ProviderCreds::default()),
            client,
        })
    }

    fn creds(&self) -> ProviderCreds {
        self.creds.read().unwrap().clone()
    }

    /// Build the Azure OpenAI URL:
    /// `https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}`
    fn build_url(&self, creds: &ProviderCreds) -> Result<String> {
        let base_url = creds.base_url.clone()
            .ok_or_else(|| AegisError::AiNotConfigured(
                "azure_openai requires base_url in format: https://{resource}.openai.azure.com".into()
            ))?;

        let deployment_id = creds.extra.get("deployment_id")
            .cloned()
            .ok_or_else(|| AegisError::AiNotConfigured(
                "azure_openai requires 'deployment_id' in extra config".into()
            ))?;

        let api_version = creds.extra.get("api_version")
            .cloned()
            .unwrap_or_else(|| "2024-06-01".into());

        // Ensure base_url doesn't have trailing slash
        let base_url = base_url.trim_end_matches('/');

        Ok(format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            base_url, deployment_id, api_version
        ))
    }
}

#[async_trait]
impl Provider for AzureOpenAiProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    fn set_creds(&self, creds: ProviderCreds) {
        *self.creds.write().unwrap() = creds;
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let creds = self.creds();
        let url = self.build_url(&creds)?;
        let api_key = creds.api_key.clone()
            .ok_or_else(|| AegisError::AiNotConfigured(
                "azure_openai requires an API key".into()
            ))?;

        let model = req.model.clone()
            .or(creds.model.clone())
            .unwrap_or_else(|| self.descriptor.default_model.clone());

        let body = json!({
            "model": model,
            "messages": req.messages.iter().map(|m| json!({
                "role": match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens.unwrap_or(2048),
        });

        let resp = self.client
            .post(&url)
            .header("api-key", &api_key)  // Azure uses "api-key" header, not Bearer
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AegisError::Ai(format!("HTTP {status} from azure_openai: {text}")));
        }

        let resp_body: serde_json::Value = resp.json().await?;
        parse_openai_response(&resp_body)
    }

    async fn chat_stream(
        &self,
        req: ChatRequest,
        on_chunk: Box<dyn Fn(ChatStreamChunk) + Send + Sync>,
    ) -> Result<ChatResponse> {
        let creds = self.creds();
        let url = self.build_url(&creds)?;
        let api_key = creds.api_key.clone()
            .ok_or_else(|| AegisError::AiNotConfigured(
                "azure_openai requires an API key".into()
            ))?;

        let model = req.model.clone()
            .or(creds.model.clone())
            .unwrap_or_else(|| self.descriptor.default_model.clone());

        let body = json!({
            "model": model,
            "messages": req.messages.iter().map(|m| json!({
                "role": match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens.unwrap_or(2048),
            "stream": true,
        });

        let resp = self.client
            .post(&url)
            .header("api-key", &api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AegisError::Ai(format!("HTTP {status} from azure_openai: {text}")));
        }

        crate::ai::providers::openai_compat::parse_sse_stream(resp, on_chunk).await
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

fn parse_openai_response(body: &serde_json::Value) -> Result<ChatResponse> {
    let model = body["model"].as_str().unwrap_or("azure_openai").to_string();
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let usage = body.get("usage").and_then(|u| {
        Some(Usage {
            prompt_tokens: u["prompt_tokens"].as_u64()? as u32,
            completion_tokens: u["completion_tokens"].as_u64()? as u32,
            total_tokens: u["total_tokens"].as_u64()? as u32,
        })
    });

    Ok(ChatResponse {
        message: ChatMessage::assistant(content),
        model,
        usage,
    })
}
