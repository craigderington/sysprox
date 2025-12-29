// New service creation form

use crate::events::Action;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
enum FormField {
    Name,
    Description,
    ExecStart,
    WorkingDirectory,
    Restart,
    Environment,
}

#[derive(Debug, Clone)]
pub struct NewServiceForm {
    // Form fields
    pub name: String,
    pub description: String,
    pub exec_start: String,
    pub working_directory: String,
    pub restart: String, // "no", "on-failure", "always"
    pub environment: String, // Space-separated KEY=VALUE pairs

    // UI state
    current_field: FormField,
    restart_options: Vec<&'static str>,
    restart_selected: usize,
    error_message: Option<String>,
}

impl Default for NewServiceForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            exec_start: String::new(),
            working_directory: String::new(),
            restart: "no".to_string(),
            environment: String::new(),
            current_field: FormField::Name,
            restart_options: vec!["no", "on-failure", "always", "on-abnormal", "on-abort"],
            restart_selected: 0,
            error_message: None,
        }
    }
}

impl NewServiceForm {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_key(&mut self, key: char) -> Option<Action> {
        // Clear error on any input
        self.error_message = None;

        match key {
            '\n' => {
                // Enter key - move to next field or submit
                self.next_field();
                None
            }
            '\x7f' | '\x08' => {
                // Backspace
                self.delete_char();
                None
            }
            '\t' => {
                // Tab - move to next field
                self.next_field();
                None
            }
            _ => {
                // Regular character input
                if !key.is_control() {
                    self.insert_char(key);
                }
                None
            }
        }
    }

    pub fn handle_special_key(&mut self, key: &str) -> Option<Action> {
        match key {
            "up" => {
                if self.current_field == FormField::Restart {
                    self.restart_selected = self.restart_selected.saturating_sub(1);
                    self.restart = self.restart_options[self.restart_selected].to_string();
                } else {
                    self.prev_field();
                }
                None
            }
            "down" => {
                if self.current_field == FormField::Restart {
                    if self.restart_selected < self.restart_options.len() - 1 {
                        self.restart_selected += 1;
                        self.restart = self.restart_options[self.restart_selected].to_string();
                    }
                } else {
                    self.next_field();
                }
                None
            }
            "esc" => Some(Action::Back),
            _ => None,
        }
    }

    fn insert_char(&mut self, c: char) {
        match self.current_field {
            FormField::Name => self.name.push(c),
            FormField::Description => self.description.push(c),
            FormField::ExecStart => self.exec_start.push(c),
            FormField::WorkingDirectory => self.working_directory.push(c),
            FormField::Restart => {}, // Handled by up/down arrows
            FormField::Environment => self.environment.push(c),
        }
    }

    fn delete_char(&mut self) {
        match self.current_field {
            FormField::Name => { self.name.pop(); },
            FormField::Description => { self.description.pop(); },
            FormField::ExecStart => { self.exec_start.pop(); },
            FormField::WorkingDirectory => { self.working_directory.pop(); },
            FormField::Restart => {}, // Handled by up/down arrows
            FormField::Environment => { self.environment.pop(); },
        }
    }

    fn next_field(&mut self) {
        self.current_field = match self.current_field {
            FormField::Name => FormField::Description,
            FormField::Description => FormField::ExecStart,
            FormField::ExecStart => FormField::WorkingDirectory,
            FormField::WorkingDirectory => FormField::Restart,
            FormField::Restart => FormField::Environment,
            FormField::Environment => FormField::Name, // Loop back
        };
    }

    fn prev_field(&mut self) {
        self.current_field = match self.current_field {
            FormField::Name => FormField::Environment,
            FormField::Description => FormField::Name,
            FormField::ExecStart => FormField::Description,
            FormField::WorkingDirectory => FormField::ExecStart,
            FormField::Restart => FormField::WorkingDirectory,
            FormField::Environment => FormField::Restart,
        };
    }

    pub fn validate(&mut self) -> Result<(), String> {
        // Service name is required
        if self.name.trim().is_empty() {
            return Err("Service name is required".to_string());
        }

        // Service name should only contain alphanumeric, dash, underscore
        if !self.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err("Service name can only contain letters, numbers, dash, and underscore".to_string());
        }

        // ExecStart is required
        if self.exec_start.trim().is_empty() {
            return Err("ExecStart command is required".to_string());
        }

        // Description is recommended but not required
        if self.description.trim().is_empty() {
            self.description = format!("User service: {}", self.name);
        }

