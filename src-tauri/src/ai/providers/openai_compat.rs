//! Shared OpenAI-compatible chat completions client.
//!
//! Many providers (OpenAI itself, DeepSeek, Groq, OpenRouter, Mistral, Together,
//! Anyscale, Moonshot, Zhipu, Yi, DeepInfra, Fireworks, LM Studio, LocalAI,
//! llama.cpp, GPT4All, Jan, KoboldCpp, vLLM, Llamafile) expose the same
//! `/v1/chat/completions` JSON shape, so we share the HTTP code via this module.

use parking_lot::RwLock;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use crate::error::{AegisError, Result};

use super::super::provider::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, Provider, ProviderCategory,
    ProviderCreds, ProviderDescriptor, Role, Usage,
};

/// Shared state for every OpenAI-compatible provider.
pub struct OpenAiCompatProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
    client: Client,
}

impl OpenAiCompatProvider {
    pub fn new(descriptor: ProviderDescriptor) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("reqwest client");
        Self {
            descriptor,
            creds: RwLock::new(ProviderCreds::default()),
            client,
        }
    }

    fn creds(&self) -> ProviderCreds {
        self.creds.read().clone()
    }

    fn base_url(&self) -> String {
        let c = self.creds();
        c.base_url
            .clone()
            .or_else(|| self.descriptor.default_base_url.clone())
            .unwrap_or_else(|| "https://api.openai.com/v1".into())
    }

    /// Returns the API key if the provider needs one.
    /// Local providers (Ollama, LM Studio, …) do not require an API key
    /// and will return `None`.
    fn maybe_api_key(&self) -> Option<String> {
        if self.descriptor.requires_api_key {
            self.creds().api_key.clone()
        } else {
            None
        }
    }

    async fn do_chat(&self, req: ChatRequest, stream: bool) -> Result<reqwest::Response> {
        let creds = self.creds();
        let model = req
            .model
            .clone()
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
            "top_p": req.top_p.unwrap_or(1.0),
            "stop": req.stop,
            "stream": stream,
        });

        let url = format!("{}/chat/completions", self.base_url());

        if self.descriptor.requires_api_key {
            let api_key = self.maybe_api_key().ok_or_else(|| {
                AegisError::AiNotConfigured(format!(
                    "provider '{}' requires an API key",
                    self.descriptor.id
                ))
            })?;
            let resp = self
                .client
                .post(&url)
                .bearer_auth(&api_key)
                .json(&body)
                .send()
                .await?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(AegisError::Ai(format!(
                    "HTTP {status} from {}: {text}",
                    self.descriptor.id
                )));
            }
            Ok(resp)
        } else {
            let resp = self.client.post(&url).json(&body).send().await?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(AegisError::Ai(format!(
                    "HTTP {status} from {}: {text}",
                    self.descriptor.id
                )));
            }
            Ok(resp)
        }
    }
}

