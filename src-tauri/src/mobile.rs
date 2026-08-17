//! Mobile companion scaffold (Phase 3.5 — v0.6).
//!
//! Tauri 2.0 supports mobile targets (iOS + Android) via the
//! `tauri::mobile_entry_point` macro. v0.6 lays the groundwork for a
//! read-only mobile companion app that can:
//!
//! - Display the user's recent conversations (read-only).
//! - Show the current security status (monitor + auto-defense on/off).
//! - Push notifications for critical security events.
//! - End-to-end-encrypted sync of conversation history via a relay
//!   (placeholder — full sync is Phase 4).
//!
//! On desktop targets this module is a no-op: the entry point is only
//! compiled when `cfg(mobile)` is set, which Tauri sets automatically when
//! building for iOS or Android.
//!
//! The mobile UI reuses the same React components as the desktop app, but
//! the desktop-only subsystems (computer-use, file-system watcher, voice
//! PTT hotkey, CalDAV calendar) are stubbed out via feature flags in the
//! Tauri config.
//!
//! ## Build instructions (Phase 4 — not yet wired into CI)
//!
//! ```bash
//! # Initialize mobile projects (one-time)
//! cargo tauri android init
//! cargo tauri ios init
//!
//! # Build debug APK
//! cargo tauri android dev
//!
//! # Build release IPA (requires Apple Developer account)
//! cargo tauri ios build --release
//! ```
//!
//! The mobile build is gated behind a `mobile` feature in `Cargo.toml` so
//! desktop builds don't pull in the Android/iOS NDK dependencies.

use serde::{Deserialize, Serialize};

/// Mobile companion capabilities reported by the desktop app to the mobile
/// client during the (future) pairing handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileCapabilities {
    /// Maximum number of conversations the mobile client can read.
    pub max_conversations: u32,
    /// Whether the mobile client can trigger remote actions on the desktop.
    pub remote_actions_enabled: bool,
    /// Whether end-to-end-encrypted sync is available (Phase 4).
    pub e2ee_sync_available: bool,
    /// Server version of the desktop app the mobile is paired with.
    pub desktop_version: String,
}

impl Default for MobileCapabilities {
    fn default() -> Self {
        Self {
            max_conversations: 50,
            remote_actions_enabled: false,
            e2ee_sync_available: false,
            desktop_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Return the current mobile companion capabilities. Used by the (future)
/// pairing endpoint over the local relay.
pub fn capabilities() -> MobileCapabilities {
    MobileCapabilities::default()
}

/// Mobile entry point. On desktop this is never called; on mobile Tauri
/// invokes it via the `#[cfg_attr(mobile, tauri::mobile_entry_point)]`
/// attribute on the main `run()` function in `lib.rs`.
#[cfg(mobile)]
pub fn mobile_run() {
    // The mobile entry point shares the same `run()` as desktop — Tauri
    // handles the platform-specific window/webview setup. We just delegate.
    crate::run();
}

/// Stub for the (future) E2EE sync handshake. Phase 4 will implement a
/// proper key-exchange protocol (Signal-style X3DH) and a relay protocol
/// (probably WebSocket over Tor or a self-hosted Matrix backend).
///
/// For v0.6 we just surface a "not yet implemented" error so the UI can
/// show a "coming in Phase 4" badge.
pub fn e2ee_sync_status() -> &'static str {
    "Phase 4 — not yet implemented"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_default() {
        let c = capabilities();
        assert!(!c.remote_actions_enabled);
        assert!(!c.e2ee_sync_available);
        assert!(!c.desktop_version.is_empty());
    }

    #[test]
    fn e2ee_status_is_phase4() {
        assert!(e2ee_sync_status().contains("Phase 4"));
    }
}
