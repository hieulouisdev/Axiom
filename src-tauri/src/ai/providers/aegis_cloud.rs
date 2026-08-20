//! Aegis Cloud — the built-in preconfigured provider.
//!
//! This is the "out-of-the-box" AI: the user installs Aegis AI and it works
//! immediately, with zero configuration, as long as one of the following
//! supplies an API key:
//!
//! 1. `AEGIS_DEFAULT_API_KEY` environment variable (highest priority).
//! 2. `ZAI_API_KEY` environment variable (alias).
//! 3. A user-supplied key stored in the OS keychain under `"aegis-cloud"`.
//!
//! The provider is backed by the Z.AI GLM family (the same model that powers
//! the Aegis developers' own assistant) and exposed via a fully
//! OpenAI-compatible endpoint. If the env-var key is present at boot, this
//! provider is automatically marked `enabled` and selected as the active
//! provider — so a fresh install is ready to chat in seconds.
//!
//! Users can still override the built-in key from the Settings UI (their
//! custom key is stored in the OS keychain and takes precedence over the
//! env-var fallback). They can also switch to any of the 33+ other providers
//! at any time.

use parking_lot::RwLock;
use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::ai::provider::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, Provider, ProviderCategory,
    ProviderCreds, ProviderDescriptor, Role, Usage,
};
use crate::error::{AegisError, Result};

/// Environment variable consulted at boot for the bundled API key.
pub const ENV_KEY_PRIMARY: &str = "AEGIS_DEFAULT_API_KEY";
/// Alias env var (more memorable for Z.AI users).
pub const ENV_KEY_ALIAS: &str = "ZAI_API_KEY";
/// Keyring service name used to store a user-supplied override key.
pub const KEYRING_SERVICE: &str = "aegis-ai";
pub const KEYRING_USER: &str = "aegis-cloud";

/// Default base URL for the Z.AI GLM OpenAI-compatible endpoint.
pub const DEFAULT_BASE_URL: &str = "https://api.z.ai/api/paas/v4";
/// Default model used by Aegis Cloud. Picked for its balance of speed and
/// quality on commodity prompts.
pub const DEFAULT_MODEL: &str = "glm-4.6";

/// The Aegis Cloud provider.
pub struct AegisCloudProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
    client: Client,
    /// True when an API key was found at construction time (env or keyring).
    /// Surfaces in the UI as the "ready out of the box" badge.
    preconfigured: RwLock<bool>,
}

impl AegisCloudProvider {
    pub fn new() -> Arc<Self> {
        let descriptor = ProviderDescriptor {
            id: "aegis-cloud".into(),
            name: "Aegis Cloud (Built-in)".into(),
            description: "Zero-config built-in AI. Backed by Z.AI GLM-4.6. Reads API key from AEGIS_DEFAULT_API_KEY or ZAI_API_KEY env var; can be overridden from Settings.".into(),
            homepage: "https://z.ai".into(),
            category: ProviderCategory::CloudMajor,
            requires_api_key: true,
            local: false,
            default_base_url: Some(DEFAULT_BASE_URL.into()),
            default_model: DEFAULT_MODEL.into(),
            known_models: vec![
                "glm-4.6".into(),
                "glm-4.5".into(),
                "glm-4.5-air".into(),
                "glm-4-flash".into(),
                "glm-4-air".into(),
                "glm-4-long".into(),
            ],
            implemented: true,
        };

        // Build a tuned HTTP client — see `fast_path` module docs for rationale.
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .connect_timeout(std::time::Duration::from_secs(8))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(8)
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .tcp_nodelay(true)
            .build()
            .expect("aegis-cloud reqwest client");

        // Resolve any preconfigured API key now so we can mark the provider
        // "ready" before any user interaction.
        let (api_key, preconfigured) = resolve_preconfigured_key();
        let creds = ProviderCreds {
            api_key,
            base_url: Some(DEFAULT_BASE_URL.into()),
            model: Some(DEFAULT_MODEL.into()),
            extra: Default::default(),
        };

        Arc::new(Self {
            descriptor,
            creds: RwLock::new(creds),
            client,
            preconfigured: RwLock::new(preconfigured),
        })
    }

    /// True when a key was found at construction (env var or keyring).
    pub fn is_preconfigured(&self) -> bool {
        *self.preconfigured.read()
    }

    fn creds(&self) -> ProviderCreds {
        self.creds.read().clone()
    }

