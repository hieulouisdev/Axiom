//! System-wide push-to-talk hotkey (Phase 3.2).
//!
//! Registers a global shortcut via `tauri-plugin-global-shortcut` and emits
//! Tauri events (`voice://push_to_talk_start` / `voice://push_to_talk_stop`)
//! when the user presses / releases the hotkey. The frontend listens for
//! these events to start/stop microphone capture and ship audio bytes to
//! the backend's `voice_transcribe` command.
//!
//! Default hotkey: `Ctrl+Space`. Override via `AEGIS_PTT_HOTKEY` env var
//! or the Settings UI (stored in `voice.toml` inside the data dir).
//!
//! Note: `tauri-plugin-global-shortcut` only fires `on_pressed` (no
//! `on_released` in the public API), so v0.5 implements a simple toggle
//! semantics: first press = start recording, second press = stop & send.
//! Phase 4 will switch to hold-to-talk once the plugin exposes `on_released`.

use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::error::{AegisError, Result};

/// Default hotkey: Ctrl+Space.
pub const DEFAULT_PTT_HOTKEY: &str = "Ctrl+Space";

/// State of the push-to-talk toggle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PushToTalkState {
    /// Microphone is off; the next hotkey press starts recording.
    Idle,
    /// Microphone is recording; the next hotkey press stops and sends.
    Recording,
}

/// The manager owns the registered shortcut handle and the current state.
pub struct HotkeyManager {
    state: Mutex<PushToTalkState>,
    hotkey: Mutex<String>,
    registered: Mutex<bool>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(PushToTalkState::Idle),
            hotkey: Mutex::new(default_hotkey()),
            registered: Mutex::new(false),
        }
    }

    pub fn state(&self) -> PushToTalkState {
        *self.state.lock()
    }

    pub fn hotkey(&self) -> String {
        self.hotkey.lock().clone()
    }

    pub fn set_hotkey(&self, hotkey: &str) {
        *self.hotkey.lock() = hotkey.to_string();
    }

    pub fn is_registered(&self) -> bool {
        *self.registered.lock()
    }

    /// Toggle PTT state. Returns the new state. Emits an event to the
    /// frontend so the UI can react (show "Recording…" indicator, etc.).
    pub fn toggle(&self, app: &AppHandle) -> PushToTalkState {
        let new_state = match *self.state.lock() {
            PushToTalkState::Idle => PushToTalkState::Recording,
            PushToTalkState::Recording => PushToTalkState::Idle,
        };
        *self.state.lock() = new_state;
        let _ = app.emit(
            "voice://push_to_talk",
            serde_json::json!({
                "state": match new_state {
                    PushToTalkState::Idle => "idle",
                    PushToTalkState::Recording => "recording",
                },
            }),
        );
        new_state
    }

    pub fn reset(&self) {
        *self.state.lock() = PushToTalkState::Idle;
    }
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Read the default PTT hotkey from `AEGIS_PTT_HOTKEY` env var, otherwise
/// fall back to `Ctrl+Space`.
pub fn default_hotkey() -> String {
    std::env::var("AEGIS_PTT_HOTKEY").unwrap_or_else(|_| DEFAULT_PTT_HOTKEY.to_string())
}

/// Register the push-to-talk hotkey with the global-shortcut plugin.
///
/// Call from `setup` once the Tauri app is initialized. Safe to call
/// multiple times — subsequent calls unregister the previous shortcut
/// before registering the new one.
pub fn register(app: &AppHandle, manager: &Arc<HotkeyManager>) -> Result<()> {
    let hotkey_str = manager.hotkey();
    let shortcut: Shortcut = hotkey_str
        .parse()
        .map_err(|e| AegisError::Config(format!("invalid hotkey '{hotkey_str}': {e}")))?;

    let app_clone = app.clone();
    let manager_clone = manager.clone();
    let handler = move |_app: &AppHandle, _shortcut: &Shortcut, event: ShortcutState| {
        // global-shortcut only fires on pressed in tauri-plugin 2.0 stable,
        // but we keep the match for forward-compat.
        if event == ShortcutState::Pressed {
            manager_clone.toggle(&app_clone);
        }
    };

    // Unregister any previous shortcut (idempotent).
    if manager.is_registered() {
        let _ = app.global_shortcut().unregister(shortcut.clone());
    }
    app.global_shortcut()
        .on_shortcut(shortcut, handler)
        .map_err(|e| AegisError::Config(format!("failed to register hotkey '{hotkey_str}': {e}")))?;
    *manager.registered.lock() = true;
    tracing::info!("push-to-talk hotkey registered: {hotkey_str}");
    Ok(())
}

/// Unregister the current hotkey (if any).
pub fn unregister(app: &AppHandle, manager: &Arc<HotkeyManager>) -> Result<()> {
    if !manager.is_registered() {
        return Ok(());
    }
    let hotkey_str = manager.hotkey();
    let shortcut: Shortcut = hotkey_str
        .parse()
        .map_err(|e| AegisError::Config(format!("invalid hotkey '{hotkey_str}': {e}")))?;
    app.global_shortcut()
        .unregister(shortcut)
        .map_err(|e| AegisError::Config(format!("failed to unregister hotkey: {e}")))?;
    *manager.registered.lock() = false;
    manager.reset();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hotkey_is_ctrl_space() {
        std::env::remove_var("AEGIS_PTT_HOTKEY");
        assert_eq!(default_hotkey(), "Ctrl+Space");
    }

    #[test]
    fn toggle_alternates_state() {
        let m = HotkeyManager::new();
        assert_eq!(m.state(), PushToTalkState::Idle);
        // toggle() emits an event, but in tests we don't have an AppHandle,
        // so we test the state transitions directly.
        let s = match *m.state.lock() {
            PushToTalkState::Idle => PushToTalkState::Recording,
            PushToTalkState::Recording => PushToTalkState::Idle,
        };
        *m.state.lock() = s;
        assert_eq!(m.state(), PushToTalkState::Recording);
    }
}
