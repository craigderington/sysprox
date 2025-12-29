// Dashboard view - service list

use crate::events::{Action, FilterAction};
use crate::systemd::{Service, ServiceScope};
use crate::ui::{state_color, status_emoji};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub enum FilterType {
    All,
    Running,
    Stopped,
    Failed,
}

impl FilterType {
    pub fn label(&self) -> &'static str {
        match self {
            FilterType::All => "All",
            FilterType::Running => "Running",
            FilterType::Stopped => "Stopped",
            FilterType::Failed => "Failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeFilter {
    All,
    System,
    User,
}

impl ScopeFilter {
    pub fn label(&self) -> &'static str {
        match self {
            ScopeFilter::All => "All",
            ScopeFilter::System => "System",
            ScopeFilter::User => "User",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ScopeFilter::All => ScopeFilter::System,
            ScopeFilter::System => ScopeFilter::User,
            ScopeFilter::User => ScopeFilter::All,
        }
    }
}

#[derive(Debug)]
pub struct DashboardState {
    pub services: Vec<Service>,
    pub filter: FilterType,
    pub scope_filter: ScopeFilter,
    pub search_term: String,
    pub table_state: TableState,
    pub searching: bool,
}

impl Default for DashboardState {
    fn default() -> Self {
        Self::new()
    }
}

