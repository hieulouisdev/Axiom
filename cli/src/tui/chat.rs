//! Chat panel — placeholder, the chat UI is inlined in app.rs for now.

use ratatui::layout::Rect;
use ratatui::Frame;
use crate::commands::Context;
use super::theme::Theme;

pub struct ChatPanel;

impl ChatPanel {
    pub fn render(_f: &mut Frame, _area: Rect, _theme: &Theme, _ctx: &Context) {}
}
