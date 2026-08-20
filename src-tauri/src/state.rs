//! Global application state shared across Tauri command handlers.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::AppHandle;

use crate::{
    ai::{
        orchestrator::Orchestrator, provider::ProviderRegistry as AiProviderRegistry,
        router::AiRouter,
    },
    calendar::{CalendarClient, CalendarConfig},
    config::{AppConfig, ConfigStore},
    intelligence::ProactiveEngine,
    memory::store::MemoryStore,
    security::{quarantine::QuarantineStore, sandbox::SandboxPolicy, telemetry::TelemetryConfig},
    tasks::TaskQueue,
    voice::HotkeyManager,
    workflow::WorkflowEngine,
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

    /// Cancel tokens for streaming chat requests.
    /// Maps stream_id → watch sender (send true to cancel).
    pub cancel_tokens: Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>,

    /// Pending actions awaiting user confirmation.
    /// Maps token → PendingAction (with 60-second TTL).
    pub pending_actions: Mutex<HashMap<String, crate::commands::PendingAction>>,

    /// File quarantine store (lifted from defender task to AppState in Phase 2).
    pub quarantine: Mutex<QuarantineStore>,

    /// v0.5: Push-to-talk hotkey manager (Ctrl+Space by default).
    pub hotkey: Arc<HotkeyManager>,

    /// v0.5: CalDAV calendar client. Cheap to clone; safe to share.
    pub calendar: Mutex<CalendarClient>,

    /// v0.7: AI sandbox policy — enforces file-write allow-lists.
    pub sandbox: Mutex<SandboxPolicy>,

    /// v0.7: Telemetry config — opt-in only, never on by default.
    pub telemetry: Mutex<TelemetryConfig>,

    /// v1.6: Multi-agent DAG orchestrator.
    pub orchestrator: Arc<Orchestrator>,

    /// v1.6: Declarative workflow engine.
    pub workflow: Arc<WorkflowEngine>,

    /// v1.6: Background task queue — durable substrate behind long-running
    /// plans, workflow runs, and batch operations.
    pub tasks: Arc<TaskQueue>,

    /// v1.6: Proactive intelligence layer — pattern detection + insight
    /// surfacing. Tickled from the continuous-mode heartbeat.
    pub proactive: Arc<ProactiveEngine>,
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

        // Boot with an in-memory SQLite store. The persistent store is
        // opened later in `boot()` and replaces this one.
        let memory = MemoryStore::open_in_memory().expect(
            "in-memory SQLite store must open (this never fails unless system is critically OOM)",
        );

        // v0.5: bootstrap the calendar client. We use a default (no-op)
        // config if the user hasn't configured one yet — they can wire it
        // up via the Settings UI at runtime.
        let calendar_cfg = CalendarConfig::default();
        let calendar = CalendarClient::new(calendar_cfg).unwrap_or_else(|e| {
            tracing::warn!("failed to init calendar client: {e}; using no-op default");
            CalendarClient::new(CalendarConfig::default())
                .expect("default calendar config should always construct")
        });

        Arc::new(Mutex::new(AppState {
            config,
            providers: Mutex::new(providers),
            router,
            memory: Arc::new(memory),
            app_handle: Mutex::new(None),
            cancel_tokens: Mutex::new(HashMap::new()),
            pending_actions: Mutex::new(HashMap::new()),
            quarantine: Mutex::new(QuarantineStore::new()),
            hotkey: Arc::new(HotkeyManager::new()),
            calendar: Mutex::new(calendar),
            sandbox: Mutex::new(SandboxPolicy::default()),
            telemetry: Mutex::new(TelemetryConfig::new()),
            orchestrator: Arc::new(Orchestrator::new()),
            workflow: Arc::new(WorkflowEngine::new()),
            tasks: Arc::new(TaskQueue::new()),
            proactive: Arc::new(ProactiveEngine::new()),
        }))
    }

    /// Boot subsystems that need async initialization (memory store migration,
    /// security monitor spawn, etc.). Called from inside Tauri's `setup`.
    pub async fn boot(state: &Arc<Mutex<Self>>, app: &AppHandle) -> anyhow::Result<()> {
        {
            let s = state.lock();
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
            if let Some(mem_arc) = Arc::get_mut(&mut s.memory) {
                *mem_arc = memory;
            } else {
                tracing::warn!("could not swap memory store — keeping in-memory");
            }
        }

        // v0.5: backfill vector embeddings for any knowledge entries that
        // predate the embeddings table (i.e. v0.4 facts that haven't been
        // re-embedded yet). Best-effort; logged but not fatal.
        {
            let s = state.lock();
            match s.memory.embeddings.backfill() {
                Ok(n) if n > 0 => tracing::info!("backfilled {n} knowledge embeddings"),
                Ok(_) => {}
                Err(e) => tracing::warn!("backfill embeddings failed: {e}"),
            }
        }

        // Load integrity baselines from DB
        {
            let s = state.lock();
            let conn = s.memory.shared_conn();
            let conn = conn.lock();
            if let Err(e) = crate::security::integrity::load_baselines_from_db(&conn) {
                tracing::warn!("failed to load integrity baselines: {e}");
            }
        }

        // v1.6: Wire the orchestrator's parallelism ceiling from config.
        {
            let s = state.lock();
            s.orchestrator
                .set_max_parallel(cfg.orchestrator_max_parallel as usize);
            if cfg.proactive_intelligence {
                s.proactive.enable();
            } else {
                s.proactive.disable();
            }
        }

        tracing::info!(
            "boot complete | language={} | mode={:?} | auto_defense={} | proactive={} | max_parallel={}",
            cfg.language,
            cfg.mode,
            cfg.security.auto_defense,
            cfg.proactive_intelligence,
            cfg.orchestrator_max_parallel,
        );

        // Spawn the security monitor if enabled.
        if cfg.security.monitor {
            let state_for_monitor = state.clone();
            tokio::spawn(async move {
                if let Err(e) = crate::security::monitor::start(state_for_monitor).await {
                    tracing::error!("security monitor exited with error: {e:#}");
                }
            });
        }

        // Spawn the auto-defense watcher if enabled.
        if cfg.security.auto_defense {
            let state_for_defense = state.clone();
            tokio::spawn(async move {
                if let Err(e) = crate::security::defender::start(state_for_defense).await {
                    tracing::error!("auto-defense watcher exited with error: {e:#}");
                }
            });
        }

        // Save initial integrity baseline if none exists
        {
            let baselines = crate::security::integrity::critical_files();
            if !baselines.is_empty()
                && let Ok(saved) = crate::security::integrity::save_baseline()
            {
                tracing::info!("saved integrity baseline for {} files", saved.len());
                // Persist to DB
                let s = state.lock();
                let conn = s.memory.shared_conn();
                let conn = conn.lock();
                let _ = crate::security::integrity::save_baselines_to_db(&conn);
            }
        }

        // v0.5: Register the push-to-talk hotkey (best-effort; the plugin
        // can fail on platforms without accessibility permission, e.g.
        // a fresh Linux box without X11 shortcut support).
        {
            let s = state.lock();
            if let Err(e) = crate::voice::hotkey::register(app, &s.hotkey) {
                tracing::warn!("push-to-talk hotkey registration failed: {e}");
            }
        }

        Ok(())
    }
}
