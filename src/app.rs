// Main application state and view routing

use crate::error::Result;
use crate::events::{key_event_to_action, Action, AppEvent};
use crate::systemd::{JournalReader, LogLine, ServiceController, SystemdClient};
use crate::ui::{DashboardState, DetailAction, DetailState, LogsAction, LogsState, HelpState, NewServiceForm};
use crossterm::event::Event as CrosstermEvent;
use ratatui::{layout::{Constraint, Direction, Layout}, style::Style, widgets::{Block, Borders}};
use ratatui::Frame;
use tokio::sync::mpsc;
use anyhow;

/// Application views
#[derive(Debug)]
pub enum View {
    Dashboard(DashboardState),
    Detail(Box<DetailState>),
    Logs(LogsState),
    Help(HelpState),
    NewService(NewServiceForm),
}

impl View {
    /// Get mutable reference to dashboard state
    pub fn dashboard_mut(&mut self) -> Option<&mut DashboardState> {
        match self {
            View::Dashboard(state) => Some(state),
            _ => None,
        }
    }
    
    /// Get reference to dashboard state
    pub fn dashboard(&self) -> Option<&DashboardState> {
        match self {
            View::Dashboard(state) => Some(state),
            _ => None,
        }
    }
}

/// Main application state
pub struct App {
    pub view: View,
    pub should_quit: bool,
    pub client: SystemdClient,
    pub controller: ServiceController,
    pub tx: mpsc::Sender<AppEvent>,
    pub journal_reader: Option<JournalReader>,
    pub status_message: Option<String>,
    pub needs_full_redraw: bool,
}

impl App {
    pub async fn new(tx: mpsc::Sender<AppEvent>) -> Result<Self> {
        let client = SystemdClient::new().await?;
        let controller = ServiceController::new().await?;

        Ok(Self {
            view: View::Dashboard(DashboardState::new()),
            should_quit: false,
            client,
            controller,
            tx,
            journal_reader: None,
            status_message: None,
            needs_full_redraw: true,
        })
    }