impl DashboardState {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Self {
            services: Vec::new(),
            filter: FilterType::All,
            scope_filter: ScopeFilter::All,
            search_term: String::new(),
            table_state,
            searching: false,
        }
    }

    pub fn set_services(&mut self, mut services: Vec<Service>) {
        // Sort services alphabetically by name
        services.sort_by(|a, b| a.name.cmp(&b.name));
        self.services = services;

        // Smart selection: maintain current selection if possible, otherwise select first
        self.smart_select();
    }

    pub fn smart_select(&mut self) {
        if self.services.is_empty() {
            return;
        }

        let filtered = self.filtered_services();
        if filtered.is_empty() {
            self.table_state.select(Some(0));
            return;
        }

        // Try to maintain current selection
        if let Some(current_idx) = self.table_state.selected() {
            if current_idx < filtered.len() {
                // Current selection is still valid
                return;
            }
        }

        // Otherwise select first item
        self.table_state.select(Some(0));
    }

    pub fn handle_action(&mut self, action: Action) -> Option<String> {
        match action {
            Action::MoveUp => {
                self.move_selection(-1);
                None
            }
            Action::MoveDown => {
                self.move_selection(1);
                None
            }
            Action::MoveTop => {
                self.table_state.select(Some(0));
                None
            }
            Action::MoveBottom => {
                let filtered = self.filtered_services();
                if !filtered.is_empty() {
                    self.table_state.select(Some(filtered.len() - 1));
                }
                None
            }
            Action::ToggleFilter(filter_action) => {
                self.filter = match filter_action {
                    FilterAction::All => FilterType::All,
                    FilterAction::Running => FilterType::Running,
                    FilterAction::Stopped => FilterType::Stopped,
                    FilterAction::Failed => FilterType::Failed,
                };
                self.smart_select();
                None
            }
            Action::ToggleScope => {
                self.scope_filter = self.scope_filter.next();
                self.smart_select();
                None
            }
            Action::Search(_) => {
                // Enter search mode
                self.searching = true;
                self.search_term.clear();
                None
            }
            Action::ClearSearch => {
                // Clear search and exit search mode
                self.searching = false;
                self.search_term.clear();
                self.smart_select();
                None
            }
            Action::Select => {
                // Return selected service name for detail view
                self.get_selected_service().map(|s| s.name.clone())
            }
            Action::CreateService => {
                // TODO: Implement service creation form
                // For now, this is a placeholder
                None
            }
            _ => None,
        }
    }

    pub fn handle_search_input(&mut self, c: char) {
        if self.searching {
            self.search_term.push(c);
            self.smart_select();
        }
    }

    pub fn handle_search_backspace(&mut self) {
        if self.searching && !self.search_term.is_empty() {
            self.search_term.pop();
            self.smart_select();
        }
    }

    pub fn finish_search(&mut self) {
        self.searching = false;
        self.smart_select();
    }

    fn move_selection(&mut self, delta: isize) {
        let filtered = self.filtered_services();
        if filtered.is_empty() {
            return;
        }

        let current = self.table_state.selected().unwrap_or(0);
        let new_index = if delta < 0 {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            (current + delta as usize).min(filtered.len() - 1)
        };

        self.table_state.select(Some(new_index));
    }

    pub fn filtered_services(&self) -> Vec<&Service> {
        self.services
            .iter()
            .filter(|s| self.matches_filter(s))
            .filter(|s| self.matches_scope(s))
            .filter(|s| self.matches_search(s))
            .collect()
    }

    fn matches_filter(&self, service: &Service) -> bool {
        match self.filter {
            FilterType::All => true,
            FilterType::Running => service.is_active(),
            FilterType::Stopped => service.is_inactive(),
            FilterType::Failed => service.is_failed(),
        }
    }

    fn matches_scope(&self, service: &Service) -> bool {
        match self.scope_filter {
            ScopeFilter::All => true,
            ScopeFilter::System => service.scope == ServiceScope::System,
            ScopeFilter::User => service.scope == ServiceScope::User,
        }
    }

    fn matches_search(&self, service: &Service) -> bool {
        if self.search_term.is_empty() {
            return true;
        }

        let search_lower = self.search_term.to_lowercase();
        service.name.to_lowercase().contains(&search_lower)
            || service.description.to_lowercase().contains(&search_lower)
    }

    pub fn get_selected_service(&self) -> Option<&Service> {
        let filtered = self.filtered_services();
        self.table_state
            .selected()
            .and_then(|i| filtered.get(i).copied())
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, show_footer: bool) {
        let constraints = if show_footer {
            vec![
                Constraint::Length(3),  // Header
                Constraint::Length(3),  // Stats/filter (increased for visibility)
                Constraint::Min(0),     // Services table
                Constraint::Length(1),  // Help footer
            ]
        } else {
            vec![
                Constraint::Length(3),  // Header
                Constraint::Length(3),  // Stats/filter (increased for visibility)
                Constraint::Min(0),     // Services table
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        // Render header
        self.render_header(frame, chunks[0]);

        // Render stats/filter
        self.render_stats(frame, chunks[1]);

        // Render services table
        self.render_table(frame, chunks[2]);

        // Render help footer only if requested
        if show_footer {
            self.render_help(frame, chunks[3]);
        }
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = if self.searching {
            "Searching... | [Tab] Select first | [Enter/Esc] Finish | [Backspace] Delete | [c] Clear".to_string()
        } else {
            // Only show [N] New Service when in User scope mode
            let new_service = if self.scope_filter == ScopeFilter::User {
                " | [N] New Service"
            } else {
                ""
            };
            format!("[Enter] Details | [↑↓/jk] Navigate | [a/r/s/f] Filter | [m] Scope{} | [/] Search | [q] Quit", new_service)
        };
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(ratatui::style::Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(help, area);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        use crate::version;

        let build = version::build_info();
        let version_text = format!("v{}  ", build.version); // Extra spaces to prevent truncation

        let title = "⚡ Sysprox - Systemd Service Monitor";

        // Create title paragraph (centered)
        let title_para = Paragraph::new(title)
            .style(Style::default().fg(ratatui::style::Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        // Render title first
        frame.render_widget(title_para, area);

        // Calculate position for version (top-right corner, inside the border)
        let version_x = area.x + area.width.saturating_sub(version_text.len() as u16 + 2);
        let version_y = area.y + 1; // Inside the top border

        // Render version on top of the title in the top-right
        let version_area = Rect {
            x: version_x,
            y: version_y,
            width: version_text.len() as u16,
            height: 1,
        };

        let version_para = Paragraph::new(version_text)
            .style(Style::default().fg(ratatui::style::Color::DarkGray));

        frame.render_widget(version_para, version_area);
    }

    fn render_stats(&self, frame: &mut Frame, area: Rect) {
        let (total, active, inactive, failed) = self.get_stats();
        let filtered = self.filtered_services();

        let search_info = if self.searching {
            format!(" | Search: {}_", self.search_term)
        } else if !self.search_term.is_empty() {
            format!(" | Search: '{}'", self.search_term)
        } else {
            String::new()
        };

        let stats_text = format!(
            "Total: {} | Active: {} | Inactive: {} | Failed: {} | Showing: {} | Filter: {} | Scope: {}{}",
            total, active, inactive, failed, filtered.len(), self.filter.label(), self.scope_filter.label(), search_info
        );

        let stats = Paragraph::new(stats_text)
            .style(Style::default().fg(ratatui::style::Color::Yellow))
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(stats, area);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let filtered = self.filtered_services();

        // Build table rows
        let rows: Vec<Row> = filtered
            .iter()
            .map(|service| {
                let icon = status_emoji(&service.active_state);
                let enabled = if service.enabled { "✓" } else { "" };
                let scope_label = service.scope.label();

                Row::new(vec![
                    Cell::from(format!("{} {}", icon, service.name)),
                    Cell::from(scope_label)
                        .style(Style::default().fg(
                            if service.scope == ServiceScope::User {
                                ratatui::style::Color::LightBlue
                            } else {
                                ratatui::style::Color::Gray
                            }
                        )),
                    Cell::from(service.active_state.clone())
                        .style(Style::default().fg(state_color(&service.active_state))),
                    Cell::from(service.sub_state.clone()),
                    Cell::from(enabled),
                ])
            })
            .collect();

        // Create table
        let widths = [
            Constraint::Percentage(40),
            Constraint::Percentage(10),
            Constraint::Percentage(18),
            Constraint::Percentage(18),
            Constraint::Percentage(14),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["Service", "Scope", "State", "Sub-State", "Enabled"])
                    .style(
                        Style::default()
                            .bg(ratatui::style::Color::DarkGray)
                            .fg(ratatui::style::Color::White)
                            .add_modifier(Modifier::BOLD),
                    )
                    .bottom_margin(1),
            )
            .block(
                Block::default()
                    .title(" Services ")
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .bg(ratatui::style::Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    pub fn get_stats(&self) -> (usize, usize, usize, usize) {
        let total = self.services.len();
        let active = self.services.iter().filter(|s| s.is_active()).count();
        let inactive = self.services.iter().filter(|s| s.is_inactive()).count();
        let failed = self.services.iter().filter(|s| s.is_failed()).count();

        (total, active, inactive, failed)
    }
}
