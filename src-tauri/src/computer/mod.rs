//! Computer-use subsystem.
//!
//! Lets the AI agent perform actions on the user's machine: launch apps,
//! read/write files, execute shell commands, automate the GUI (mouse/keyboard),
//! and capture the screen. Every potentially destructive operation flows
//! through the [`safety`] module which may require explicit user confirmation.
//!
//! v0.3 adds three new safety layers:
//! - [`kill_switch`] — process-wide halt that aborts every running agent loop.
//! - [`rate_limiter`] — token-bucket limiter (30 actions/min by default).
//! - [`audit`] — append-only SQLite record of every AI tool call.

pub mod apps;
pub mod audit;
pub mod automation;
pub mod clipboard;
pub mod commands;
pub mod files;
pub mod kill_switch;
pub mod rate_limiter;
pub mod safety;
pub mod screen;

pub use apps::{AppDescriptor, list_apps, open_app};
pub use automation::{AutoAction, auto_perform};
pub use clipboard::{
    ClipboardContent, clipboard_read, clipboard_watch_start, clipboard_watch_stop, clipboard_write,
};
pub use commands::{ExecResult, exec_command};
pub use files::{FileReadResult, file_read, file_write};
pub use safety::{ActionRisk, SafetyCheck, SafetyDecision, SafetyPolicy};
pub use screen::screenshot;
