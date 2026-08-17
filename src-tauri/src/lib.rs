//! Aegis AI — Secure cross-platform AI assistant library.
//!
//! This crate provides the full backend for the Aegis AI desktop application:
//! - Multi-provider AI routing (20+ providers)
//! - Streaming chat over Tauri events
//! - Computer-use agent with real GUI automation (enigo)
//! - Screenshot capture + OCR
//! - OS keychain credential storage
//! - Persistent memory store (SQLite)
//! - Security monitor with auto-defense
//! - File integrity monitoring
//! - Network anomaly detection
//! - Webhook/email security alerts
//! - Two operational modes (continuous / on-demand)
//! - File system watcher for proactive AI
//! - Clipboard monitoring
//! - System tray integration
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
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};

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
            Some("aegis-ai"),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(move |app| {
            // Boot background services.
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                if let Err(e) = AppState::boot(&app_state_for_setup, &handle).await {
                    tracing::error!("boot failed: {e:#}");
                }
            });

            // Setup system tray
            let _ = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
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
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
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
                    }
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
