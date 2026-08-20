//! Aegis AI — Secure cross-platform AI assistant library.
//!
//! This crate provides the full backend for the Aegis AI desktop application:
//! - Multi-provider AI routing (90+ providers)
//! - Streaming chat over Tauri events
//! - Computer-use agent with real GUI automation (enigo)
//! - Screenshot capture + OCR
//! - OS keychain credential storage
//! - Persistent memory store (SQLite)
//! - Vector-embedding RAG (v0.5)
//! - Security monitor with auto-defense
//! - File integrity monitoring
//! - Network anomaly detection
//! - Webhook/email security alerts
//! - Two operational modes (continuous / on-demand)
//! - File system watcher for proactive AI
//! - Clipboard monitoring
//! - System tray integration
//! - Voice I/O: STT (cloud Whisper), TTS (OS-native + ElevenLabs), PTT (v0.5)
//! - CalDAV calendar integration + intent dispatch (v0.5)
//! - Bilingual UI (English default, Vietnamese)
//! - **v1.6: Multi-agent DAG orchestrator (planner → executor → critic)**
//! - **v1.6: Declarative workflow engine with conditional branches + parallel steps**
//! - **v1.6: Knowledge graph with entity-relation triples + multi-hop queries**
//! - **v1.6: Proactive intelligence layer (pattern detection + insight surfacing)**
//! - **v1.6: Background task queue with cancellation + progress streaming**

// Provider factories intentionally return `Arc<dyn Provider>` rather than
// `Self`, because each provider is registered by id into a heterogeneous
// registry. The `new_ret_no_self` lint would otherwise fire on every
// provider module.
#![allow(clippy::new_ret_no_self)]
// A few Tauri command handlers legitimately accept 7+ arguments because they
// mirror the typed Tauri command ABI. Clippy's default threshold of 7 is too
// tight here.
#![allow(clippy::too_many_arguments)]
// The `&PathBuf` -> `&Path` lint fires on getters that simply forward to
// inner `&PathBuf` fields; the cost of `&PathBuf` is identical to `&Path`
// at the ABI level and the explicit type signals ownership to callers.
#![allow(clippy::ptr_arg)]

pub mod ai;
pub mod calendar;
pub mod commands;
pub mod computer;
pub mod config;
pub mod error;
pub mod i18n;
pub mod intelligence;
pub mod memory;
pub mod modes;
pub mod security;
pub mod state;
pub mod tasks;
pub mod voice;
pub mod workflow;

use std::sync::Arc;

use tauri::Manager;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use crate::state::AppState;

