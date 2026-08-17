//! Global application state shared across Tauri command handlers.

use std::sync::Arc;

use parking_lot::Mutex;
use tauri::AppHandle;

use crate::{
    ai::{provider::ProviderRegistry as AiProviderRegistry, router::AiRouter},
    config::{AppConfig, ConfigStore},
    memory::store::MemoryStore,
};

/// Top-level container holding live handles to every backend subsystem.
///
/// Created once at startup and wrapped in [`std::sync::Arc`] so Tauri
/// command handlers can share it cheaply. Internal mutability is provided
/// by `parking_lot::Mutex` (cheap, non-async) — subsystems that need async
/// locking (e.g. AI router during streaming) use their own tokio-aware
/// primitives internally.
pub struct AppState {
    /// Persisted application configuration (config.toml).
    pub config: Arc<ConfigStore>,

    /// All configured AI providers (live instances).
    pub providers: Mutex<AiProviderRegistry>,

    /// AI router that picks the active provider and forwards requests.
    pub router: Arc<AiRouter>,

    /// Persistent memory store (SQLite).
    pub memory: Arc<MemoryStore>,

    /// Handle to the running Tauri application (set during boot).
    pub app_handle: Mutex<Option<AppHandle>>,
}

impl AppState {
    /// Load config from disk (or create a default) and wire up subsystems.
    pub fn new_shared() -> Arc<Mutex<AppState>> {
        let cfg = AppConfig::load().unwrap_or_else(|e| {
            tracing::warn!("failed to load config ({e}); falling back to default");
            AppConfig::default()
        });

        let config = Arc::new(ConfigStore::new(cfg));

        let providers = AiProviderRegistry::with_builtin();
        let router = Arc::new(AiRouter::new(config.clone()));
        let memory = MemoryStore::open_in_memory().unwrap_or_else(|e| {
            tracing::error!("failed to open memory store: {e}");
            MemoryStore::open_in_memory().expect("in-memory sqlite should always open")
        });

        Arc::new(Mutex::new(AppState {
            config,
            providers: Mutex::new(providers),
            router,
            memory: Arc::new(memory),
            app_handle: Mutex::new(None),
        }))
    }

    /// Boot subsystems that need async initialization (memory store migration,
    /// security monitor spawn, etc.). Called from inside Tauri's `setup`.
    pub async fn boot(state: &Arc<Mutex<Self>>, app: &AppHandle) -> anyhow::Result<()> {
        {
            let mut s = state.lock();
            *s.app_handle.lock() = Some(app.clone());
        }

        // Initialize persistent SQLite database.
        let data_dir = AppConfig::data_dir();
        tokio::fs::create_dir_all(&data_dir).await.ok();
        let db_path = data_dir.join("aegis.db");

        let cfg = {
            let s = state.lock();
            s.config.read().clone()
        };

        let memory = MemoryStore::open(&db_path)?;
        memory.migrate()?;
        // Replace the in-memory store with the persistent one.
        {
            let mut s = state.lock();
            // SAFETY: we are the only holder at boot time.
            // Use Arc::get_mut to swap; if it fails (refs exist), fall back.
            if let Some(mem_arc) = Arc::get_mut(&mut s.memory) {
                *mem_arc = memory;
            } else {
                tracing::warn!("could not swap memory store — keeping in-memory");
            }
        }

        tracing::info!(
            "boot complete | language={} | mode={:?} | auto_defense={}",
            cfg.language,
            cfg.mode,
            cfg.security.auto_defense
        );

        // Spawn the security monitor if enabled.
        if cfg.security.monitor {
            let state_for_monitor = state.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    crate::security::monitor::start(state_for_monitor).await
                {
                    tracing::error!("security monitor exited with error: {e:#}");
                }
            });
        }

        // Spawn the auto-defense watcher if enabled.
        if cfg.security.auto_defense {
            let state_for_defense = state.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    crate::security::defender::start(state_for_defense).await
                {
                    tracing::error!("auto-defense watcher exited with error: {e:#}");
                }
            });
        }

        Ok(())
    }
}
