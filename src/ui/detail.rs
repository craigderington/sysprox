// Detail view - service details

use crate::events::Action;
use crate::systemd::ServiceDetail;
use crate::ui::{state_color, status_emoji, load_state_color, result_color, sub_state_color};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

#[derive(Debug)]
pub struct DetailState {
    pub detail: Option<ServiceDetail>,
    pub loading: bool,
    pub confirmation_dialog: Option<ConfirmationDialog>,
}

#[derive(Debug)]
pub struct ConfirmationDialog {
    pub service: String,
    pub operation: String,
    pub message: String,
    pub confirmed: bool,
}

impl Default for DetailState {
    fn default() -> Self {
        Self::new()
    }
}

impl DetailState {
    pub fn new() -> Self {
        Self {
            detail: None,
            loading: true,
            confirmation_dialog: None,
        }
    }

    pub fn set_detail(&mut self, detail: ServiceDetail) {
        self.detail = Some(detail);
        self.loading = false;
    }

    pub fn show_confirmation(&mut self, service: String, operation: String, message: String) {
        self.confirmation_dialog = Some(ConfirmationDialog {
            service,
            operation,
            message,
            confirmed: false,
        });
    }

    pub fn hide_confirmation(&mut self) {
        self.confirmation_dialog = None;
    }

    pub fn confirm_action(&mut self) -> bool {
        if let Some(dialog) = &mut self.confirmation_dialog {
            dialog.confirmed = true;
            true
        } else {
            false
        }
    }

    pub fn handle_action(&mut self, action: Action) -> DetailAction {
        // Handle confirmation dialog first
        if self.confirmation_dialog.is_some() {
            match action {
                Action::ConfirmAction => {
                    if self.confirm_action() {
                        // Extract dialog info before clearing
                        let service = self.confirmation_dialog.as_ref().unwrap().service.clone();
                        let operation = self.confirmation_dialog.as_ref().unwrap().operation.clone();
                        self.hide_confirmation();
                        return DetailAction::ExecuteServiceControl { service, operation };
                    }
                    DetailAction::None
                }
                Action::CancelAction | Action::GoBack => {
                    self.hide_confirmation();
                    DetailAction::None
                }
                _ => DetailAction::None,
            }
        } else {
            // Normal action handling
            match action {
                Action::GoBack => DetailAction::GoBack,
                Action::ViewLogs => {
                    if let Some(detail) = &self.detail {
                        DetailAction::ViewLogs(detail.service.name.clone())
                    } else {
                        DetailAction::None
                    }
                }
                Action::StartService => {
                    if let Some(detail) = &self.detail {
                        // Only allow start if service is NOT active
                        if !detail.service.is_active() {
                            self.show_confirmation(
                                detail.service.name.clone(),
                                "start".to_string(),
                                format!("Start service '{}'?", detail.service.name),
                            );
                        }
                    }
                    DetailAction::None
                }
                Action::StopService => {
                    if let Some(detail) = &self.detail {
                        // Only allow stop if service IS active
                        if detail.service.is_active() {
                            self.show_confirmation(
                                detail.service.name.clone(),
                                "stop".to_string(),
                                format!("Stop service '{}'?", detail.service.name),
                            );
                        }
                    }
                    DetailAction::None
                }
                Action::RestartService => {
                    if let Some(detail) = &self.detail {
                        // Only allow restart if service IS active
                        if detail.service.is_active() {
                            self.show_confirmation(
                                detail.service.name.clone(),
                                "restart".to_string(),
                                format!("Restart service '{}'?", detail.service.name),
                            );
                        }
                    }
                    DetailAction::None
                }
                Action::EnableService => {
                    if let Some(detail) = &self.detail {
                        // Only allow enable if service is NOT already enabled
                        if !detail.service.enabled {
                            self.show_confirmation(
                                detail.service.name.clone(),
                                "enable".to_string(),
                                format!("Enable service '{}' to start on boot?", detail.service.name),
                            );
                        }
                    }
                    DetailAction::None
                }
                Action::DisableService => {
                    if let Some(detail) = &self.detail {
                        // Only allow disable if service IS enabled
                        if detail.service.enabled {
                            self.show_confirmation(
                                detail.service.name.clone(),
                                "disable".to_string(),
                                format!("Disable service '{}' from starting on boot?", detail.service.name),
                            );
                        }
                    }
                    DetailAction::None
                }
                Action::ReloadService => {
                    if let Some(detail) = &self.detail {
                        // Only allow reload if service IS active
                        if detail.service.is_active() {
                            self.show_confirmation(
                                detail.service.name.clone(),
                                "reload".to_string(),
                                format!("Reload service '{}' configuration?", detail.service.name),
                            );
                        }
                    }
                    DetailAction::None
                }
                _ => DetailAction::None,
            }
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // If confirmation dialog is active, render it on top
        if let Some(dialog) = &self.confirmation_dialog {
            self.render_confirmation_dialog(frame, area, dialog);
            return;
        }

        if self.loading {
            let loading = Paragraph::new("Loading service details...")
                .block(Block::default().borders(Borders::ALL).title(" Detail "))
                .style(Style::default().fg(ratatui::style::Color::Gray));
            frame.render_widget(loading, area);
            return;
        }

        let Some(detail) = &self.detail else {
            let error = Paragraph::new("No service details available")
                .block(Block::default().borders(Borders::ALL).title(" Detail "))
                .style(Style::default().fg(ratatui::style::Color::Red));
            frame.render_widget(error, area);
            return;
        };

        // Split area into sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Length(7),  // Status
                Constraint::Length(15), // Metrics (split columns with dot matrix graphs)
                Constraint::Length(5),  // Dependencies
                Constraint::Min(6),     // Service Configuration
                Constraint::Length(2),  // Help
            ])
            .split(area);

