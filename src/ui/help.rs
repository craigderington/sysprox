// Help view implementation

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

#[derive(Debug)]
pub struct HelpState;

impl Default for HelpState {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpState {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Clear the area first
        frame.render_widget(Clear, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(0),    // Content
                Constraint::Length(1),  // Footer
            ])
            .split(area);

        // Header
        let header = Paragraph::new("Sysprox Help")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(header, chunks[0]);

        // Help content
        let help_content = vec![
            Line::from(vec![
                Span::styled("Navigation", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(": "),
            ]),
            Line::from("  ↑/↓ or j/k    - Move up/down in lists"),
            Line::from("  g/G           - Jump to top/bottom"),
            Line::from("  Enter         - Select item"),
            Line::from("  Esc           - Go back"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Dashboard", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(": "),
            ]),
            Line::from("  a/r/s/f       - Filter: All/Running/Stopped/Failed"),
            Line::from("  /             - Search services"),
            Line::from("  c             - Clear search"),
            Line::from("  l             - View logs for selected service"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Service Control", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(": "),
            ]),
            Line::from("  Shift+S       - Start service"),
            Line::from("  Shift+T       - Stop service"),
            Line::from("  Shift+R       - Restart service"),
            Line::from("  Shift+E       - Enable service"),
            Line::from("  Shift+D       - Disable service"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Logs View", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(": "),
            ]),
            Line::from("  Space         - Toggle follow mode"),
            Line::from("  n/p           - Next/previous search result"),
            Line::from(""),
            Line::from(vec![
                Span::styled("General", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(": "),
            ]),
            Line::from("  F5            - Refresh"),
            Line::from("  V             - Show version"),
            Line::from("  ?             - Show this help"),
            Line::from("  q/Ctrl+C      - Quit"),
        ];

        let help_paragraph = Paragraph::new(help_content)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Keyboard Shortcuts"),
            );

        frame.render_widget(help_paragraph, chunks[1]);

        // Footer
        let footer = Paragraph::new("Esc to return")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(footer, chunks[2]);
    }
}