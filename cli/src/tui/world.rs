//! World panel — news + markets + risk.

use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::commands::Context;

use super::theme::Theme;

#[derive(Default)]
pub struct WorldPanel {
    pub tab: u8, // 0=news, 1=markets, 2=risk
    pub news: Vec<crate::world::NewsBrief>,
    pub quotes: Vec<crate::world::FinanceQuote>,
    pub last_fetch: String,
}

impl WorldPanel {
    pub async fn handle_key(&mut self, key: crossterm::event::KeyEvent, _ctx: &Context) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Char('1') => self.tab = 0,
            KeyCode::Char('2') => self.tab = 1,
            KeyCode::Char('3') => self.tab = 2,
            KeyCode::Char('r') => {
                self.last_fetch = "Fetching…".into();
                if self.tab == 0 {
                    let agg = crate::world::NewsAggregator::new();
                    self.news = agg.fetch_all(20).await;
                    self.last_fetch = format!("Fetched {} items", self.news.len());
                } else if self.tab == 1 {
                    let snap = crate::world::fetch_market_snapshot().await;
                    self.quotes = snap.quotes;
                    self.last_fetch = format!("Fetched {} quotes", self.quotes.len());
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme, _ctx: &Context) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(area);

        let tab_titles = ["1.News", "2.Markets", "3.Risk"];
        let mut spans = Vec::new();
        for (i, t) in tab_titles.iter().enumerate() {
            let style = if i as u8 == self.tab {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            spans.push(Span::styled(format!(" {} ", t), style));
            if i < tab_titles.len() - 1 { spans.push(Span::raw("  ")); }
        }
        spans.push(Span::raw("    "));
        spans.push(Span::styled("(press 'r' to refresh)", Style::default().fg(Color::Yellow)));
        let tabs = Block::default().borders(Borders::ALL).border_style(theme.panel_border)
            .title(Span::styled(" World Intelligence ", theme.panel_title));
        let tabs_p = Paragraph::new(vec![Line::from(spans)]).block(tabs);
        f.render_widget(tabs_p, chunks[0]);

        match self.tab {
            0 => self.render_news(f, chunks[1], theme),
            1 => self.render_markets(f, chunks[1], theme),
            2 => self.render_risk(f, chunks[1], theme),
            _ => {}
        }
    }

    fn render_news(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default().borders(Borders::ALL).border_style(theme.panel_border)
            .title(Span::styled(format!(" News ({}) ", self.news.len()), theme.panel_title));
        if self.news.is_empty() {
            let p = Paragraph::new(vec![Line::from(Span::styled(
                "  Press 'r' to fetch the latest news.",
                Style::default().fg(Color::DarkGray),
            ))]).block(block);
            f.render_widget(p, area);
            return;
        }
        let items: Vec<ListItem> = self.news.iter().enumerate().map(|(i, b)| {
            let cat_color = match b.category {
                crate::world::NewsCategory::Geopolitics => Color::Red,
                crate::world::NewsCategory::Security => Color::Magenta,
                crate::world::NewsCategory::Finance => Color::Green,
                crate::world::NewsCategory::Disaster => Color::Yellow,
                crate::world::NewsCategory::Tech => Color::Cyan,
                _ => Color::Blue,
            };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("{:>2}. ", i + 1), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("[{}]", b.category.as_str()), Style::default().fg(cat_color)),
                    Span::raw("  "),
                    Span::styled(format!("{:.2} ", b.salience), Style::default().fg(Color::DarkGray)),
                    Span::styled(&b.title, Style::default().add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled(format!("    {} — {}", b.feed_title, b.link),
                        Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(format!("    {}", b.summary.chars().take(150).collect::<String>())),
                Line::from(""),
            ])
        }).collect();
        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }

    fn render_markets(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default().borders(Borders::ALL).border_style(theme.panel_border)
            .title(Span::styled(format!(" Markets ({}) ", self.quotes.len()), theme.panel_title));
        let mut lines: Vec<Line> = Vec::new();
        if self.quotes.is_empty() {
            lines.push(Line::from(Span::styled(
                "  Press 'r' to fetch a market snapshot.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for q in &self.quotes {
                let change_str = match q.change_pct {
                    Some(c) => {
                        let color = if c >= 0.0 { Color::Green } else { Color::Red };
                        let sign = if c >= 0.0 { "+" } else { "" };
                        Span::styled(format!("  {}{:.2}%", sign, c), Style::default().fg(color))
                    }
                    None => Span::raw(""),
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {:<20}", q.symbol), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" {:>12.4} {}", q.price, q.currency), Style::default()),
                    change_str,
                    Span::styled(format!("   via {}", q.source), Style::default().fg(Color::DarkGray)),
                ]));
            }
        }
        let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
        f.render_widget(p, area);
    }

    fn render_risk(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default().borders(Borders::ALL).border_style(theme.panel_border)
            .title(Span::styled(" Country Instability Index ", theme.panel_title));
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Country risk scoring runs from the CLI:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::raw("    aegis world risk --countries VN:Vietnam,US:United States")),
            Line::from(""),
            Line::from(Span::styled(
                "  In TUI, press 'r' to fetch news first, then risk scores",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  are computed automatically from the news volume + sentiment.",
                Style::default().fg(Color::DarkGray),
            )),
        ]).block(block);
        f.render_widget(p, area);
    }
}
