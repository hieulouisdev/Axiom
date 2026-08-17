//! Core AI provider abstractions.
//!
//! Every AI backend (OpenAI, Anthropic, Gemini, Ollama, …) implements the
//! [`Provider`] trait so the rest of Aegis AI can talk to them uniformly.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{AegisError, Result};

/// Role of a single chat message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A single message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    /// Optional name (for tool calls / function calling).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional tool call payload (provider-specific JSON).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<serde_json::Value>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: Role::System, content: content.into(), name: None, tool_calls: None }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into(), name: None, tool_calls: None }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: content.into(), name: None, tool_calls: None }
    }
}

/// A request to a chat-completion provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    /// Optional model override (falls back to the provider's default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Sampling temperature (0.0 = deterministic, 1.0 = creative).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens to generate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Top-p nucleus sampling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Stop sequences.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    /// Provider-specific extra params.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// A response from a chat-completion provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub model: String,
    /// Tokens used (if reported by the provider).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// A streaming chat token (one chunk of the assistant's reply).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamChunk {
    pub delta: String,
    pub done: bool,
}

/// Metadata describing a provider (for UI + routing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDescriptor {
    /// Stable unique id, e.g. "openai".
    pub id: String,
    /// Human-readable name, e.g. "OpenAI".
    pub name: String,
    /// Short description for the settings UI.
    pub description: String,
    /// Homepage URL.
    pub homepage: String,
    /// Provider category.
    pub category: ProviderCategory,
    /// Whether the provider requires an API key.
    pub requires_api_key: bool,
    /// Whether the provider runs locally.
    pub local: bool,
    /// Default base URL (None = use provider's official endpoint).
    pub default_base_url: Option<String>,
    /// Default model.
    pub default_model: String,
    /// List of well-known models the provider offers.
    pub known_models: Vec<String>,
    /// Whether the trait implementation actually issues real requests.
    pub implemented: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snakecase")]
pub enum ProviderCategory {
    CloudMajor,
    CloudOther,
    Local,
    Custom,
}

/// Per-provider credentials (passed from config to the provider on every call).
#[derive(Debug, Clone, Default)]
pub struct ProviderCreds {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub extra: BTreeMap<String, String>,
}

/// The trait every AI backend implements.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Static descriptor (id, name, category, default model, …).
    fn descriptor(&self) -> &ProviderDescriptor;

    /// Update the credentials used by this provider at runtime.
    fn set_creds(&self, creds: ProviderCreds);

    /// Non-streaming chat completion.
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse>;

    /// Streaming chat completion. Default returns `unsupported`.
    async fn chat_stream(
        &self,
        _req: ChatRequest,
        _on_chunk: Box<dyn Fn(ChatStreamChunk) + Send + Sync>,
    ) -> Result<ChatResponse> {
        Err(AegisError::Ai("streaming not supported by this provider".into()))
    }

    /// Lightweight connectivity test.
    async fn ping(&self) -> Result<()> {
        Ok(())
    }
}

/// Registry of all known providers (live instances + descriptors).
pub struct ProviderRegistry {
    pub providers: BTreeMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    /// Returns a registry pre-populated with every builtin provider.
    /// Each provider starts in an unconfigured state; credentials are
    /// supplied later via [`Provider::set_creds`].
    pub fn with_builtin() -> Self {
        use providers::*;

        let mut map: BTreeMap<String, Arc<dyn Provider>> = BTreeMap::new();

        // Cloud-major
        register(&mut map, OpenAiProvider::new());
        register(&mut map, AnthropicProvider::new());
        register(&mut map, GeminiProvider::new());
        register(&mut map, DeepSeekProvider::new());
        register(&mut map, GroqProvider::new());
        register(&mut map, OpenRouterProvider::new());
        register(&mut map, MistralProvider::new());
        register(&mut map, CohereProvider::new());
        register(&mut map, TogetherProvider::new());
        register(&mut map, AnyscaleProvider::new());

        // Local
        register(&mut map, OllamaProvider::new());
        register(&mut map, LmStudioProvider::new());
        register(&mut map, LocalAiProvider::new());
        register(&mut map, LlamaCppProvider::new());
        register(&mut map, Gpt4AllProvider::new());
        register(&mut map, JanProvider::new());
        register(&mut map, KoboldCppProvider::new());
        register(&mut map, VllmProvider::new());
        register(&mut map, LlamafileProvider::new());

        // Cloud-other
        register(&mut map, AzureOpenAiProvider::new());
        register(&mut map, BedrockProvider::new());
        register(&mut map, HuggingFaceProvider::new());
        register(&mut map, ReplicateProvider::new());
        register(&mut map, MoonshotProvider::new());
        register(&mut map, ZhipuProvider::new());
        register(&mut map, YiProvider::new());
        register(&mut map, DeepInfraProvider::new());
        register(&mut map, FireworksProvider::new());

        // Custom
        register(&mut map, CustomOpenAiProvider::new());
        register(&mut map, CustomAnthropicProvider::new());
        register(&mut map, CustomOllamaProvider::new());
        register(&mut map, WebhookProvider::new());

        Self { providers: map }
    }

    pub fn list(&self) -> Vec<ProviderDescriptor> {
        self.providers.values().map(|p| p.descriptor().clone()).collect()
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(id).cloned()
    }
}

fn register(map: &mut BTreeMap<String, Arc<dyn Provider>>, p: Arc<dyn Provider>) {
    let id = p.descriptor().id.clone();
    map.insert(id, p);
}
