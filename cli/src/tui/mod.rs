//! Beautiful interactive TUI for Aegis AI.
//!
//! Built with `ratatui` + `crossterm`. Five panels:
//!
//! - Chat    — main conversation interface
//! - Memory  — atoms, scenarios, persona
//! - World   — news + markets + risk
//! - Skills  — versioned skill library
//! - Settings— provider config, version info
//!
//! Navigation: Tab/Ctrl+T to switch panels, Esc/q to quit.

pub mod app;
pub mod chat;
pub mod memory;
pub mod world;
pub mod skills;
pub mod settings;
pub mod theme;

use crate::commands::Context;

pub async fn run(ctx: Context) -> anyhow::Result<()> {
    let mut app = app::App::new(ctx);
    app.run().await
}
