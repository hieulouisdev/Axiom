//! Color palette + style constants for the TUI.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub muted: Color,
    pub success: Color,
    pub warn: Color,
    pub danger: Color,
    pub panel_title: Style,
    pub panel_border: Style,
    pub user_msg: Style,
    pub assistant_msg: Style,
    pub system_msg: Style,
    pub input_prompt: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::Reset,
            accent: Color::Cyan,
            muted: Color::DarkGray,
            success: Color::Green,
            warn: Color::Yellow,
            danger: Color::Red,
            panel_title: Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            panel_border: Style::default().fg(Color::DarkGray),
            user_msg: Style::default().fg(Color::Blue),
            assistant_msg: Style::default().fg(Color::Green),
            system_msg: Style::default().fg(Color::DarkGray),
            input_prompt: Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        }
    }
}

impl Theme {
    pub fn brand_header(&self) -> Vec<Line> {
        vec![
            Line::from(vec![
                Span::styled("╔══════════════════════════════════════════════════════════════╗\n", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("║   ", Style::default().fg(Color::Cyan)),
                Span::styled("AEGIS AI", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(" v1.7.0 — Singularity II                  ", Style::default().fg(Color::DarkGray)),
                Span::styled("║\n", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("╚══════════════════════════════════════════════════════════════╝", Style::default().fg(Color::Cyan)),
            ]),
        ]
    }
}
