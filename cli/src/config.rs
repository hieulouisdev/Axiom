//! Configuration for the CLI.
//!
//! Stored as TOML in the platform data directory.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ai::provider::{ProviderConfig, ProviderRegistry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub active_provider: Option<String>,
    pub default_model: Option<String>,
    #[serde(default)]
    pub providers: HashMap<String, ProviderEntry>,
    #[serde(default)]
    pub persona_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            active_provider: Some("zai".into()),
            default_model: None,
            providers: HashMap::new(),
            persona_user_id: "default".into(),
        }
    }
}

impl Config {
    pub fn load_or_create(path: &Path) -> anyhow::Result<Self> {
        if path.exists() {
            let s = std::fs::read_to_string(path)?;
            let mut cfg: Config = toml::from_str(&s).unwrap_or_default();
            if cfg.persona_user_id.is_empty() {
                cfg.persona_user_id = "default".into();
            }
            Ok(cfg)
        } else {
            let cfg = Config::default();
            cfg.save(path)?;
            Ok(cfg)
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = toml::to_string_pretty(self)?;
        std::fs::write(path, s)?;
        Ok(())
    }

    /// Ensure the built-in zero-key Z.AI provider is always present.
    pub fn ensure_default_providers(&mut self) {
        self.providers.entry("zai".into()).or_insert(ProviderEntry {
            api_key: None,
            base_url: Some("https://api.z.ai/api/paas/v4".into()),
            default_model: Some("glm-4.6".into()),
        });
        self.providers.entry("ollama".into()).or_insert(ProviderEntry {
            api_key: None,
            base_url: Some("http://localhost:11434/v1".into()),
            default_model: Some("llama3.2".into()),
        });
    }

    pub fn set_provider(
        &mut self,
        id: &str,
        api_key: Option<String>,
        base_url: Option<String>,
        default_model: Option<String>,
    ) {
        let entry = self.providers.entry(id.into()).or_insert(ProviderEntry {
            api_key: None,
            base_url: None,
            default_model: None,
        });
        if let Some(k) = api_key { entry.api_key = Some(k); }
        if let Some(u) = base_url { entry.base_url = Some(u); }
        if let Some(m) = default_model { entry.default_model = Some(m); }
    }

    pub fn provider_registry(&self) -> ProviderRegistry {
        let mut reg = ProviderRegistry::new();
        for (id, e) in &self.providers {
            reg.set_config(ProviderConfig {
                id: id.clone(),
                api_key: e.api_key.clone(),
                base_url: e.base_url.clone(),
                default_model: e.default_model.clone(),
            });
        }
        if let Some(active) = &self.active_provider {
            let _ = reg.set_active(active);
        }
        reg
    }
}
