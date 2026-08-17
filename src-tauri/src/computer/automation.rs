//! GUI automation: mouse / keyboard simulation.
//!
//! v0.1 provides a single high-level `auto_perform` entry-point that maps
//! a declarative [`AutoAction`] into the appropriate platform API.
//! Phase 3 of the ROADMAP replaces the stubs with full
//! `enigo`-style simulation (cross-platform) including OCR-driven targeting.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{AegisError, Result};

/// A single declarative GUI action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snakecase")]
pub enum AutoAction {
    /// Move the mouse to (x, y).
    MouseMove { x: i32, y: i32 },
    /// Left-click at (x, y).
    MouseClick { x: i32, y: i32, button: MouseButton },
    /// Double-click at (x, y).
    MouseDoubleClick { x: i32, y: i32 },
    /// Type a string.
    TypeText { text: String },
    /// Press a single key (e.g. "Enter", "Ctrl+C").
    PressKey { combo: String },
    /// Wait for a number of milliseconds.
    Sleep { ms: u64 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Execute a sequence of declarative actions.
///
/// Each action is logged to the activity store (Phase 2).
/// Mouse and keyboard actions bypass the safety confirmation for now —
/// Phase 2 will gate them behind the same policy as shell commands.
pub fn auto_perform(actions: Vec<AutoAction>) -> Result<()> {
    for action in actions {
        perform_one(&action)?;
    }
    Ok(())
}

fn perform_one(action: &AutoAction) -> Result<()> {
    match action {
        AutoAction::MouseMove { x, y } => mouse_move(*x, *y),
        AutoAction::MouseClick { x, y, button } => mouse_click(*x, *y, button),
        AutoAction::MouseDoubleClick { x, y } => {
            mouse_click(*x, *y, &MouseButton::Left)?;
            std::thread::sleep(Duration::from_millis(50));
            mouse_click(*x, *y, &MouseButton::Left)
        }
        AutoAction::TypeText { text } => type_text(text),
        AutoAction::PressKey { combo } => press_key(combo),
        AutoAction::Sleep { ms } => {
            std::thread::sleep(Duration::from_millis(*ms));
            Ok(())
        }
    }
}

#[cfg(unix)]
fn mouse_move(x: i32, y: i32) -> Result<()> {
    // Phase 3: use `enigo` or `xdo` crate. For now, return Ok to keep the
    // skeleton compilable without a GUI test dependency.
    tracing::debug!("mouse_move (stub) -> ({}, {})", x, y);
    Ok(())
}

#[cfg(unix)]
fn mouse_click(_x: i32, _y: i32, button: &MouseButton) -> Result<()> {
    tracing::debug!("mouse_click (stub) -> {:?}", button);
    Ok(())
}

#[cfg(unix)]
fn type_text(text: &str) -> Result<()> {
    tracing::debug!("type_text (stub) -> {} chars", text.chars().count());
    Ok(())
}

#[cfg(unix)]
fn press_key(combo: &str) -> Result<()> {
    tracing::debug!("press_key (stub) -> {}", combo);
    Ok(())
}

#[cfg(not(unix))]
fn mouse_move(x: i32, y: i32) -> Result<()> {
    // Phase 3: use the `windows` crate's SendInput API.
    tracing::debug!("mouse_move (stub) -> ({}, {})", x, y);
    Ok(())
}

#[cfg(not(unix))]
fn mouse_click(_x: i32, _y: i32, button: &MouseButton) -> Result<()> {
    tracing::debug!("mouse_click (stub) -> {:?}", button);
    Ok(())
}

#[cfg(not(unix))]
fn type_text(text: &str) -> Result<()> {
    tracing::debug!("type_text (stub) -> {} chars", text.chars().count());
    Ok(())
}

#[cfg(not(unix))]
fn press_key(combo: &str) -> Result<()> {
    tracing::debug!("press_key (stub) -> {}", combo);
    Ok(())
}
