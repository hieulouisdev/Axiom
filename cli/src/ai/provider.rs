//! Provider trait + registry.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn default_model(&self) -> &str;
    async fn chat(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse>;
}

pub struct ProviderRegistry {
    pub configs: HashMap<String, ProviderConfig>,
    pub active: Option<String>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { configs: HashMap::new(), active: None }
    }

    pub fn set_config(&mut self, cfg: ProviderConfig) {
        self.configs.insert(cfg.id.clone(), cfg);
    }

    pub fn get_config(&self, id: &str) -> Option<&ProviderConfig> {
        self.configs.get(id)
    }

    pub fn list(&self) -> Vec<&ProviderConfig> {
        let mut v: Vec<_> = self.configs.values().collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    pub fn set_active(&mut self, id: &str) -> anyhow::Result<()> {
        if !self.configs.contains_key(id) {
            anyhow::bail!("unknown provider: {id}");
        }
        self.active = Some(id.to_string());
        Ok(())
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active.as_deref()
    }

    /// Build a live provider instance from its config.
    pub fn build(&self, id: &str) -> anyhow::Result<Arc<dyn Provider>> {
        let cfg = self
            .get_config(id)
            .ok_or_else(|| anyhow::anyhow!("provider {id} not configured"))?;
        Ok(match id {
            "openai" => Arc::new(super::providers::openai::OpenAiProvider::new(cfg.clone())),
            "anthropic" => Arc::new(super::providers::anthropic::AnthropicProvider::new(cfg.clone())),
            "gemini" => Arc::new(super::providers::gemini::GeminiProvider::new(cfg.clone())),
            "deepseek" => Arc::new(super::providers::deepseek::DeepSeekProvider::new(cfg.clone())),
            "zai" => Arc::new(super::providers::zai::ZaiProvider::new(cfg.clone())),
            "ollama" => Arc::new(super::providers::ollama::OllamaProvider::new(cfg.clone())),
            "openrouter" => Arc::new(super::providers::openrouter::OpenRouterProvider::new(cfg.clone())),
            _ => Arc::new(super::providers::openai_compat::OpenAiCompatProvider::new(cfg.clone())),
        })
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