#[async_trait]
impl Provider for OpenAiCompatProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    fn set_creds(&self, creds: ProviderCreds) {
        *self.creds.write() = creds;
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let resp = self.do_chat(req, false).await?;
        let body: OpenAiChatResponse = resp.json().await?;
        let choice = body
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AegisError::Ai("empty choices in response".into()))?;
        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: choice.message.content,
                name: None,
                tool_calls: choice.message.tool_calls,
            },
            model: body.model,
            usage: body.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }

    async fn chat_stream(
        &self,
        req: ChatRequest,
        on_chunk: Box<dyn Fn(ChatStreamChunk) + Send + Sync>,
    ) -> Result<ChatResponse> {
        let resp = self.do_chat(req, true).await?;
        let mut full = String::new();
        let mut model = String::new();
        let mut final_usage: Option<Usage> = None;
        let mut buf = String::new();

        let mut stream = resp.bytes_stream();
        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buf.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(idx) = buf.find('\n') {
                let line: String = buf.drain(..=idx).collect();
                let line = line.trim();
                if line.is_empty() || !line.starts_with("data:") {
                    continue;
                }
                let data = line[5..].trim_start();
                if data == "[DONE]" {
                    on_chunk(ChatStreamChunk {
                        delta: String::new(),
                        done: true,
                    });
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<OpenAiChatResponse>(data) {
                    if model.is_empty() {
                        model = parsed.model.clone();
                    }
                    if let Some(choice) = parsed.choices.first()
                        && let Some(delta) = choice.delta.as_ref()
                        && !delta.content.is_empty()
                    {
                        full.push_str(&delta.content);
                        on_chunk(ChatStreamChunk {
                            delta: delta.content.clone(),
                            done: false,
                        });
                    }
                    if let Some(u) = parsed.usage {
                        final_usage = Some(Usage {
                            prompt_tokens: u.prompt_tokens,
                            completion_tokens: u.completion_tokens,
                            total_tokens: u.total_tokens,
                        });
                    }
                }
            }
        }

        Ok(ChatResponse {
            message: ChatMessage::assistant(full),
            model,
            usage: final_usage,
        })
    }

    async fn ping(&self) -> Result<()> {
        // A minimal chat request with max_tokens=1 to verify credentials.
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

/// Parse an SSE stream from an OpenAI-compatible API.
/// Public so other providers (e.g. Azure OpenAI) can reuse it.
pub async fn parse_sse_stream(
    resp: reqwest::Response,
    on_chunk: Box<dyn Fn(ChatStreamChunk) + Send + Sync>,
) -> Result<ChatResponse> {
    let mut full = String::new();
    let mut model = String::new();
    let mut final_usage: Option<Usage> = None;
    let mut buf = String::new();

    let mut stream = resp.bytes_stream();
    use futures::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(idx) = buf.find('\n') {
            let line: String = buf.drain(..=idx).collect();
            let line = line.trim();
            if line.is_empty() || !line.starts_with("data:") {
                continue;
            }
            let data = line[5..].trim_start();
            if data == "[DONE]" {
                on_chunk(ChatStreamChunk {
                    delta: String::new(),
                    done: true,
                });
                continue;
            }
            if let Ok(parsed) = serde_json::from_str::<OpenAiChatResponse>(data) {
                if model.is_empty() {
                    model = parsed.model.clone();
                }
                if let Some(choice) = parsed.choices.first()
                    && let Some(delta) = choice.delta.as_ref()
                    && !delta.content.is_empty()
                {
                    full.push_str(&delta.content);
                    on_chunk(ChatStreamChunk {
                        delta: delta.content.clone(),
                        done: false,
                    });
                }
                if let Some(u) = parsed.usage {
                    final_usage = Some(Usage {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                        total_tokens: u.total_tokens,
                    });
                }
            }
        }
    }

    Ok(ChatResponse {
        message: ChatMessage::assistant(full),
        model,
        usage: final_usage,
    })
}

#[derive(Debug, Deserialize)]
pub struct OpenAiChatResponse {
    pub model: String,
    pub choices: Vec<OpenAiChoice>,
    #[serde(default)]
    pub usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiChoice {
    #[serde(default)]
    pub message: OpenAiMessage,
    #[serde(default)]
    pub delta: Option<OpenAiMessage>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenAiMessage {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tool_calls: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiUsage {
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub completion_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
}

/// Helper to declare a provider descriptor.
pub fn descriptor(
    id: &str,
    name: &str,
    description: &str,
    homepage: &str,
    category: ProviderCategory,
    requires_api_key: bool,
    local: bool,
    default_base_url: Option<&str>,
    default_model: &str,
    known_models: &[&str],
    implemented: bool,
) -> ProviderDescriptor {
    ProviderDescriptor {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        homepage: homepage.into(),
        category,
        requires_api_key,
        local,
        default_base_url: default_base_url.map(Into::into),
        default_model: default_model.into(),
        known_models: known_models.iter().map(|s| s.to_string()).collect(),
        implemented,
    }
}

/// One-liner constructor that wraps a descriptor into a live
/// OpenAI-compatible [`Provider`] trait object.
///
/// Used by every thin provider wrapper (`deepseek`, `groq`, `mistral`, …).
pub fn make(desc: ProviderDescriptor) -> std::sync::Arc<dyn Provider> {
    std::sync::Arc::new(OpenAiCompatProvider::new(desc))
}

/// Stubbed provider: returns "not implemented" for every call but still
/// advertises itself in the registry so the UI can list it.
pub struct StubProvider {
    descriptor: ProviderDescriptor,
}

impl StubProvider {
    pub fn new(descriptor: ProviderDescriptor) -> Self {
        Self { descriptor }
    }
}

#[async_trait]
impl Provider for StubProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }
    fn set_creds(&self, _creds: ProviderCreds) {}
    async fn chat(&self, _req: ChatRequest) -> Result<ChatResponse> {
        Err(AegisError::Ai(format!(
            "provider '{}' is not yet implemented in v0.1 (see ROADMAP Phase 2)",
            self.descriptor.id
        )))
    }
    async fn ping(&self) -> Result<()> {
        Err(AegisError::Ai(format!(
            "provider '{}' is not yet implemented in v0.1",
            self.descriptor.id
        )))
    }
}
