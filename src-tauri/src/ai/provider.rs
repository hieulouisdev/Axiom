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
        Self {
            role: Role::System,
            content: content.into(),
            name: None,
            tool_calls: None,
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            name: None,
            tool_calls: None,
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            name: None,
            tool_calls: None,
        }
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
#[serde(rename_all = "snake_case")]
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
        Err(AegisError::Ai(
            "streaming not supported by this provider".into(),
        ))
    }

    /// Lightweight connectivity test.
    async fn ping(&self) -> Result<()> {
        Ok(())
    }
}

/// Registry of all known providers (live instances + descriptors).
#[derive(Clone)]
pub struct ProviderRegistry {
    pub providers: BTreeMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    /// Returns a registry pre-populated with every builtin provider.
    /// Each provider starts in an unconfigured state; credentials are
    /// supplied later via [`Provider::set_creds`].
    ///
    /// v0.3: Aegis Cloud is registered FIRST so that when its preconfigured
    /// API key is available, it is automatically the active provider.
    pub fn with_builtin() -> Self {
        use super::providers::*;

        let mut map: BTreeMap<String, Arc<dyn Provider>> = BTreeMap::new();

        // Built-in zero-config provider (Z.AI GLM, env-var key).
        register(&mut map, AegisCloudProvider::new());

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

        // === v0.4 generated providers registration (auto-generated) ===
        register(&mut map, Ai302Provider::new());
        register(&mut map, AbacusProvider::new());
        register(&mut map, AbliterationAiProvider::new());
        register(&mut map, AihubmixProvider::new());
        register(&mut map, AlibabaCnProvider::new());
        register(&mut map, AmbientProvider::new());
        register(&mut map, ApiAirforceProvider::new());
        register(&mut map, AvianProvider::new());
        register(&mut map, BasetenProvider::new());
        register(&mut map, BergetProvider::new());
        register(&mut map, CerebrasProvider::new());
        register(&mut map, ChutesProvider::new());
        register(&mut map, CortecsProvider::new());
        register(&mut map, CrofProvider::new());
        register(&mut map, EmpiriolabsProvider::new());
        register(&mut map, FastrouterProvider::new());
        register(&mut map, FriendliProvider::new());
        register(&mut map, GithubCopilotProvider::new());
        register(&mut map, HeliconeProvider::new());
        register(&mut map, HyperProvider::new());
        register(&mut map, ImpossiblProvider::new());
        register(&mut map, InceptionProvider::new());
        register(&mut map, InceptronProvider::new());
        register(&mut map, IoNetProvider::new());
        register(&mut map, JiekouProvider::new());
        register(&mut map, KenariProvider::new());
        register(&mut map, KiloProvider::new());
        register(&mut map, LlmgatewayProvider::new());
        register(&mut map, LlmtrProvider::new());
        register(&mut map, MoarkProvider::new());
        register(&mut map, ModelscopeProvider::new());
        register(&mut map, NanoGptProvider::new());
        register(&mut map, NearaiProvider::new());
        register(&mut map, NeuralwattProvider::new());
        register(&mut map, NovitaProvider::new());
        register(&mut map, NvidiaProvider::new());
        register(&mut map, OfoxProvider::new());
        register(&mut map, OllamaCloudProvider::new());
        register(&mut map, OpencodeZenProvider::new());
        register(&mut map, OrcarouterProvider::new());
        register(&mut map, OvhcloudProvider::new());
        register(&mut map, PerplexityProvider::new());
        register(&mut map, PioneerProvider::new());
        register(&mut map, PoeProvider::new());
        register(&mut map, QiniuProvider::new());
        register(&mut map, QuiverProvider::new());
        register(&mut map, RequestyProvider::new());
        register(&mut map, RoutingRunProvider::new());
        register(&mut map, SakanaProvider::new());
        register(&mut map, SyntheticProvider::new());
        register(&mut map, TetrateProvider::new());
        register(&mut map, TokenrouterProvider::new());
        register(&mut map, TrustedrouterProvider::new());
        register(&mut map, VeniceProvider::new());
        register(&mut map, VercelProvider::new());
        register(&mut map, WaferAiProvider::new());
        register(&mut map, WandbProvider::new());
        register(&mut map, XaiProvider::new());
        register(&mut map, XpersonaProvider::new());
        register(&mut map, ZenmuxProvider::new());

        register(&mut map, CustomAnthropicProvider::new());
        register(&mut map, CustomOllamaProvider::new());
        register(&mut map, WebhookProvider::new());

        Self { providers: map }
    }

    pub fn list(&self) -> Vec<ProviderDescriptor> {
        self.providers
            .values()
            .map(|p| p.descriptor().clone())
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(id).cloned()
    }

    /// Returns true if the built-in Aegis Cloud provider is preconfigured
    /// (i.e. an API key was found at construction time).
    pub fn aegis_cloud_preconfigured(&self) -> bool {
        // We can't downcast `Arc<dyn Provider>` to `AegisCloudProvider` without
        // adding `Any` bounds to the trait, so we ask the provider itself via
        // a dedicated descriptor flag. The `AegisCloudProvider` overrides
        // `requires_api_key` to `true` AND we check whether the env var is set
        // here as a lightweight proxy.
        if std::env::var("AEGIS_DEFAULT_API_KEY").is_ok() || std::env::var("ZAI_API_KEY").is_ok() {
            return true;
        }
        if let Ok(entry) = keyring::Entry::new("aegis-ai", "aegis-cloud") {
            if entry.get_password().is_ok() {
                return true;
            }
        }
        false
    }
}

fn register(map: &mut BTreeMap<String, Arc<dyn Provider>>, p: Arc<dyn Provider>) {
    let id = p.descriptor().id.clone();
    map.insert(id, p);
}
