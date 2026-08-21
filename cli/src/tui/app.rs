//! Main app loop — owns the terminal, event poll, and panel switching.

use std::io::{self, stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap};
use ratatui::Terminal;

use crate::commands::Context;

use super::theme::Theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Chat,
    Memory,
    World,
    Skills,
    Settings,
}

impl Panel {
    pub fn all() -> &'static [Panel] {
        &[Panel::Chat, Panel::Memory, Panel::World, Panel::Skills, Panel::Settings]
    }
    pub fn title(self) -> &'static str {
        match self {
            Panel::Chat => "Chat",
            Panel::Memory => "Memory",
            Panel::World => "World",
            Panel::Skills => "Skills",
            Panel::Settings => "Settings",
        }
    }
    pub fn next(self) -> Self {
        let all = Panel::all();
        let i = all.iter().position(|p| *p == self).unwrap_or(0);
        all[(i + 1) % all.len()].clone()
    }
    pub fn prev(self) -> Self {
        let all = Panel::all();
        let i = all.iter().position(|p| *p == self).unwrap_or(0);
        if i == 0 { all[all.len() - 1].clone() } else { all[i - 1].clone() }
    }
}

pub struct App {
    pub ctx: Context,
    pub theme: Theme,
    pub panel: Panel,
    pub input: String,
    pub messages: Vec<(String, bool)>, // (text, is_user)
    pub status: String,
    pub should_quit: bool,
    pub chat_scroll: usize,
    pub memory_state: super::memory::MemoryPanel,
    pub world_state: super::world::WorldPanel,
    pub skills_state: super::skills::SkillsPanel,
    pub settings_state: super::settings::SettingsPanel,
}

