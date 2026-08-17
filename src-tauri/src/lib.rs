//! Aegis AI — Secure cross-platform AI assistant library.
//!
//! This crate provides the full backend for the Aegis AI desktop application:
//! - Multi-provider AI routing (20+ providers)
//! - Computer-use agent with safety confirmation
//! - Persistent memory store (SQLite)
//! - Security monitor with auto-defense
//! - Two operational modes (continuous / on-demand)
//! - Bilingual UI (English default, Vietnamese)

pub mod ai;
pub mod commands;
pub mod computer;
pub mod config;
pub mod error;
pub mod i18n;
pub mod memory;
pub mod modes;
pub mod security;
pub mod state;

use std::sync::Arc;

use tauri::Manager;

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
        .setup(move |app| {
            // Boot background services.
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                if let Err(e) = AppState::boot(&app_state_for_setup, &handle).await {
                    tracing::error!("boot failed: {e:#}");
                }
            });
            app.manage(app_state_for_setup);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ===== AI =====
            commands::ai_chat,
            commands::ai_chat_stream,
            commands::ai_list_providers,
            commands::ai_set_active_provider,
            commands::ai_configure_provider,
            commands::ai_test_provider,
            // ===== Computer use =====
            commands::computer_exec_command,
            commands::computer_open_app,
            commands::computer_list_apps,
            commands::computer_file_read,
            commands::computer_file_write,
            commands::computer_screenshot,
            commands::computer_automate,
            commands::computer_confirm_action,
            // ===== Memory =====
            commands::memory_list_conversations,
            commands::memory_get_conversation,
            commands::memory_clear_all,
            commands::memory_search,
            commands::memory_stats,
            // ===== Security =====
            commands::security_status,
            commands::security_scan,
            commands::security_quarantine_list,
            commands::security_restore_file,
            commands::security_set_auto_defense,
            // ===== Modes =====
            commands::modes_get_active,
            commands::modes_set_mode,
            // ===== Settings / i18n =====
            commands::settings_get,
            commands::settings_set,
            commands::i18n_get_locale,
            commands::i18n_set_locale,
            commands::i18n_translate,
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
