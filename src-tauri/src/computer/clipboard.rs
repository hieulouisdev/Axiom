//! Clipboard monitoring and control.
//!
//! Provides commands for reading/writing the system clipboard and
//! a watch mode that logs clipboard changes to the activity store.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::error::{AegisError, Result};
use std::sync::LazyLock;

/// Result of reading the clipboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardContent {
    pub text: String,
    pub timestamp_ms: u64,
}

/// State for clipboard watching.
static WATCHING: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

/// Last clipboard content (for change detection).
static LAST_CONTENT: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::new()));

/// Read the current clipboard text.
pub fn clipboard_read() -> Result<ClipboardContent> {
    let text = read_clipboard_text()?;
    Ok(ClipboardContent {
        text,
        timestamp_ms: now_ms(),
    })
}

/// Write text to the clipboard.
pub fn clipboard_write(text: &str) -> Result<()> {
    write_clipboard_text(text)?;
    *LAST_CONTENT.lock() = text.to_string();
    Ok(())
}

/// Start watching the clipboard for changes.
pub fn clipboard_watch_start() -> Result<()> {
    *WATCHING.lock() = true;
    tracing::info!("clipboard watcher started");
    Ok(())
}

/// Stop watching the clipboard.
pub fn clipboard_watch_stop() -> Result<()> {
    *WATCHING.lock() = false;
    tracing::info!("clipboard watcher stopped");
    Ok(())
}

/// Check if clipboard watching is active, and if so detect changes.
/// Called periodically from the continuous mode heartbeat.
pub fn clipboard_poll() -> Option<String> {
    if !*WATCHING.lock() {
        return None;
    }
    let text = read_clipboard_text().ok()?;
    let mut last = LAST_CONTENT.lock();
    if text != *last && !text.is_empty() {
        *last = text.clone();
        Some(text)
    } else {
        None
    }
}

fn now_ms() -> u64 {
    time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000
}

// Platform-specific clipboard implementation using command-line tools
// as a fallback when tauri-plugin-clipboard-manager is not accessible.

fn read_clipboard_text() -> Result<String> {
    #[cfg(unix)]
    {
        // Try xclip first, then xsel, then wl-paste (Wayland)
        if let Ok(output) = std::process::Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .output()
            && output.status.success()
        {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
        if let Ok(output) = std::process::Command::new("xsel")
            .args(["--clipboard", "--output"])
            .output()
            && output.status.success()
        {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
        if let Ok(output) = std::process::Command::new("wl-paste").output()
            && output.status.success()
        {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
        Err(AegisError::Internal(
            "no clipboard tool available (install xclip, xsel, or wl-paste)".into(),
        ))
    }

    #[cfg(windows)]
    {
        if let Ok(output) = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-Clipboard"])
            .output()
        {
            if output.status.success() {
                return Ok(String::from_utf8_lossy(&output.stdout).to_string());
            }
        }
        Err(AegisError::Internal(
            "clipboard read failed on Windows".into(),
        ))
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(AegisError::Internal(
            "clipboard not supported on this platform".into(),
        ))
    }
}

fn write_clipboard_text(text: &str) -> Result<()> {
    #[cfg(unix)]
    {
        // Try xclip first
        if let Ok(mut child) = std::process::Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
                drop(stdin);
            }
            if let Ok(status) = child.wait()
                && status.success()
            {
                return Ok(());
            }
        }
        // Try xsel
        if let Ok(mut child) = std::process::Command::new("xsel")
            .args(["--clipboard", "--input"])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
                drop(stdin);
            }
            if let Ok(status) = child.wait()
                && status.success()
            {
                return Ok(());
            }
        }
        // Try wl-copy (Wayland)
        if let Ok(mut child) = std::process::Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
                drop(stdin);
            }
            if let Ok(status) = child.wait()
                && status.success()
            {
                return Ok(());
            }
        }
        Err(AegisError::Internal(
            "no clipboard tool available (install xclip, xsel, or wl-copy)".into(),
        ))
    }

    #[cfg(windows)]
    {
        if let Ok(status) = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Set-Clipboard -Value '{}'", text.replace('\'', "''")),
            ])
            .status()
        {
            if status.success() {
                return Ok(());
            }
        }
        Err(AegisError::Internal(
            "clipboard write failed on Windows".into(),
        ))
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = text;
        Err(AegisError::Internal(
            "clipboard not supported on this platform".into(),
        ))
    }
}
