// Password input dialog for sudo authentication

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub struct PasswordDialog {
    password: String,
    show_error: bool,
    error_message: String,
}

impl PasswordDialog {
    pub fn new() -> Self {
        Self {
            password: String::new(),
            show_error: false,
            error_message: String::new(),
        }
    }

    pub fn add_char(&mut self, c: char) {
        self.password.push(c);
        self.show_error = false;
    }

    pub fn backspace(&mut self) {
        self.password.pop();
        self.show_error = false;
    }

    pub fn get_password(&self) -> &str {
        &self.password
    }

    pub fn clear(&mut self) {
        self.password.clear();
        self.show_error = false;
        self.error_message.clear();
    }

    pub fn show_error(&mut self, message: String) {
        self.show_error = true;
        self.error_message = message;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, prompt_message: &str) {
        // Center the dialog
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Min(8),
                Constraint::Percentage(40),
            ])
            .split(area);

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(vertical[1]);

        let dialog_area = horizontal[1];

        // Clear the background
        frame.render_widget(Clear, dialog_area);

        // Create the dialog box
        let block = Block::default()
            .title(" Authentication Required ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Layout for content
        let content_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Message
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Password label
                Constraint::Length(1), // Password input
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Error or help
                Constraint::Length(1), // Help text
            ])
            .split(inner);

        // Message
        let message = Paragraph::new(prompt_message)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);
        frame.render_widget(message, content_layout[0]);

        // Password label
        let label = Paragraph::new("Password:")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        frame.render_widget(label, content_layout[2]);

        // Password input (masked)
        let masked = "*".repeat(self.password.len());
        let cursor = if self.password.is_empty() { "█" } else { "" };
        let input_text = format!("{}{}", masked, cursor);

        let input = Paragraph::new(input_text)
            .style(Style::default().fg(Color::White).bg(Color::DarkGray));
        frame.render_widget(input, content_layout[3]);

        // Error or help
        if self.show_error {
            let error = Paragraph::new(Line::from(vec![
                Span::styled("✗ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(&self.error_message, Style::default().fg(Color::Red)),
            ]))
            .alignment(Alignment::Center);
            frame.render_widget(error, content_layout[5]);
        }

        // Help text
        let help = Paragraph::new("[Enter] Submit | [Esc] Cancel")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(help, content_layout[6]);
    }
}