        // Render each section
        self.render_header(frame, chunks[0], detail);
        self.render_status(frame, chunks[1], detail);
        self.render_metrics(frame, chunks[2], detail);
        self.render_dependencies(frame, chunks[3], detail);
        self.render_service_config(frame, chunks[4], detail);
        self.render_help(frame, chunks[5], detail);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect, detail: &ServiceDetail) {
        let icon = status_emoji(&detail.service.active_state);
        let title = format!("{} {}", icon, detail.service.name);

        let subtitle = format!(
            "{} - {}",
            detail.service.active_state, detail.service.description
        );

        let text = format!("{}\n{}", title, subtitle);

        let header = Paragraph::new(text)
            .style(
                Style::default()
                    .fg(state_color(&detail.service.active_state))
                    .add_modifier(Modifier::BOLD),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(header, area);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect, detail: &ServiceDetail) {
        use ratatui::text::{Line, Span};

        let pid_str = if detail.main_pid > 0 {
            detail.main_pid.to_string()
        } else {
            "-".to_string()
        };

        let lines = vec![
            Line::from(vec![
                Span::styled("Load State:   ", Style::default().fg(Color::Cyan)),
                Span::styled(&detail.service.load_state, Style::default().fg(load_state_color(&detail.service.load_state))),
            ]),
            Line::from(vec![
                Span::styled("Active State: ", Style::default().fg(Color::Cyan)),
                Span::styled(&detail.service.active_state, Style::default().fg(state_color(&detail.service.active_state))),
            ]),
            Line::from(vec![
                Span::styled("Sub State:    ", Style::default().fg(Color::Cyan)),
                Span::styled(&detail.service.sub_state, Style::default().fg(sub_state_color(&detail.service.sub_state))),
            ]),
            Line::from(vec![
                Span::styled("Main PID:     ", Style::default().fg(Color::Cyan)),
                Span::styled(&pid_str, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Result:       ", Style::default().fg(Color::Cyan)),
                Span::styled(&detail.result, Style::default().fg(result_color(&detail.result))),
            ]),
        ];

        let status = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" Status "))
            .wrap(Wrap { trim: false });

        frame.render_widget(status, area);
    }

    /// Generate a horizontal bar graph for percentage visualization
    /// Returns a single clean horizontal bar representing 0-100%
    fn render_bar_graph(percent: f64, width: usize) -> ratatui::text::Line<'static> {
        use ratatui::text::{Line, Span};

        let filled = ((percent / 100.0) * width as f64).round() as usize;
        let empty = width.saturating_sub(filled);

        // Create filled portion (cyan/blue)
        let mut filled_bar = String::new();
        for _ in 0..filled {
            filled_bar.push('█');
        }

        // Create empty portion (dark gray)
        let mut empty_bar = String::new();
        for _ in 0..empty {
            empty_bar.push('░');
        }

        Line::from(vec![
            Span::raw("  "),
            Span::styled(filled_bar, Style::default().fg(ratatui::style::Color::Cyan)),
            Span::styled(empty_bar, Style::default().fg(ratatui::style::Color::DarkGray)),
        ])
    }

    fn render_metrics(&self, frame: &mut Frame, area: Rect, detail: &ServiceDetail) {
        use ratatui::text::{Line, Span};

        // Split area into two columns
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // === LEFT COLUMN: Memory ===
        let memory_mb = detail.memory_current / 1024 / 1024;
        let has_memory_limit = detail.memory_limit != u64::MAX && detail.memory_limit > 0;

        let memory_percent = if has_memory_limit {
            Some((detail.memory_current as f64 / detail.memory_limit as f64) * 100.0)
        } else {
            None
        };

        let mut memory_lines = vec![];

        // Memory usage value with optional percentage
        if has_memory_limit {
            let percent = memory_percent.unwrap_or(0.0);

            memory_lines.push(Line::from(vec![
                Span::styled("Usage: ", Style::default().fg(ratatui::style::Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)),
                Span::styled(format!("{} MB", memory_mb), Style::default().fg(ratatui::style::Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
                Span::styled(" / ", Style::default().fg(ratatui::style::Color::DarkGray)),
                Span::styled(format!("{} MB", detail.memory_limit / 1024 / 1024), Style::default().fg(ratatui::style::Color::Gray)),
                Span::raw("  "),
                Span::styled(format!("({:.1}%)", percent), Style::default().fg(match percent {
                    p if p < 50.0 => ratatui::style::Color::Green,
                    p if p < 75.0 => ratatui::style::Color::Yellow,
                    p if p < 90.0 => ratatui::style::Color::Red,
                    _ => ratatui::style::Color::Red,
                }).add_modifier(ratatui::style::Modifier::BOLD)),
            ]));

            // Add horizontal bar graph
            memory_lines.push(Self::render_bar_graph(percent, 40));
        } else {
            // No memory limit - show usage with a simple bar graph
            // Use a reasonable assumed max (2GB) to show relative usage
            memory_lines.push(Line::from(vec![
                Span::styled("Usage: ", Style::default().fg(ratatui::style::Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)),
                Span::styled(format!("{} MB", memory_mb), Style::default().fg(ratatui::style::Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
                Span::styled(" (no limit)", Style::default().fg(ratatui::style::Color::DarkGray)),
            ]));

            // Show bar graph based on assumed 2GB max for visualization
            let assumed_max_mb = 2048.0;
            let relative_percent = ((memory_mb as f64 / assumed_max_mb) * 100.0).min(100.0);
            memory_lines.push(Self::render_bar_graph(relative_percent, 40));
        }

        memory_lines.push(Line::from(""));
        memory_lines.push(Line::from(vec![
            Span::styled("Tasks: ", Style::default().fg(ratatui::style::Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::styled(format!("{}", detail.tasks_current), Style::default().fg(ratatui::style::Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            if detail.tasks_max > 0 {
                Span::styled(format!(" / {}", detail.tasks_max), Style::default().fg(ratatui::style::Color::Gray))
            } else {
                Span::styled(" / ∞", Style::default().fg(ratatui::style::Color::Gray))
            },
        ]));

        let memory_widget = ratatui::widgets::Paragraph::new(memory_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Memory ")
                    .border_style(Style::default().fg(ratatui::style::Color::Cyan))
                    .title_style(Style::default().fg(ratatui::style::Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(memory_widget, columns[0]);

        // === RIGHT COLUMN: CPU ===
        let cpu_seconds = detail.cpu_usage_nsec as f64 / 1_000_000_000.0;
        let cpu_display = if cpu_seconds < 60.0 {
            format!("{:.2}s", cpu_seconds)
        } else if cpu_seconds < 3600.0 {
            format!("{:.1}m", cpu_seconds / 60.0)
        } else if cpu_seconds < 86400.0 {
            format!("{:.1}h", cpu_seconds / 3600.0)
        } else {
            format!("{:.1}d", cpu_seconds / 86400.0)
        };

        let cpu_avg_percent = if let Some(uptime) = detail.uptime() {
            let uptime_secs = uptime.as_secs() as f64;
            tracing::debug!("CPU calculation for {}: cpu_usage_nsec={}, cpu_seconds={:.2}, uptime_secs={:.2}",
                detail.service.name, detail.cpu_usage_nsec, cpu_seconds, uptime_secs);
            if uptime_secs > 0.0 {
                let percent = (cpu_seconds / uptime_secs) * 100.0;
                tracing::debug!("CPU percentage for {}: {:.2}%", detail.service.name, percent);
                Some(percent)
            } else {
                tracing::debug!("Uptime is 0 for {}, returning None", detail.service.name);
                None
            }
        } else {
            tracing::debug!("No uptime available for {}, returning None for CPU", detail.service.name);
            None
        };

        let cpu_color = if let Some(percent) = cpu_avg_percent {
            match percent {
                p if p < 50.0 => ratatui::style::Color::Green,
                p if p < 75.0 => ratatui::style::Color::Yellow,
                p if p < 90.0 => ratatui::style::Color::LightRed,
                _ => ratatui::style::Color::Red,
            }
        } else {
            ratatui::style::Color::DarkGray
        };

        let mut cpu_lines = vec![];

        // CPU average percentage
        if let Some(percent) = cpu_avg_percent {
            cpu_lines.push(Line::from(vec![
                Span::styled("Average: ", Style::default().fg(ratatui::style::Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)),
                Span::styled(format!("{:.2}%", percent), Style::default().fg(cpu_color).add_modifier(ratatui::style::Modifier::BOLD)),
            ]));

            // Add horizontal bar graph for CPU
            cpu_lines.push(Self::render_bar_graph(percent, 40));
        } else {
            cpu_lines.push(Line::from(vec![
                Span::styled("Average: ", Style::default().fg(ratatui::style::Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)),
                Span::styled("N/A", Style::default().fg(ratatui::style::Color::DarkGray)),
            ]));
        }

        cpu_lines.push(Line::from(""));
        cpu_lines.push(Line::from(vec![
            Span::styled("Total Time", Style::default().fg(ratatui::style::Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)),
        ]));
        cpu_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(&cpu_display, Style::default().fg(ratatui::style::Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
        ]));
        cpu_lines.push(Line::from(""));
        cpu_lines.push(Line::from(vec![
            Span::styled("Restarts", Style::default().fg(ratatui::style::Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)),
        ]));
        cpu_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{}", detail.n_restarts), Style::default().fg(ratatui::style::Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
        ]));

        let cpu_widget = ratatui::widgets::Paragraph::new(cpu_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" CPU ")
                    .border_style(Style::default().fg(ratatui::style::Color::Cyan))
                    .title_style(Style::default().fg(ratatui::style::Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(cpu_widget, columns[1]);
    }

    fn render_dependencies(&self, frame: &mut Frame, area: Rect, detail: &ServiceDetail) {
        let wants = if !detail.wants.is_empty() {
            let base = detail.wants.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            let suffix = if detail.wants.len() > 3 {
                format!(" (+{} more)", detail.wants.len() - 3)
            } else {
                String::new()
            };
            format!("Wants: {}{}", base, suffix)
        } else {
            "Wants: (none)".to_string()
        };

        let after = if !detail.after.is_empty() {
            let base = detail.after.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            let suffix = if detail.after.len() > 3 {
                format!(" (+{} more)", detail.after.len() - 3)
            } else {
                String::new()
            };
            format!("After: {}{}", base, suffix)
        } else {
            "After: (none)".to_string()
        };

        let text = format!("{}\n{}", wants, after);

        let deps = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Dependencies "),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(deps, area);
    }

    fn render_service_config(&self, frame: &mut Frame, area: Rect, detail: &ServiceDetail) {
        use ratatui::text::{Line, Span};

        let mut lines = vec![];

        // ExecStart
        lines.push(Line::from(vec![
            Span::styled("ExecStart:    ", Style::default().fg(Color::Cyan)),
            Span::styled(&detail.exec_main_start, Style::default().fg(Color::White)),
        ]));

        // Type
        if !detail.service_type.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Type:         ", Style::default().fg(Color::Cyan)),
                Span::styled(&detail.service_type, Style::default().fg(Color::White)),
            ]));
        }

        // Restart
        if !detail.restart.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Restart:      ", Style::default().fg(Color::Cyan)),
                Span::styled(&detail.restart, Style::default().fg(Color::White)),
            ]));
        }

        // User
        if !detail.user.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("User:         ", Style::default().fg(Color::Cyan)),
                Span::styled(&detail.user, Style::default().fg(Color::White)),
            ]));
        }

        // Group
        if !detail.group.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Group:        ", Style::default().fg(Color::Cyan)),
                Span::styled(&detail.group, Style::default().fg(Color::White)),
            ]));
        }

        // WorkingDirectory
        if !detail.working_directory.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("WorkingDir:   ", Style::default().fg(Color::Cyan)),
                Span::styled(&detail.working_directory, Style::default().fg(Color::White)),
            ]));
        }

        // Environment variables (show first 2)
        if !detail.environment.is_empty() {
            let env_display = if detail.environment.len() > 2 {
                format!("{}, {} (+{} more)",
                    detail.environment[0],
                    detail.environment[1],
                    detail.environment.len() - 2)
            } else {
                detail.environment.join(", ")
            };
            lines.push(Line::from(vec![
                Span::styled("Environment:  ", Style::default().fg(Color::Cyan)),
                Span::styled(env_display, Style::default().fg(Color::Gray)),
            ]));
        }

        let config = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Service Configuration "),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(config, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect, detail: &ServiceDetail) {
        use ratatui::text::{Line, Span};

        let is_active = detail.service.is_active();
        let is_enabled = detail.service.enabled;

        // Build help line with conditional formatting
        let mut spans = vec![
            Span::styled("[l] Logs | ", Style::default().fg(ratatui::style::Color::DarkGray)),
        ];

        // Start - only if not active
        if !is_active {
            spans.push(Span::styled("[S] Start | ", Style::default().fg(ratatui::style::Color::DarkGray)));
        }

        // Stop - only if active
        if is_active {
            spans.push(Span::styled("[T] Stop | ", Style::default().fg(ratatui::style::Color::DarkGray)));
        }

        // Restart - only if active
        if is_active {
            spans.push(Span::styled("[R] Restart | ", Style::default().fg(ratatui::style::Color::DarkGray)));
        }

        // Enable - highlight in bright green if enabled, gray otherwise
        if is_enabled {
            spans.push(Span::styled("[E] Enabled", Style::default().fg(ratatui::style::Color::Green).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled(" | ", Style::default().fg(ratatui::style::Color::DarkGray)));
        } else {
            spans.push(Span::styled("[E] Enable | ", Style::default().fg(ratatui::style::Color::DarkGray)));
        }

        // Disable - only if enabled
        if is_enabled {
            spans.push(Span::styled("[D] Disable | ", Style::default().fg(ratatui::style::Color::DarkGray)));
        }

        // Reload - only if active
        if is_active {
            spans.push(Span::styled("[L] Reload | ", Style::default().fg(ratatui::style::Color::DarkGray)));
        }

        spans.push(Span::styled("[Esc/←] Back | [q] Quit ", Style::default().fg(ratatui::style::Color::DarkGray)));

        let help = Paragraph::new(Line::from(spans))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(help, area);
    }

    fn render_confirmation_dialog(&self, frame: &mut Frame, area: Rect, dialog: &ConfirmationDialog) {
        // Create dialog area (centered)
        let dialog_width = 60.min(area.width - 4);
        let dialog_height = 8.min(area.height - 4);
        let dialog_x = (area.width - dialog_width) / 2;
        let dialog_y = (area.height - dialog_height) / 2;

        let dialog_area = Rect {
            x: area.x + dialog_x,
            y: area.y + dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        // Darken background
        let background = Block::default()
            .style(Style::default().bg(ratatui::style::Color::DarkGray).fg(ratatui::style::Color::Reset));
        frame.render_widget(background, area);

        // Dialog box
        let dialog = Paragraph::new(format!(
            "{}\n\n[y] Yes  [n] No  [Esc] Cancel",
            dialog.message
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Confirm {} ", dialog.operation))
                .border_style(Style::default().fg(ratatui::style::Color::Yellow))
                .style(
                    Style::default()
                        .bg(ratatui::style::Color::Black)
                        .fg(ratatui::style::Color::White)
                )
        )
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: false });

        frame.render_widget(dialog, dialog_area);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DetailAction {
    None,
    GoBack,
    ViewLogs(String),
    ExecuteServiceControl {
        service: String,
        operation: String,
    },
}