impl App {
    pub fn new(ctx: Context) -> Self {
        Self {
            ctx,
            theme: Theme::default(),
            panel: Panel::Chat,
            input: String::new(),
            messages: vec![
                ("Welcome to Aegis AI v1.7.0 — Singularity II.".to_string(), false),
                ("Type a message and press Enter to chat.".to_string(), false),
                ("Tab/Ctrl+T: switch panel  |  q/Esc: quit".to_string(), false),
            ],
            status: String::from("Ready"),
            should_quit: false,
            chat_scroll: 0,
            memory_state: super::memory::MemoryPanel::default(),
            world_state: super::world::WorldPanel::default(),
            skills_state: super::skills::SkillsPanel::default(),
            settings_state: super::settings::SettingsPanel::default(),
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // bootstrap terminal
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.main_loop(&mut terminal).await;

        // restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    async fn main_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> anyhow::Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.render(f))?;

            // Poll events with a short timeout so async tasks can make progress
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key).await?;
                }
            }
        }
        Ok(())
    }

    async fn handle_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        // Global keys
        match key.code {
            KeyCode::Tab => { self.panel = self.panel.next(); return Ok(()); }
            KeyCode::BackTab => { self.panel = self.panel.prev(); return Ok(()); }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.panel = self.panel.next();
                return Ok(());
            }
            KeyCode::Esc => { self.should_quit = true; return Ok(()); }
            KeyCode::Char('q') if self.panel != Panel::Chat => { self.should_quit = true; return Ok(()); }
            _ => {}
        }

        // Panel-specific keys
        match self.panel {
            Panel::Chat => self.handle_chat_key(key).await?,
            Panel::Memory => self.memory_state.handle_key(key, &self.ctx)?,
            Panel::World => self.world_state.handle_key(key, &self.ctx).await?,
            Panel::Skills => self.skills_state.handle_key(key, &self.ctx)?,
            Panel::Settings => self.settings_state.handle_key(key, &self.ctx)?,
        }
        Ok(())
    }

    async fn handle_chat_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Enter => {
                if self.input.trim().is_empty() { return Ok(()); }
                let user_msg = self.input.trim().to_string();
                self.input.clear();
                self.messages.push((user_msg.clone(), true));
                self.status = "Thinking…".into();
                // Build request
                use crate::ai::provider::{ChatMessage, ChatRequest, Role};
                let mem = self.ctx.memory.hierarchy.render_prompt_fragment(&self.ctx.config.persona_user_id, 10).unwrap_or_default();
                let mut messages = Vec::new();
                messages.push(ChatMessage {
                    role: Role::System,
                    content: format!("You are Aegis AI, a secure cross-platform assistant. User context:\n\n{}", mem),
                });
                for (text, is_user) in &self.messages {
                    messages.push(ChatMessage {
                        role: if *is_user { Role::User } else { Role::Assistant },
                        content: text.clone(),
                    });
                }
                let req = ChatRequest {
                    model: String::new(),
                    messages,
                    temperature: Some(0.7),
                    max_tokens: Some(2048),
                };
                let provider_id = self.ctx.config.active_provider.clone().unwrap_or_else(|| "zai".into());
                match self.ctx.router.chat(&req, Some(&provider_id)).await {
                    Ok(resp) => {
                        self.messages.push((resp.content, false));
                        self.status = format!("Ready — {} ({} tokens)", resp.provider,
                            resp.usage.as_ref().map(|u| u.total_tokens).unwrap_or(0));
                        // auto-extract atoms
                        let atoms = crate::memory::deterministic_extract(&user_msg);
                        for (k, s) in atoms {
                            let _ = self.ctx.memory.hierarchy.add_atom(k, &s, None, None, None, 0.7);
                        }
                    }
                    Err(e) => {
                        self.messages.push((format!("⚠ error: {}", e), false));
                        self.status = "Error".into();
                    }
                }
            }
            KeyCode::Char(c) => { self.input.push(c); }
            KeyCode::Backspace => { self.input.pop(); }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.messages.clear();
                self.status = "Chat cleared".into();
            }
            _ => {}
        }
        Ok(())
    }

    fn render(&self, f: &mut ratatui::Frame) {
        let size = f.size();

        // Header
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3), Constraint::Length(3), Constraint::Length(1)])
            .split(size);

        let title_block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.panel_border)
            .title(Span::styled(" Aegis AI v1.7.0 — Singularity II ", self.theme.panel_title));
        let title_p = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  Hierarchical Memory  ·  World Intelligence  ·  MCP  ·  90+ Providers",
                    Style::default().fg(Color::DarkGray)),
            ]),
        ]).block(title_block);
        f.render_widget(title_p, chunks[0]);

        // Tabs
        let titles: Vec<Line> = Panel::all().iter().enumerate().map(|(i, p)| {
            let marker = if *p == self.panel { "▌" } else { " " };
            Line::from(vec![
                Span::styled(format!("{}", i + 1), Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(marker, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(p.title(), if *p == self.panel {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Reset)
                }),
                Span::raw("   "),
            ])
        }).collect();
        let tabs = Tabs::new(titles);
        f.render_widget(tabs, chunks[2]);

        // Body
        match self.panel {
            Panel::Chat => self.render_chat(f, chunks[1]),
            Panel::Memory => self.memory_state.render(f, chunks[1], &self.theme, &self.ctx),
            Panel::World => self.world_state.render(f, chunks[1], &self.theme, &self.ctx),
            Panel::Skills => self.skills_state.render(f, chunks[1], &self.theme, &self.ctx),
            Panel::Settings => self.settings_state.render(f, chunks[1], &self.theme, &self.ctx),
        }

        // Footer
        let footer = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(" Tab ", Style::default().fg(Color::Cyan)),
                Span::raw("switch panel  "),
                Span::styled("Ctrl+L ", Style::default().fg(Color::Cyan)),
                Span::raw("clear chat  "),
                Span::styled("Esc/q ", Style::default().fg(Color::Cyan)),
                Span::raw("quit  |  "),
                Span::styled(&self.status, Style::default().fg(Color::Yellow)),
            ]),
        ]);
        f.render_widget(footer, chunks[3]);
    }

    fn render_chat(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.panel_border)
            .title(Span::styled(" Chat ", self.theme.panel_title));

        let mut lines: Vec<Line> = Vec::new();
        for (text, is_user) in &self.messages {
            let prefix = if *is_user { "▸ You" } else { "✦ Aegis" };
            let style = if *is_user { self.theme.user_msg } else { self.theme.assistant_msg };
            lines.push(Line::from(vec![
                Span::styled(format!("{}: ", prefix), style.add_modifier(Modifier::BOLD)),
                Span::raw(""),
            ]));
            for ln in text.lines() {
                lines.push(Line::from(format!("  {}", ln)));
            }
            lines.push(Line::from(""));
        }
        let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
        f.render_widget(p, chunks[0]);

        // Input box
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(" › Message (Enter to send) ", self.theme.input_prompt));
        let input_p = Paragraph::new(self.input.as_str()).block(input_block);
        f.render_widget(input_p, chunks[1]);
        // cursor
        f.set_cursor(chunks[1].x + self.input.len() as u16 + 1, chunks[1].y + 1);
    }
}
