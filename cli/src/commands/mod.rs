//! Command handlers for the CLI.

use std::sync::Arc;

use crate::ai::AiRouter;
use crate::memory::MemoryStore;
use crate::mcp::ToolRegistry;

pub mod chat;
pub mod code;
pub mod configure;
pub mod mcp;
pub mod memory;
pub mod providers;
pub mod skills;
pub mod wiki;
pub mod world;

pub struct Context {
    pub config_path: std::path::PathBuf,
    pub db_path: std::path::PathBuf,
    pub config: crate::config::Config,
    pub memory: Arc<MemoryStore>,
    pub router: AiRouter,
    pub mcp_registry: ToolRegistry,
}

impl Context {
    pub fn new(config_path: Option<&str>, db_path: Option<&str>) -> anyhow::Result<Self> {
        let data_dir = directories::ProjectDirs::from("com", "aegis", "aegis-ai")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from(".aegis"));
        std::fs::create_dir_all(&data_dir)?;

        let config_path = config_path
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| data_dir.join("config.toml"));
        let db_path = db_path
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| data_dir.join("memory.db"));

        let mut config = crate::config::Config::load_or_create(&config_path)?;
        // Ensure Z.AI is always preconfigured (zero-key GLM-4.6 public preview)
        config.ensure_default_providers();
        if config.active_provider.is_none() {
            config.active_provider = Some("zai".into());
        }
        config.save(&config_path)?;

        let memory = Arc::new(MemoryStore::open(&db_path)?);
        let router = AiRouter::new(config.provider_registry());
        let mcp_registry = ToolRegistry::new();

        Ok(Self {
            config_path,
            db_path,
            config,
            memory,
            router,
            mcp_registry,
        })
    }
}
