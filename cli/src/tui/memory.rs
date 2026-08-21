//! Memory panel — browse atoms, scenarios, persona.

use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::commands::Context;

use super::theme::Theme;

#[derive(Default)]
pub struct MemoryPanel {
    pub tab: u8, // 0=atoms, 1=scenarios, 2=persona
}

impl MemoryPanel {
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent, _ctx: &Context) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Char('1') => self.tab = 0,
            KeyCode::Char('2') => self.tab = 1,
            KeyCode::Char('3') => self.tab = 2,
            _ => {}
        }
        Ok(())
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme, ctx: &Context) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(area);

        // Tab bar
        let tab_titles = ["1.Atoms", "2.Scenarios", "3.Persona"];
        let mut spans = Vec::new();
        for (i, t) in tab_titles.iter().enumerate() {
            let style = if i as u8 == self.tab {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            spans.push(Span::styled(format!(" {} ", t), style));
            if i < tab_titles.len() - 1 {
                spans.push(Span::raw("  "));
            }
        }
        let tabs = Block::default().borders(Borders::ALL).border_style(theme.panel_border)
            .title(Span::styled(" Memory ", theme.panel_title));
        let tabs_p = Paragraph::new(vec![Line::from(spans)]).block(tabs);
        f.render_widget(tabs_p, chunks[0]);

        match self.tab {
            0 => self.render_atoms(f, chunks[1], theme, ctx),
            1 => self.render_scenarios(f, chunks[1], theme, ctx),
            2 => self.render_persona(f, chunks[1], theme, ctx),
            _ => {}
        }
    }

    fn render_atoms(&self, f: &mut Frame, area: Rect, theme: &Theme, ctx: &Context) {
        let atoms = ctx.memory.hierarchy.list_atoms(50).unwrap_or_default();
        let block = Block::default().borders(Borders::ALL).border_style(theme.panel_border)
            .title(Span::styled(format!(" Atoms ({}) ", atoms.len()), theme.panel_title));
        let items: Vec<ListItem> = atoms.iter().map(|a| {
            let style = match a.kind {
                crate::memory::AtomKind::Decision => Style::default().fg(Color::Yellow),
                crate::memory::AtomKind::Preference => Style::default().fg(Color::Blue),
                crate::memory::AtomKind::Goal => Style::default().fg(Color::Green),
                _ => Style::default(),
            };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("#{} ", a.id), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("[{}]", a.kind.as_str()), style),
                    Span::raw("  "),
                    Span::raw(&a.summary),
                ]),
                Line::from(vec![
                    Span::styled(format!("    conf={:.2}  recalled={}  created={}",
                        a.confidence, a.recall_count, format_ts(a.created_at_ms)),
                        Style::default().fg(Color::DarkGray)),
                ]),
            ])
        }).collect();
        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }

    fn render_scenarios(&self, f: &mut Frame, area: Rect, theme: &Theme, ctx: &Context) {
        let s = ctx.memory.hierarchy.list_scenarios().unwrap_or_default();
        let block = Block::default().borders(Borders::ALL).border_style(theme.panel_border)
            .title(Span::styled(format!(" Scenarios ({}) ", s.len()), theme.panel_title));
        let items: Vec<ListItem> = s.iter().map(|sc| {
            let tags_str = if sc.tags.is_empty() { String::new() } else { format!(" [{}]", sc.tags.join(",")) };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("#{} ", sc.id), Style::default().fg(Color::DarkGray)),
                    Span::styled(sc.title.clone(), Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(tags_str),
                ]),
                Line::from(format!("   {} atoms — updated {}", sc.atom_count, format_ts(sc.updated_at_ms))),
            ])
        }).collect();
        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }

    fn render_persona(&self, f: &mut Frame, area: Rect, theme: &Theme, ctx: &Context) {
        let p = ctx.memory.hierarchy.load_persona(&ctx.config.persona_user_id).unwrap_or_else(|_| crate::memory::Persona {
            user_id: ctx.config.persona_user_id.clone(),
            traits: vec![],
            updated_at_ms: 0,
        });
        let block = Block::default().borders(Borders::ALL).border_style(theme.panel_border)
            .title(Span::styled(format!(" Persona ({}) ", p.traits.len()), theme.panel_title));
        let mut lines: Vec<Line> = Vec::new();
        if p.traits.is_empty() {
            lines.push(Line::from(Span::styled("(no persona traits yet — chat to auto-build)",
                Style::default().fg(Color::DarkGray))));
        }
        for t in &p.traits {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<20} ", t.key), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(&t.value),
                Span::styled(format!("  (conf={:.2})", t.confidence), Style::default().fg(Color::DarkGray)),
            ]));
        }
        let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
        f.render_widget(p, area);
    }
}

fn format_ts(ms: i64) -> String {
    if ms <= 0 { return "—".into(); }
    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms).unwrap_or_else(|| chrono::Utc::now());
    dt.format("%Y-%m-%d %H:%M").to_string()
}
