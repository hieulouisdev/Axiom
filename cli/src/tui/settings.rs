//! Settings panel — provider config + version info.

use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::commands::Context;

use super::theme::Theme;

#[derive(Default)]
pub struct SettingsPanel;

impl SettingsPanel {
    pub fn handle_key(&mut self, _key: crossterm::event::KeyEvent, _ctx: &Context) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme, ctx: &Context) {
        let block = Block::default().borders(Borders::ALL).border_style(theme.panel_border)
            .title(Span::styled(" Settings ", theme.panel_title));

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![
            Span::styled("Aegis AI v1.7.0", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  —  Singularity II"),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Active provider: ", Style::default().fg(Color::DarkGray)),
            Span::styled(ctx.config.active_provider.as_deref().unwrap_or("(none)"),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Database:        ", Style::default().fg(Color::DarkGray)),
            Span::raw(ctx.db_path.to_string_lossy().to_string()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Config:          ", Style::default().fg(Color::DarkGray)),
            Span::raw(ctx.config_path.to_string_lossy().to_string()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Persona user:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(ctx.config.persona_user_id.clone()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Configured providers:",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))));
        let reg = ctx.config.provider_registry();
        for c in reg.list() {
            let star = if Some(c.id.as_str()) == ctx.config.active_provider.as_deref() { "★" } else { " " };
            let key_status = if c.api_key.is_some() { "✓" } else { "·" };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", star), Style::default().fg(Color::Yellow)),
                Span::styled(format!("{:<15}", c.id), Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", key_status),
                    if c.api_key.is_some() { Style::default().fg(Color::Green) } else { Style::default().fg(Color::DarkGray) }),
                Span::raw(c.base_url.as_deref().unwrap_or("(default)")),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("To configure a provider:",
            Style::default().fg(Color::Yellow))));
        lines.push(Line::from("  aegis configure openai --key sk-..."));
        lines.push(Line::from("  aegis configure anthropic --key sk-ant-..."));
        lines.push(Line::from("  aegis configure gemini --key AIza..."));
        lines.push(Line::from("  aegis configure deepseek --key sk-..."));
        lines.push(Line::from("  aegis configure ollama --base-url http://localhost:11434/v1"));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Features:",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))));
        lines.push(Line::from("  ✓ Hierarchical memory (L0→L3)"));
        lines.push(Line::from("  ✓ Skill library with versions"));
        lines.push(Line::from("  ✓ Wiki knowledge base"));
        lines.push(Line::from("  ✓ CodeGraph symbol indexer"));
        lines.push(Line::from("  ✓ World intelligence (news + markets + risk)"));
        lines.push(Line::from("  ✓ MCP server (JSON-RPC over stdio)"));
        lines.push(Line::from("  ✓ 7 built-in AI providers"));
        let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
        f.render_widget(p, area);
    }
}
