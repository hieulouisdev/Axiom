//! GUI automation: mouse / keyboard simulation.
//!
//! Phase 2: Uses `enigo` for cross-platform mouse/keyboard input simulation.

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
    /// Scroll the mouse wheel.
    MouseScroll { x: i32, y: i32, delta: i32 },
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
pub fn auto_perform(actions: Vec<AutoAction>) -> Result<()> {
    let mut enigo = enigo::Enigo::new(&enigo::Settings::default())
        .map_err(|e| AegisError::Internal(format!("enigo init: {e}")))?;

    for action in &actions {
        perform_one(&mut enigo, action)?;
    }
    Ok(())
}

fn perform_one(enigo: &mut enigo::Enigo, action: &AutoAction) -> Result<()> {
    match action {
        AutoAction::MouseMove { x, y } => mouse_move(enigo, *x, *y),
        AutoAction::MouseClick { x, y, button } => mouse_click(enigo, *x, *y, button),
        AutoAction::MouseDoubleClick { x, y } => {
            mouse_click(enigo, *x, *y, &MouseButton::Left)?;
            std::thread::sleep(Duration::from_millis(50));
            mouse_click(enigo, *x, *y, &MouseButton::Left)
        }
        AutoAction::TypeText { text } => type_text(enigo, text),
        AutoAction::PressKey { combo } => press_key(enigo, combo),
        AutoAction::MouseScroll { x, y, delta } => mouse_scroll(enigo, *x, *y, *delta),
        AutoAction::Sleep { ms } => {
            std::thread::sleep(Duration::from_millis(*ms));
            Ok(())
        }
    }
}

fn mouse_move(enigo: &mut enigo::Enigo, x: i32, y: i32) -> Result<()> {
    enigo.move_mouse(x, y, enigo::Coordinate::Abs)
        .map_err(|e| AegisError::Internal(format!("mouse_move: {e}")))?;
    tracing::debug!("mouse_move -> ({}, {})", x, y);
    Ok(())
}

fn mouse_click(enigo: &mut enigo::Enigo, x: i32, y: i32, button: &MouseButton) -> Result<()> {
    // Move to position first
    enigo.move_mouse(x, y, enigo::Coordinate::Abs)
        .map_err(|e| AegisError::Internal(format!("mouse_move: {e}")))?;

    let enigo_button = match button {
        MouseButton::Left => enigo::MouseButton::Left,
        MouseButton::Right => enigo::MouseButton::Right,
        MouseButton::Middle => enigo::MouseButton::Middle,
    };

    enigo.button(enigo_button, enigo::Direction::Press)
        .map_err(|e| AegisError::Internal(format!("mouse_press: {e}")))?;
    enigo.button(enigo_button, enigo::Direction::Release)
        .map_err(|e| AegisError::Internal(format!("mouse_release: {e}")))?;

    tracing::debug!("mouse_click at ({}, {}) {:?}", x, y, button);
    Ok(())
}

fn type_text(enigo: &mut enigo::Enigo, text: &str) -> Result<()> {
    enigo.text(text)
        .map_err(|e| AegisError::Internal(format!("type_text: {e}")))?;
    tracing::debug!("type_text -> {} chars", text.chars().count());
    Ok(())
}

fn press_key(enigo: &mut enigo::Enigo, combo: &str) -> Result<()> {
    use enigo::Key;

    // Parse key combos like "Ctrl+C", "Alt+Tab", "Enter", "Escape"
    let parts: Vec<&str> = combo.split('+').collect();
    let modifiers: Vec<&str> = if parts.len() > 1 {
        parts[..parts.len() - 1].to_vec()
    } else {
        Vec::new()
    };
    let key_str = parts.last().unwrap_or(&"");

    // Press modifiers
    let mod_keys: Vec<Key> = modifiers.iter().filter_map(|m| parse_modifier(m)).collect();
    for mk in &mod_keys {
        enigo.key(*mk, enigo::Direction::Press)
            .map_err(|e| AegisError::Internal(format!("key_press modifier: {e}")))?;
    }

    // Press and release the main key
    if let Some(key) = parse_key(key_str) {
        enigo.key(key, enigo::Direction::Press)
            .map_err(|e| AegisError::Internal(format!("key_press: {e}")))?;
        enigo.key(key, enigo::Direction::Release)
            .map_err(|e| AegisError::Internal(format!("key_release: {e}")))?;
    } else {
        // If we can't parse the key, try typing it as text
        enigo.text(key_str)
            .map_err(|e| AegisError::Internal(format!("key_text: {e}")))?;
    }

    // Release modifiers in reverse order
    for mk in mod_keys.iter().rev() {
        enigo.key(*mk, enigo::Direction::Release)
            .map_err(|e| AegisError::Internal(format!("key_release modifier: {e}")))?;
    }

    tracing::debug!("press_key -> {}", combo);
    Ok(())
}

fn mouse_scroll(enigo: &mut enigo::Enigo, x: i32, y: i32, delta: i32) -> Result<()> {
    // Move to position first
    enigo.move_mouse(x, y, enigo::Coordinate::Abs)
        .map_err(|e| AegisError::Internal(format!("mouse_move: {e}")))?;

    // enigo scrolls vertically with positive = down, negative = up
    enigo.scroll(enigo::ScrollDirection::Vertical, delta)
        .map_err(|e| AegisError::Internal(format!("mouse_scroll: {e}")))?;

    tracing::debug!("mouse_scroll at ({}, {}) delta={}", x, y, delta);
    Ok(())
}

/// Parse a modifier key string to an enigo Key.
fn parse_modifier(s: &str) -> Option<enigo::Key> {
    use enigo::Key;
    match s.to_lowercase().as_str() {
        "ctrl" | "control" => Some(Key::Control),
        "alt" => Some(Key::Alt),
        "shift" => Some(Key::Shift),
        "meta" | "super" | "win" | "cmd" => Some(Key::Meta),
        _ => None,
    }
}

/// Parse a key string to an enigo Key.
fn parse_key(s: &str) -> Option<enigo::Key> {
    use enigo::Key;
    match s.to_lowercase().as_str() {
        "enter" | "return" => Some(Key::Return),
        "tab" => Some(Key::Tab),
        "escape" | "esc" => Some(Key::Escape),
        "backspace" => Some(Key::Backspace),
        "delete" | "del" => Some(Key::Delete),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" | "page_up" => Some(Key::PageUp),
        "pagedown" | "page_down" => Some(Key::PageDown),
        "up" => Some(Key::UpArrow),
        "down" => Some(Key::DownArrow),
        "left" => Some(Key::LeftArrow),
        "right" => Some(Key::RightArrow),
        "space" => Some(Key::Space),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        "capslock" | "caps_lock" => Some(Key::CapsLock),
        "numlock" | "num_lock" => Some(Key::NumLock),
        "scrolllock" | "scroll_lock" => Some(Key::ScrollLock),
        "insert" => Some(Key::Insert),
        "ctrl" | "control" => Some(Key::Control),
        "alt" => Some(Key::Alt),
        "shift" => Some(Key::Shift),
        "meta" | "super" | "win" | "cmd" => Some(Key::Meta),
        // Single character keys
        s if s.len() == 1 => {
            let c = s.chars().next()?;
            if c.is_ascii_alphabetic() {
                // Layout::KeyMap requires a u32 mapping — use unicode
                Some(Key::Unicode(c))
            } else {
                Some(Key::Unicode(c))
            }
        }
        _ => None,
    }
}