        Ok(())
    }

    pub fn generate_service_file(&self) -> String {
        let mut content = String::new();

        content.push_str("[Unit]\n");
        content.push_str(&format!("Description={}\n", self.description));
        content.push('\n');

        content.push_str("[Service]\n");
        content.push_str("Type=simple\n");
        content.push_str(&format!("ExecStart={}\n", self.exec_start));

        if !self.working_directory.trim().is_empty() {
            content.push_str(&format!("WorkingDirectory={}\n", self.working_directory));
        }

        content.push_str(&format!("Restart={}\n", self.restart));

        if !self.environment.trim().is_empty() {
            for env_var in self.environment.split_whitespace() {
                if env_var.contains('=') {
                    content.push_str(&format!("Environment=\"{}\"\n", env_var));
                }
            }
        }

        content.push('\n');
        content.push_str("[Install]\n");
        content.push_str("WantedBy=default.target\n");

        content
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(0),     // Form fields
                Constraint::Length(3),  // Help/Error
            ])
            .split(area);

        self.render_header(frame, chunks[0]);
        self.render_form(frame, chunks[1]);
        self.render_footer(frame, chunks[2]);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = Paragraph::new("📝 Create New User Service")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, area);
    }

    #[allow(clippy::vec_init_then_push)]
    fn render_form(&self, frame: &mut Frame, area: Rect) {
        let mut lines = vec![];

        // Service Name
        lines.push(self.render_field_label("Service Name", FormField::Name));
        lines.push(self.render_field_value(&self.name, FormField::Name, "my-app"));
        lines.push(Line::from(""));

        // Description
        lines.push(self.render_field_label("Description", FormField::Description));
        lines.push(self.render_field_value(&self.description, FormField::Description, "My custom service"));
        lines.push(Line::from(""));

        // ExecStart
        lines.push(self.render_field_label("ExecStart (command)", FormField::ExecStart));
        lines.push(self.render_field_value(&self.exec_start, FormField::ExecStart, "/usr/bin/myapp --daemon"));
        lines.push(Line::from(""));

        // WorkingDirectory
        lines.push(self.render_field_label("WorkingDirectory (optional)", FormField::WorkingDirectory));
        lines.push(self.render_field_value(&self.working_directory, FormField::WorkingDirectory, "/home/user/myapp"));
        lines.push(Line::from(""));

        // Restart Policy
        lines.push(self.render_field_label("Restart Policy", FormField::Restart));
        let restart_line = if self.current_field == FormField::Restart {
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("< {} >", self.restart),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                ),
                Span::styled(" (use ↑↓ to change)", Style::default().fg(Color::DarkGray)),
            ])
        } else {
            Line::from(vec![
                Span::raw("  "),
                Span::styled(&self.restart, Style::default().fg(Color::White)),
            ])
        };
        lines.push(restart_line);
        lines.push(Line::from(""));

        // Environment Variables
        lines.push(self.render_field_label("Environment (optional)", FormField::Environment));
        lines.push(self.render_field_value(&self.environment, FormField::Environment, "KEY=value KEY2=value2"));
        lines.push(Line::from(""));

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" Form "))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    fn render_field_label<'a>(&self, label: &'a str, field: FormField) -> Line<'a> {
        let is_current = self.current_field == field;
        let style = if is_current {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        Line::from(vec![
            if is_current {
                Span::styled("▶ ", Style::default().fg(Color::Green))
            } else {
                Span::raw("  ")
            },
            Span::styled(label, style),
        ])
    }

    fn render_field_value(&self, value: &str, field: FormField, placeholder: &str) -> Line<'static> {
        let is_current = self.current_field == field;

        if value.is_empty() {
            Line::from(vec![
                Span::raw("  "),
                Span::styled(placeholder.to_string(), Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
                if is_current {
                    Span::styled("█", Style::default().fg(Color::Green))
                } else {
                    Span::raw("")
                },
            ])
        } else {
            Line::from(vec![
                Span::raw("  "),
                Span::styled(value.to_string(), if is_current {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                }),
                if is_current {
                    Span::styled("█", Style::default().fg(Color::Green))
                } else {
                    Span::raw("")
                },
            ])
        }
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let help_text = if let Some(ref error) = self.error_message {
            Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(error, Style::default().fg(Color::Red)),
            ])
        } else {
            Line::from(vec![
                Span::styled("[Tab/Enter]", Style::default().fg(Color::Cyan)),
                Span::raw(" Next field | "),
                Span::styled("[Ctrl+S]", Style::default().fg(Color::Green)),
                Span::raw(" Save & Create | "),
                Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
                Span::raw(" Cancel"),
            ])
        };

        let paragraph = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(paragraph, area);
    }

    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }
}
