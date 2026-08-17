//! Computer-use subsystem.
//!
//! Lets the AI agent perform actions on the user's machine: launch apps,
//! read/write files, execute shell commands, automate the GUI (mouse/keyboard),
//! and capture the screen. Every potentially destructive operation flows
//! through the [`safety`] module which may require explicit user confirmation.

pub mod apps;
pub mod automation;
pub mod clipboard;
pub mod commands;
pub mod files;
pub mod safety;
pub mod screen;

pub use apps::{list_apps, open_app, AppDescriptor};
pub use automation::{auto_perform, AutoAction};
pub use clipboard::{clipboard_read, clipboard_write, clipboard_watch_start, clipboard_watch_stop, ClipboardContent};
pub use commands::{exec_command, ExecResult};
pub use files::{file_read, file_write, FileReadResult};
pub use safety::{ActionRisk, SafetyCheck, SafetyDecision, SafetyPolicy};
pub use screen::screenshot;
