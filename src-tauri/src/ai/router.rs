//! AI router: selects the active provider, applies cost-saving policies,
//! and forwards chat requests.

use std::sync::Arc;

use parking_lot::RwLock;

use crate::config::ConfigStore;
use crate::error::{AegisError, Result};

use super::provider::{ChatRequest, ChatResponse, ChatStreamChunk, Provider, ProviderCreds};

/// The AI router is the single entry-point the rest of the app uses to talk
/// to AI providers. It is responsible for:
///
/// 1. Resolving the active provider from the user's config.
/// 2. Injecting the latest credentials into the provider before each call.
/// 3. Applying cost-saving policies (e.g. caching, max-token caps).
/// 4. Surfacing a unified error type to callers.
pub struct AiRouter {
    config: Arc<ConfigStore>,
    /// Cached active provider id (so we don't re-read the config on every call).
    active_provider: RwLock<Option<String>>,
}

impl AiRouter {
    pub fn new(config: Arc<ConfigStore>) -> Self {
        let active = config.read().active_provider.clone();
        Self {
            config,
            active_provider: RwLock::new(active),
        }
    }

    /// Synchronize the router's cached active-provider id with the config.
    pub fn refresh(&self) {
        let active = self.config.read().active_provider.clone();
        *self.active_provider.write() = active;
    }

    pub fn set_active(&self, id: Option<String>) -> Result<()> {
        {
            let mut cfg = self.config.write();
            cfg.active_provider = id.clone();
        }
        self.config.persist()?;
        *self.active_provider.write() = id;
        Ok(())
    }

    /// Resolve the active provider from a registry.
    pub fn resolve(
        &self,
        registry: &super::provider::ProviderRegistry,
    ) -> Result<Arc<dyn Provider>> {
        let id = self
            .active_provider
            .read()
            .clone()
            .or_else(|| {
                // Default to the first configured provider if none is active.
                let cfg = self.config.read();
                cfg.providers
                    .credentials
                    .iter()
                    .find(|(_, c)| c.enabled)
                    .map(|(id, _)| id.clone())
            })
            .ok_or_else(|| {
                AegisError::AiNotConfigured(
                    "no AI provider is configured. Open Settings → Providers to add one.".into(),
                )
            })?;

        let provider = registry.get(&id).ok_or_else(|| {
            AegisError::AiNotConfigured(format!("provider '{id}' is not registered"))
        })?;

        // Inject credentials.
        let cfg = self.config.read();
        if let Some(creds_cfg) = cfg.providers.credentials.get(&id) {
            let creds = ProviderCreds {
                api_key: creds_cfg.api_key.clone(),
                base_url: creds_cfg.base_url.clone(),
                model: creds_cfg.model.clone(),
                extra: creds_cfg.extra.clone(),
            };
            provider.set_creds(creds);
        }

        Ok(provider)
    }

    pub async fn chat(
        &self,
        registry: &super::provider::ProviderRegistry,
        req: ChatRequest,
    ) -> Result<ChatResponse> {
        let provider = self.resolve(registry)?;
        provider.chat(req).await
    }

    pub async fn chat_stream(
        &self,
        registry: &super::provider::ProviderRegistry,
        req: ChatRequest,
        on_chunk: Box<dyn Fn(ChatStreamChunk) + Send + Sync>,
    ) -> Result<ChatResponse> {
        let provider = self.resolve(registry)?;
        provider.chat_stream(req, on_chunk).await
    }
}
