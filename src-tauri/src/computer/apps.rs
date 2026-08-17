//! Application launch / enumeration.

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::{AegisError, Result};

use super::safety::{SafetyDecision, SafetyPolicy};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDescriptor {
    pub name: String,
    pub path: Option<String>,
    pub icon: Option<String>,
}

/// Launch an application by name (cross-platform best-effort).
///
/// On Linux: tries `gtk-launch <name>.desktop`, falls back to direct exec.
/// On Windows: uses `cmd /C start <name>`.
pub fn open_app(policy: &SafetyPolicy, name: &str) -> Result<()> {
    match policy.check_app_launch(name) {
        SafetyDecision::Allow => {}
        SafetyDecision::Deny { reason } => {
            return Err(AegisError::SafetyDenial(reason));
        }
        SafetyDecision::RequireConfirmation { token, summary, .. } => {
            return Err(AegisError::SafetyConfirmation { token, summary });
        }
    }
    launch_app_inner(name)
}

pub fn open_app_authorized(name: &str) -> Result<()> {
    launch_app_inner(name)
}

fn launch_app_inner(name: &str) -> Result<()> {
    #[cfg(unix)]
    {
        // Try gtk-launch first (works for .desktop files registered with the DE).
        let mut c = Command::new("gtk-launch");
        c.arg(name);
        if c.spawn().is_ok() {
            return Ok(());
        }
        // Fall back to direct execution.
        let mut c = Command::new(name);
        c.spawn()
            .map_err(|e| AegisError::Io(format!("failed to launch '{name}': {e}")))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let mut c = Command::new("cmd");
        c.arg("/C").arg("start").arg("").arg(name);
        c.spawn()
            .map_err(|e| AegisError::Io(format!("failed to launch '{name}': {e}")))?;
        Ok(())
    }
}

/// List commonly-known applications on the user's machine.
///
/// v0.1: returns a curated list of well-known apps detected by checking
/// whether their executable is on PATH. Phase 3 will scan start-menu entries.
pub fn list_apps() -> Vec<AppDescriptor> {
    let known = known_apps();
    known
        .into_iter()
        .filter(|a| {
            // For now, include everything; Phase 3 will check `which` / start-menu.
            true
        })
        .collect()
}

fn known_apps() -> Vec<AppDescriptor> {
    let mut out: Vec<AppDescriptor> = Vec::new();

    #[cfg(unix)]
    {
        let names = [
            "firefox", "chromium", "google-chrome", "brave", "code", "subl", "vim",
            "emacs", "git", "htop", "vlc", "spotify", "discord", "slack", "telegram-desktop",
            "thunderbird", "libreoffice", "gimp", "inkscape", "blender", "audacity",
            "files", "nautilus", "thunar", "dolphin", "tilix", "gnome-terminal", "konsole",
            "xterm", "alacritty", "kitty", "wezterm", "tmux", "btop", "nvidia-smi",
        ];
        for n in names {
            out.push(AppDescriptor {
                name: n.into(),
                path: which(n),
                icon: None,
            });
        }
    }

    #[cfg(not(unix))]
    {
        let names = [
            "chrome", "msedge", "firefox", "brave", "code", "notepad++", "vim",
            "git", "explorer", "vlc", "spotify", "discord", "slack", "telegram",
            "outlook", "winword", "excel", "powerpnt", "gimp", "inkscape", "blender",
            "audacity", "cmd", "powershell", "pwsh", "wt", "taskmgr", "regedit",
        ];
        for n in names {
            out.push(AppDescriptor {
                name: n.into(),
                path: which(n),
                icon: None,
            });
        }
    }

    out
}

#[cfg(unix)]
fn which(name: &str) -> Option<String> {
    let output = Command::new("which").arg(name).output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(not(unix))]
fn which(name: &str) -> Option<String> {
    let output = Command::new("where").arg(name).output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("").to_string())
    } else {
        None
    }
}