/// Entry point invoked by `main.rs`.
///
/// Initializes logging, loads configuration, wires up the global application
/// state, registers Tauri commands, and starts the event loop.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tracing::info!("Aegis AI v{} starting up", env!("CARGO_PKG_VERSION"));

    let app_state = AppState::new_shared();
    let app_state_for_setup = app_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["aegis-ai"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            // Boot background services.
            let handle = app.handle().clone();
            // Clone the Arc here so the block_on closure can consume its own
            // copy while the outer setup closure retains one for `app.manage`.
            let state_for_boot = app_state_for_setup.clone();
            tauri::async_runtime::block_on(async move {
                if let Err(e) = AppState::boot(&state_for_boot, &handle).await {
                    tracing::error!("boot failed: {e:#}");
                }
            });

            // Setup system tray
            let icon = match app.default_window_icon().cloned() {
                Some(icon) => icon,
                None => {
                    tracing::warn!("no default window icon found; skipping system tray");
                    app.manage(app_state_for_setup);
                    return Ok(());
                }
            };
            let _ = TrayIconBuilder::new()
                .icon(icon)
                .tooltip("Aegis AI — Secure AI Assistant")
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app);

            app.manage(app_state_for_setup);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ===== AI =====
            commands::ai_chat,
            commands::ai_chat_stream,
            commands::ai_chat_cancel,
            commands::ai_list_providers,
            commands::ai_set_active_provider,
            commands::ai_configure_provider,
            commands::ai_test_provider,
            // ===== v0.3 — Aegis Cloud (built-in preconfigured provider) =====
            commands::aegis_cloud_preconfigured,
            commands::aegis_cloud_configure,
            commands::aegis_cloud_test,
            // ===== v0.3 — Agent loop (computer-use co-owner) =====
            commands::ai_agent_run,
            commands::agent_list_tools,
            // ===== v0.3 — Safety: kill switch / rate limiter / audit =====
            commands::safety_trip_kill_switch,
            commands::safety_reset_kill_switch,
            commands::safety_kill_switch_status,
            commands::safety_rate_limiter_status,
            commands::safety_rate_limiter_reset,
            commands::audit_recent,
            commands::audit_count,
            commands::audit_wipe,
            // ===== v0.4 — Bypass Mode =====
            commands::bypass_mode_status,
            commands::bypass_mode_enable,
            commands::bypass_mode_disable,
            // ===== v0.4 — AI model catalog =====
            commands::ai_list_models,
            commands::ai_models_for_provider,
            // ===== v0.4 — Skills =====
            commands::skills_list,
            commands::skills_active,
            commands::skills_set,
            // ===== v0.5 — Voice I/O =====
            commands::voice_transcribe,
            commands::voice_speak,
            commands::voice_ptt_state,
            commands::voice_ptt_set_hotkey,
            // ===== v0.5 — Calendar (CalDAV) =====
            commands::calendar_list_today,
            commands::calendar_configure,
            commands::calendar_dispatch_intent,
            // ===== v0.6 — Web access (real web_search + readability fetch) =====
            commands::web_search,
            commands::web_fetch,
            commands::web_fetch_raw,
            // ===== v0.6 — Memory: entity extraction + encryption status =====
            commands::memory_extract_entities,
            commands::memory_encryption_status,
            commands::memory_export_all,
            commands::memory_forget_all,
            // ===== v0.6 — Security: YARA rules + audit export =====
            commands::yara_list,
            commands::yara_ensure_dir,
            commands::audit_export,
            // ===== Computer use =====
            commands::computer_exec_command,
            commands::computer_open_app,
            commands::computer_list_apps,
            commands::computer_file_read,
            commands::computer_file_write,
            commands::computer_screenshot,
            commands::computer_automate,
            commands::computer_request_action,
            commands::computer_confirm_action,
            // ===== Clipboard =====
            commands::clipboard_read_cmd,
            commands::clipboard_write_cmd,
            commands::clipboard_watch_start_cmd,
            commands::clipboard_watch_stop_cmd,
            // ===== Memory =====
            commands::memory_list_conversations,
            commands::memory_get_conversation,
            commands::memory_clear_all,
            commands::memory_search,
            commands::memory_stats,
            commands::memory_summarize,
            // ===== Security =====
            commands::security_status,
            commands::security_scan,
            commands::security_quarantine_list,
            commands::security_restore_file,
            commands::security_set_auto_defense,
            commands::security_integrity_check,
            commands::security_integrity_save_baseline,
            commands::security_network_scan,
            // ===== Modes =====
            commands::modes_get_active,
            commands::modes_set_mode,
            // ===== Settings / i18n =====
            commands::settings_get,
            commands::settings_set,
            commands::i18n_get_locale,
            commands::i18n_set_locale,
            commands::i18n_translate,
            // ===== v0.7 — Phase 4.2: Sandbox =====
            commands::sandbox_status,
            commands::sandbox_set_enabled,
            commands::sandbox_add_dir,
            commands::sandbox_remove_dir,
            // ===== v0.7 — Phase 4.3: Telemetry =====
            commands::telemetry_status,
            commands::telemetry_opt_in,
            commands::telemetry_opt_out,
            // ===== v1.6 — Multi-Agent Orchestrator =====
            commands::orchestrator_run_plan,
            commands::orchestrator_get_plan,
            commands::orchestrator_list_plans,
            commands::orchestrator_cancel,
            // ===== v1.6 — Workflow Engine =====
            commands::workflow_upsert,
            commands::workflow_delete,
            commands::workflow_get,
            commands::workflow_list,
            commands::workflow_run,
            commands::workflow_runs,
            // ===== v1.6 — Knowledge Graph =====
            commands::graph_add_triple,
            commands::graph_query,
            commands::graph_neighbors,
            commands::graph_path,
            commands::graph_subjects,
            commands::graph_predicates,
            commands::graph_count,
            commands::graph_clear,
            // ===== v1.6 — Proactive Intelligence =====
            commands::proactive_insights,
            commands::proactive_recent,
            commands::proactive_dismiss,
            commands::proactive_enable,
            commands::proactive_disable,
            commands::proactive_enabled,
            // ===== v1.6 — Background Task Queue =====
            commands::tasks_list,
            commands::tasks_active,
            commands::tasks_get,
            commands::tasks_cancel,
            // ===== System =====
            commands::app_version,
            commands::app_quit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Aegis AI");
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,aegis=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false),
        )
        .init();
}

/// Re-exported shared application state alias used by command handlers.
pub type SharedState = Arc<parking_lot::Mutex<AppState>>;