    pub async fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Input(crossterm_event) => {
                self.handle_input(crossterm_event)?;
            }
            AppEvent::ServicesLoaded(services) => {
                if let View::Dashboard(dashboard) = &mut self.view {
                    dashboard.set_services(services);
                }
            }
            AppEvent::ServiceDetailLoaded(detail) => {
                if let View::Detail(detail_view) = &mut self.view {
                    detail_view.set_detail(*detail);
                }
            }
            AppEvent::LogLine(line) => {
                if let View::Logs(logs) = &mut self.view {
                    // Convert string line to LogLine struct
                    let log_line = LogLine {
                        timestamp: String::new(),
                        message: line.clone(),
                        priority: None,
                        raw_line: line,
                        is_live: true,
                    };
                    logs.add_line(log_line);
                }
            }
            AppEvent::LogLineParsed(log_line) => {
                if let View::Logs(logs) = &mut self.view {
                    logs.add_line(log_line);
                }
            }
            AppEvent::JournalReaderStarted(reader) => {
                // Store the journal reader to keep it alive
                self.journal_reader = Some(reader);
            }
            AppEvent::Tick => {
                // Reload services on tick (only in dashboard view)
                if matches!(self.view, View::Dashboard(_)) {
                    self.reload_services().await?;
                }
            }
            AppEvent::Quit => {
                self.should_quit = true;
            }
            AppEvent::Error(err) => {
                tracing::error!("Error: {}", err);
                // If we're in detail view and still loading, show error and go back
                if let View::Detail(detail) = &self.view {
                    if detail.loading {
                        self.status_message = Some(format!("✗ {}", err));
                        self.needs_full_redraw = true;
                        self.view = View::Dashboard(DashboardState::new());
                        // Reload services
                        let tx = self.tx.clone();
                        let client = self.client.clone();
                        tokio::spawn(async move {
                            if let Ok(services) = client.list_services().await {
                                tx.send(AppEvent::ServicesLoaded(services)).await.ok();
                            }
                        });
                    }
                }
            }
            AppEvent::ServiceOperationCompleted { service: _, operation: _, success: _ } => {
                // Service operation completed - could show temporary status here
            }
            AppEvent::RequestConfirmation { service: _, operation: _, message: _ } => {
                // Could implement global confirmation dialog here
            }
            AppEvent::StatusMessage(message) => {
                self.status_message = Some(message.clone());
                tracing::info!("Status: {}", message);
            }
            AppEvent::ServiceCreated { name } => {
                self.needs_full_redraw = true;
                self.view = View::Dashboard(DashboardState::new());
                self.status_message = Some(format!("✓ Service {} created successfully", name));
                // Reload services to show the new one
                let services = self.client.list_services().await?;
                self.tx.send(AppEvent::ServicesLoaded(services)).await.ok();
            }
            AppEvent::ServiceCreationFailed { error } => {
                // Show error in the form
                if let View::NewService(form) = &mut self.view {
                    form.set_error(error);
                }
            }
            AppEvent::ShowHelp => {
                self.needs_full_redraw = true;
                self.view = View::Help(HelpState::new());
            }
        }

        Ok(())
    }

    fn handle_input(&mut self, event: CrosstermEvent) -> Result<()> {
        if let CrosstermEvent::Key(key_event) = event {
            // Special handling for search mode in dashboard
            if let View::Dashboard(dashboard) = &mut self.view {
                if dashboard.searching {
                    use crossterm::event::{KeyCode, KeyModifiers};
                    match key_event.code {
                        KeyCode::Char(c) if key_event.modifiers == KeyModifiers::NONE || key_event.modifiers == KeyModifiers::SHIFT => {
                            dashboard.handle_search_input(c);
                            return Ok(());
                        }
                        KeyCode::Backspace => {
                            dashboard.handle_search_backspace();
                            return Ok(());
                        }
                        KeyCode::Tab => {
                            // Tab exits search and selects first result
                            dashboard.finish_search();
                            dashboard.table_state.select(Some(0));
                            return Ok(());
                        }
                        KeyCode::Enter | KeyCode::Esc => {
                            dashboard.finish_search();
                            return Ok(());
                        }
                        _ => {
                            // Ignore other keys in search mode
                            return Ok(());
                        }
                    }
                }
            }

            // Special handling for NewService form input
            if let View::NewService(form) = &mut self.view {
                use crossterm::event::{KeyCode, KeyModifiers};
                match key_event.code {
                    KeyCode::Char(c) if key_event.modifiers == KeyModifiers::NONE || key_event.modifiers == KeyModifiers::SHIFT => {
                        form.handle_key(c);
                        return Ok(());
                    }
                    KeyCode::Backspace => {
                        form.handle_key('\x7f');
                        return Ok(());
                    }
                    KeyCode::Tab | KeyCode::Enter => {
                        form.handle_key('\n');
                        return Ok(());
                    }
                    KeyCode::Up => {
                        form.handle_special_key("up");
                        return Ok(());
                    }
                    KeyCode::Down => {
                        form.handle_special_key("down");
                        return Ok(());
                    }
                    KeyCode::Esc => {
                        // Will be handled by Action::Back in key_event_to_action
                    }
                    KeyCode::Char('s') if key_event.modifiers == KeyModifiers::CONTROL => {
                        // Will be handled by Action::SubmitNewService
                    }
                    _ => {
                        return Ok(());
                    }
                }
            }

            let action = key_event_to_action(key_event);

            match action {
                Action::Quit => {
                    self.should_quit = true;
                }
                Action::ShowHelp => {
                    self.needs_full_redraw = true;
                    self.view = View::Help(HelpState::new());
                }
                Action::NewService => {
                    self.needs_full_redraw = true;
                    self.view = View::NewService(NewServiceForm::new());
                }
                Action::SubmitNewService => {
                    if let View::NewService(form) = &mut self.view {
                        if let Err(e) = form.validate() {
                            form.set_error(e);
                        } else {
                            // Spawn async task to create the service
                            let form_data = form.clone();
                            let tx = self.tx.clone();

                            tokio::spawn(async move {
                                match create_user_service_async(&form_data).await {
                                    Ok(()) => {
                                        tx.send(AppEvent::ServiceCreated { name: form_data.name.clone() }).await.ok();
                                    }
                                    Err(e) => {
                                        tx.send(AppEvent::ServiceCreationFailed { error: e.to_string() }).await.ok();
                                    }
                                }
                            });
                        }
                    }
                }
                Action::Back | Action::GoBack if matches!(self.view, View::Dashboard(_) | View::NewService(_) | View::Help(_)) => {
                    // Handle back/escape for Dashboard, NewService, and Help views
                    match &mut self.view {
                        View::Dashboard(dashboard) => {
                            if dashboard.searching {
                                dashboard.searching = false;
                            } else if !dashboard.search_term.is_empty() {
                                dashboard.search_term.clear();
                                dashboard.smart_select();
                            } else {
                                self.should_quit = true;
                            }
                        }
                        View::NewService(_) | View::Help(_) => {
                            self.needs_full_redraw = true;
                            self.view = View::Dashboard(DashboardState::new());
                            let tx = self.tx.clone();
                            let client = self.client.clone();
                            tokio::spawn(async move {
                                if let Ok(services) = client.list_services().await {
                                    tx.send(AppEvent::ServicesLoaded(services)).await.ok();
                                }
                            });
                        }
                        _ => {}
                    }
                }

                _ => {
                    // Delegate to current view
                    match &mut self.view {
                        View::Dashboard(dashboard) => {
                            if let Some(service_name) = dashboard.handle_action(action) {
                                // Switch to detail view
                                self.switch_to_detail(service_name);
                            }
                        }
                        View::Detail(detail) => {
                            let detail_action = detail.handle_action(action);
                            match detail_action {
                                DetailAction::GoBack => {
                                    // Switch back to dashboard, preserving existing state
                                    let preserved_filter = self.view.dashboard().map(|d| d.filter.clone());
                                    let preserved_search = self.view.dashboard().map(|d| d.search_term.clone());
                                    let preserved_selection = self.view.dashboard().and_then(|d| d.table_state.selected());

                                    let mut new_dashboard = DashboardState::new();
                                    if let Some(filter) = preserved_filter {
                                        new_dashboard.filter = filter;
                                    }
                                    if let Some(search) = preserved_search {
                                        new_dashboard.search_term = search;
                                    }
                                    if let Some(selection) = preserved_selection {
                                        new_dashboard.table_state.select(Some(selection));
                                    }

                                    // Clear status message when switching views
                                    self.status_message = None;
                                    self.needs_full_redraw = true; // Force terminal clear
                                    self.view = View::Dashboard(new_dashboard);
                                    
                                    // Reload services
                                    let tx = self.tx.clone();
                                    let client = self.client.clone();
                                    tokio::spawn(async move {
                                        if let Ok(services) = client.list_services().await {
                                            tx.send(AppEvent::ServicesLoaded(services)).await.ok();
                                        }
                                    });
                                }
                                DetailAction::ViewLogs(service_name) => {
                                    // Switch to logs view
                                    self.switch_to_logs(service_name);
                                }
                                DetailAction::ExecuteServiceControl { service, operation } => {
                                    // Execute service control operation in background
                                    let controller = self.controller.clone();
                                    let tx = self.tx.clone();
                                    let client = self.client.clone();

                                    tokio::spawn(async move {
                                        let result = match operation.as_str() {
                                            "start" => controller.start_service(&service).await,
                                            "stop" => controller.stop_service(&service).await,
                                            "restart" => controller.restart_service(&service).await,
                                            "enable" => controller.enable_service(&service).await,
                                            "disable" => controller.disable_service(&service).await,
                                            "reload" => controller.reload_service(&service).await,
                                            _ => Err(anyhow::anyhow!("Unknown operation: {}", operation)),
                                        };

                                        let success = result.is_ok();

                                        // Send completion event
                                        tx.send(AppEvent::ServiceOperationCompleted {
                                            service: service.clone(),
                                            operation: operation.clone(),
                                            success,
                                        }).await.ok();

                                        // Send status message
                                        let message = if success {
                                            format!("✓ Service '{}' {} successfully", service, operation)
                                        } else {
                                            let error = result.unwrap_err().to_string();
                                            // Truncate very long error messages
                                            let error_display = if error.len() > 150 {
                                                format!("{}...", &error[..150])
                                            } else {
                                                error
                                            };
                                            format!("✗ Failed to {} service '{}': {}", operation, service, error_display)
                                        };

                                        tx.send(AppEvent::StatusMessage(message)).await.ok();

                                        // Reload services to update status
                                        if let Ok(services) = client.list_services().await {
                                            tx.send(AppEvent::ServicesLoaded(services)).await.ok();
                                        }
                                    });
                                }
                                DetailAction::None => {}
                            }
                        }
                        View::Logs(logs) => {
                            let logs_action = logs.handle_action(action.into());
                            // Handle filter actions that require journal restart
                            if logs.needs_restart {
                                logs.needs_restart = false;
                                // Restart journal reader with new filters
                                if let Some(mut reader) = logs.journal_reader.take() {
                                    tokio::spawn(async move {
                                        reader.stop().await.ok();
                                    });
                                }
                                let service_name = logs.service_name.clone();
                                let tx = self.tx.clone();
                                let min_priority = logs.filters.min_priority;
                                let since = logs.filters.since.clone();
                                let until = logs.filters.until.clone();
                                tokio::spawn(async move {
                                    match JournalReader::stream_logs(service_name, tx.clone(), true, min_priority, since, until).await {
                                        Ok(reader) => {
                                            tx.send(AppEvent::JournalReaderStarted(reader)).await.ok();
                                        }
                                        Err(e) => {
                                            tx.send(AppEvent::Error(e)).await.ok();
                                        }
                                    }
                                });
                            }
                            if logs_action == Some(LogsAction::GoBack) {
                                // Stop journal reader - spawn task to stop it
                                if let Some(mut reader) = self.journal_reader.take() {
                                    tokio::spawn(async move {
                                        reader.stop().await.ok();
                                    });
                                }
                                // Switch back to dashboard, preserving state
                                let preserved_filter = self.view.dashboard().map(|d| d.filter.clone());
                                let preserved_search = self.view.dashboard().map(|d| d.search_term.clone());
                                let preserved_selection = self.view.dashboard().and_then(|d| d.table_state.selected());
                                
                                let mut new_dashboard = DashboardState::new();
                                if let Some(filter) = preserved_filter {
                                    new_dashboard.filter = filter;
                                }
                                if let Some(search) = preserved_search {
                                    new_dashboard.search_term = search;
                                }
                                if let Some(selection) = preserved_selection {
                                    new_dashboard.table_state.select(Some(selection));
                                }

                                // Clear status message when switching views
                                self.status_message = None;
                                self.needs_full_redraw = true;
                                self.view = View::Dashboard(new_dashboard);
                                let tx = self.tx.clone();
                                let client = self.client.clone();
                                tokio::spawn(async move {
                                    if let Ok(services) = client.list_services().await {
                                        tx.send(AppEvent::ServicesLoaded(services)).await.ok();
                                    }
                                });
                            }
                        }
                        View::Help(_help) => {
                            // Any key closes help
                            match action {
                                Action::GoBack | Action::ShowHelp | Action::Quit => {
                                    // Clear status message when switching views
                                    self.status_message = None;
                                    self.needs_full_redraw = true;
                                    self.view = View::Dashboard(DashboardState::new());
                                    let tx = self.tx.clone();
                                    let client = self.client.clone();
                                    tokio::spawn(async move {
                                        if let Ok(services) = client.list_services().await {
                                            tx.send(AppEvent::ServicesLoaded(services)).await.ok();
                                        }
                                    });
                                }
                                _ => {}
                            }
                        }
                        View::NewService(_form) => {
                            // Form input is handled at the top level before action conversion
                            // No actions to handle here
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn switch_to_detail(&mut self, service_name: String) {
        // Clear status message when switching views
        self.status_message = None;
        self.needs_full_redraw = true;

        // Switch to detail view (loading state)
        self.view = View::Detail(Box::default());

        // Spawn async task to load service details
        let tx = self.tx.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            match client.get_service_detail(&service_name).await {
                Ok(detail) => {
                    tx.send(AppEvent::ServiceDetailLoaded(Box::new(detail))).await.ok();
                }
                Err(e) => {
                    tx.send(AppEvent::Error(e)).await.ok();
                }
            }
        });
    }

    fn switch_to_logs(&mut self, service_name: String) {
        // Clear status message when switching views
        self.status_message = None;
        self.needs_full_redraw = true;

        // Switch to logs view
        self.view = View::Logs(LogsState::new(service_name.clone()));

        // Start journal reader - store in channel to keep alive
        let tx = self.tx.clone();
        let tx_reader = self.tx.clone();
        tokio::spawn(async move {
            match JournalReader::stream_logs(service_name, tx.clone(), true, None, None, None).await {
                Ok(reader) => {
                    // Send reader back to app to store it
                    tx_reader.send(AppEvent::JournalReaderStarted(reader)).await.ok();
                }
                Err(e) => {
                    tx.send(AppEvent::Error(e)).await.ok();
                }
            }
        });
    }

    async fn reload_services(&mut self) -> Result<()> {
        let services = self.client.list_services().await?;
        self.tx
            .send(AppEvent::ServicesLoaded(services))
            .await
            .ok();
        Ok(())
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Reserve space for status message if present
        let (content_area, status_area) = if self.status_message.is_some() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(4), // Status bar height: 1 top border + 2 content lines + 1 bottom border
                ])
                .split(area);
            (chunks[0], Some(chunks[1]))
        } else {
            (area, None)
        };

        // Determine if we should show the dashboard footer (only when no status message)
        let show_dashboard_footer = status_area.is_none();

        match &mut self.view {
            View::Dashboard(dashboard) => {
                dashboard.render(frame, content_area, show_dashboard_footer);
            }
            View::Detail(detail) => {
                detail.render(frame, content_area);
            }
            View::Logs(logs) => {
                logs.render(frame, content_area);
            }
            View::Help(help) => {
                help.render(frame, content_area);
            }
            View::NewService(form) => {
                form.render(frame, content_area);
            }
        }

        // Render status message if present
        if let Some(status_area) = status_area {
            if let Some(message) = &self.status_message {
                use ratatui::text::{Line, Span};
                use ratatui::widgets::{Paragraph, Wrap};

                // Color based on message content
                let (color, prefix) = if message.contains("✓") {
                    (ratatui::style::Color::Green, "✓")
                } else if message.contains("✗") {
                    (ratatui::style::Color::Red, "✗")
                } else {
                    (ratatui::style::Color::Yellow, "ℹ")
                };

                let status_line = Line::from(vec![
                    Span::styled(prefix, Style::default().fg(color).add_modifier(ratatui::style::Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(message.trim_start_matches("✓ ").trim_start_matches("✗ "), Style::default().fg(color)),
                ]);

                let status_bar = Paragraph::new(status_line)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .title(" Status ")
                        .border_style(Style::default().fg(color)))
                    .wrap(Wrap { trim: true });

                frame.render_widget(status_bar, status_area);
            }
        }
    }
}

// Standalone function for creating user services (can be called from async tasks)
async fn create_user_service_async(form: &NewServiceForm) -> Result<()> {
    use std::fs;
    use std::path::PathBuf;
    use tokio::process::Command;

    // Get user's home directory
    let home_dir = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("Could not determine home directory"))?;

    // Create systemd user directory if it doesn't exist
    let systemd_user_dir = PathBuf::from(&home_dir).join(".config/systemd/user");
    fs::create_dir_all(&systemd_user_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create systemd user directory: {}", e))?;

    // Generate service file path
    let service_file_name = format!("{}.service", form.name);
    let service_file_path = systemd_user_dir.join(&service_file_name);

    // Generate service file content
    let service_content = form.generate_service_file();

    // Write service file
    fs::write(&service_file_path, service_content)
        .map_err(|e| anyhow::anyhow!("Failed to write service file: {}", e))?;

    tracing::info!("Created user service file: {:?}", service_file_path);

    // Check if we're running as root - user systemd won't work
    let current_user = std::env::var("USER").unwrap_or_default();
    let is_root = current_user == "root" || std::env::var("SUDO_USER").is_ok();

    if is_root {
        // Running as root - can't reload user systemd
        tracing::warn!("Running as root - skipping systemctl --user daemon-reload");
        return Err(anyhow::anyhow!(
            "Service file created at {:?}\n\nIMPORTANT: You're running as root/sudo.\nTo activate this service, run as your regular user:\n  systemctl --user daemon-reload\n  systemctl --user start {}",
            service_file_path,
            form.name
        ));
    }

    // Try to run systemctl --user daemon-reload
    let output = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run systemctl daemon-reload: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::error!("systemctl --user daemon-reload failed: {}", stderr);

        // Service file was created successfully, but reload failed
        return Err(anyhow::anyhow!(
            "Service file created at {:?}\n\nBut systemctl --user daemon-reload failed:\n{}\n\nPlease run manually:\n  systemctl --user daemon-reload\n  systemctl --user start {}",
            service_file_path,
            stderr.trim(),
            form.name
        ));
    }

    tracing::info!("Successfully ran systemctl --user daemon-reload");

    Ok(())
}