    fn base_url(&self) -> String {
        let c = self.creds();
        c.base_url
            .clone()
            .or_else(|| self.descriptor.default_base_url.clone())
            .unwrap_or_else(|| DEFAULT_BASE_URL.into())
    }

    fn api_key(&self) -> Result<String> {
        let c = self.creds();
        if let Some(k) = c.api_key {
            return Ok(k);
        }
        // Last-resort lookup: env var (in case set_creds was called with None).
        let from_env = std::env::var(ENV_KEY_PRIMARY)
            .ok()
            .or_else(|| std::env::var(ENV_KEY_ALIAS).ok());
        from_env
            .ok_or_else(|| AegisError::AiNotConfigured(
                "Aegis Cloud needs an API key. Set AEGIS_DEFAULT_API_KEY env var, or configure it from Settings → Providers.".into()
            ))
    }

    async fn do_chat(&self, req: &ChatRequest, stream: bool) -> Result<reqwest::Response> {
        let creds = self.creds();
        let model = req
            .model
            .clone()
            .or(creds.model.clone())
            .unwrap_or_else(|| DEFAULT_MODEL.into());

        let api_key = self.api_key()?;

        let body = json!({
            "model": model,
            "messages": req.messages.iter().map(|m| {
                let role = match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                };
                json!({
                    "role": role,
                    "content": m.content,
                })
            }).collect::<Vec<_>>(),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens.unwrap_or(1024),
            "top_p": req.top_p.unwrap_or(1.0),
            "stream": stream,
        });

        let url = format!("{}/chat/completions", self.base_url());
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
                "HTTP {status} from aegis-cloud: {text}"
            )));
        }
        Ok(resp)
    }
}

/// Try to find a preconfigured API key for Aegis Cloud.
/// Order: env var AEGIS_DEFAULT_API_KEY > env var ZAI_API_KEY > OS keyring.
fn resolve_preconfigured_key() -> (Option<String>, bool) {
    if let Ok(k) = std::env::var(ENV_KEY_PRIMARY)
        && !k.trim().is_empty()
    {
        return (Some(k), true);
    }
    if let Ok(k) = std::env::var(ENV_KEY_ALIAS)
        && !k.trim().is_empty()
    {
        return (Some(k), true);
    }
    // Try the OS keyring (the user may have configured a key earlier).
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        && let Ok(password) = entry.get_password()
        && !password.is_empty()
    {
        return (Some(password), true);
    }
    (None, false)
}

#[async_trait]
impl Provider for AegisCloudProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    fn set_creds(&self, creds: ProviderCreds) {
        let was_preconfigured = self.is_preconfigured();
        *self.creds.write() = creds;
        // Preserve the preconfigured flag: once we've seen a key at boot, the
        // provider remains "ready" even if the user later clears it (they can
        // always re-set one from the UI).
        if !was_preconfigured {
            let now_ready = self.creds.read().api_key.is_some();
            *self.preconfigured.write() = now_ready;
        }
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let resp = self.do_chat(&req, false).await?;
        let body: AegisCloudResponse = resp.json().await?;
        let choice = body
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AegisError::Ai("aegis-cloud: empty choices in response".into()))?;
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
        let resp = self.do_chat(&req, true).await?;
        // Reuse the shared OpenAI-compat SSE parser from `openai_compat`.
        let parsed = crate::ai::providers::openai_compat::parse_sse_stream(resp, on_chunk).await?;
        Ok(parsed)
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

/// Wire format returned by the Z.AI OpenAI-compat endpoint.
#[derive(Debug, serde::Deserialize)]
struct AegisCloudResponse {
    pub model: String,
    pub choices: Vec<AegisCloudChoice>,
    #[serde(default)]
    pub usage: Option<AegisCloudUsage>,
}

#[derive(Debug, serde::Deserialize)]
struct AegisCloudChoice {
    #[serde(default)]
    pub message: AegisCloudMessage,
}

#[derive(Debug, Default, serde::Deserialize)]
struct AegisCloudMessage {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tool_calls: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct AegisCloudUsage {
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub completion_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_id_is_stable() {
        let p = AegisCloudProvider::new();
        assert_eq!(p.descriptor().id, "aegis-cloud");
        assert_eq!(p.descriptor().category, ProviderCategory::CloudMajor);
    }

    #[test]
    fn preconfigured_flag_is_bool() {
        // We can't assert a specific value here because it depends on env,
        // but we can at least make sure the call doesn't panic.
        let p = AegisCloudProvider::new();
        let _ = p.is_preconfigured();
    }
}
