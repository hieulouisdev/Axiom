//! Skills panel — list versioned skills.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::commands::Context;

use super::theme::Theme;

#[derive(Default)]
pub struct SkillsPanel;

impl SkillsPanel {
    pub fn handle_key(&mut self, _key: crossterm::event::KeyEvent, _ctx: &Context) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme, ctx: &Context) {
        let skills = ctx.memory.skills.list_skills(false).unwrap_or_default();
        let block = Block::default().borders(Borders::ALL).border_style(theme.panel_border)
            .title(Span::styled(format!(" Skills ({}) ", skills.len()), theme.panel_title));
        let items: Vec<ListItem> = if skills.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  No skills yet. Create one with: aegis skills create <slug> <name> <description>",
                Style::default().fg(Color::DarkGray),
            )))]
        } else {
            skills.iter().map(|s| {
                let status_color = match s.current_status {
                    crate::memory::SkillStatus::Published => Color::Green,
                    crate::memory::SkillStatus::Draft => Color::Yellow,
                    crate::memory::SkillStatus::Deprecated => Color::DarkGray,
                    _ => Color::Blue,
                };
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("[{:>9}] ", s.current_status.as_str()),
                            Style::default().fg(status_color)),
                        Span::styled(&s.slug, Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw("  v"),
                        Span::raw(s.current_version.to_string()),
                        Span::raw("  "),
                        Span::styled(&s.name, Style::default().fg(Color::Cyan)),
                    ]),
                    Line::from(format!("    {}", s.description)),
                ])
            }).collect()
        };
        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }
}
