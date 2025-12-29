// Logs view - log streaming viewer

use crate::events::Action;
use crate::systemd::{JournalReader, LogLine};
use crate::ui::priority_color;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq)]
pub enum LogsAction {
    GoBack,
    ToggleFollow,
    ScrollUp,
    ScrollDown,
    ScrollTop,
    ScrollBottom,
    PageUp,
    PageDown,
    TogglePriorityFilter,
    ClearFilters,
    TimeFilterSince1h,
    TimeFilterSince24h,
    TimeFilterSince7d,
}

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct LogFilters {
    pub min_priority: Option<u8>, // 0=emerg, 1=alert, 2=crit, 3=err, 4=warn, 5=notice, 6=info, 7=debug
    pub since: Option<String>,    // --since filter
    pub until: Option<String>,    // --until filter
}

impl LogFilters {
    pub fn is_active(&self) -> bool {
        self.min_priority.is_some() || self.since.is_some() || self.until.is_some()
    }

    pub fn clear(&mut self) {
        self.min_priority = None;
        self.since = None;
        self.until = None;
    }

    pub fn set_priority_filter(&mut self, priority: Option<u8>) {
        self.min_priority = priority;
    }

    pub fn cycle_priority_filter(&mut self) {
        self.min_priority = match self.min_priority {
            None => Some(3),        // Start with err level
            Some(3) => Some(4),     // err -> warning
            Some(4) => Some(6),     // warning -> info
            Some(6) => Some(7),     // info -> debug
            Some(7) => None,        // debug -> no filter
            _ => Some(3),           // fallback
        };
    }

    pub fn set_time_filter(&mut self, since: Option<String>) {
        self.since = since;
        self.until = None; // Clear until when setting since
    }
}

#[derive(Debug)]
pub struct LogsState {
    pub service_name: String,
    pub lines: Vec<LogLine>,
    pub offset: usize,
    pub follow_mode: bool,
    pub journal_reader: Option<JournalReader>,
    pub last_activity: Instant,
    pub is_live: bool,
    pub filters: LogFilters,
    pub needs_restart: bool, // Flag to restart journal reader with new filters
}

impl LogsState {
    pub fn new(service_name: String) -> Self {
        Self {
            service_name,
            lines: Vec::new(),
            offset: 0,
            follow_mode: true,
            journal_reader: None,
            last_activity: Instant::now(),
            is_live: false,
            filters: LogFilters::default(),
            needs_restart: false,
        }
    }

    pub fn add_line(&mut self, line: LogLine) {
        self.lines.push(line);
        self.last_activity = Instant::now();

        // Update live indicator based on recent activity
        self.is_live = self.last_activity.elapsed() < Duration::from_secs(5);

        // Auto-scroll in follow mode only if we're already near the bottom
        if self.follow_mode {
            let total_lines = self.lines.len();
            // If we're within 5 lines of the bottom, auto-scroll to follow new lines
            if self.offset >= total_lines.saturating_sub(6) {
                self.scroll_to_bottom();
            }
        }

        // Keep max 1000 lines in memory
        if self.lines.len() > 1000 {
            self.lines.remove(0);
            // Adjust scroll offset if needed - if we were scrolled past the removed line, adjust
            if self.offset > 0 {
                self.offset = self.offset.saturating_sub(1);
            }
        }

        // Ensure offset is still valid
        self.offset = self.offset.min(self.lines.len().saturating_sub(1));
    }

    pub fn handle_action(&mut self, action: LogsAction) -> Option<LogsAction> {
        match action {
            LogsAction::GoBack => Some(LogsAction::GoBack),
            LogsAction::ToggleFollow => {
                self.follow_mode = !self.follow_mode;
                if self.follow_mode {
                    self.scroll_to_bottom();
                }
                None
            },
            LogsAction::ScrollUp => {
                if self.offset > 0 {
                    self.offset -= 1;
                    // Disable follow mode when manually scrolling
                    self.follow_mode = false;
                }
                None
            },
            LogsAction::ScrollDown => {
                let max_offset = self.lines.len().saturating_sub(1);
                if self.offset < max_offset {
                    self.offset += 1;
                    // Check if we're at the bottom, if so re-enable follow mode
                    if self.offset >= max_offset {
                        self.follow_mode = true;
                    }
                }
                None
            },
            LogsAction::ScrollTop => {
                self.offset = 0;
                self.follow_mode = false;
                None
            },
            LogsAction::ScrollBottom => {
                self.scroll_to_bottom();
                self.follow_mode = true;
                None
            },
            LogsAction::PageUp => {
                self.offset = self.offset.saturating_sub(10); // Fixed page size for now
                self.follow_mode = false;
                None
            },
            LogsAction::PageDown => {
                let max_offset = self.lines.len().saturating_sub(10); // Fixed page size for now
                self.offset = (self.offset + 10).min(max_offset);
                if self.offset >= max_offset && !self.lines.is_empty() {
                    self.follow_mode = true;
                }
                None
            },
            LogsAction::TogglePriorityFilter => {
                self.filters.cycle_priority_filter();
                self.needs_restart = true;
                // Clear existing logs when changing filters
                self.lines.clear();
                self.offset = 0;
                None
            },
            LogsAction::ClearFilters => {
                if self.filters.is_active() {
                    self.filters.clear();
                    self.needs_restart = true;
                    // Clear existing logs when clearing filters
                    self.lines.clear();
                    self.offset = 0;
                }
                None
            },
            LogsAction::TimeFilterSince1h => {
                self.filters.set_time_filter(Some("1 hour ago".to_string()));
                self.needs_restart = true;
                self.lines.clear();
                self.offset = 0;
                None
            },
            LogsAction::TimeFilterSince24h => {
                self.filters.set_time_filter(Some("24 hours ago".to_string()));
                self.needs_restart = true;
                self.lines.clear();
                self.offset = 0;
                None
            },
            LogsAction::TimeFilterSince7d => {
                self.filters.set_time_filter(Some("7 days ago".to_string()));
                self.needs_restart = true;
                self.lines.clear();
                self.offset = 0;
                None
            }
        }
    }

    fn scroll_to_bottom(&mut self) {
        // For follow mode, we want the viewport to show the most recent lines
        // The render() method will clamp this appropriately based on viewport height
        if !self.lines.is_empty() {
            // Set offset to a large value - render() will clamp it to show the last lines
            self.offset = self.lines.len().saturating_sub(1);
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Log content
                Constraint::Length(3), // Footer/help
            ])
            .split(area);

        self.render_header(f, chunks[0]);
        self.render_logs(f, chunks[1]);
        self.render_footer(f, chunks[2]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let live_indicator = if self.is_live {
            "● LIVE"
        } else if self.follow_mode {
            "○ Following"
        } else {
            "○ Paused"
        };

        let live_style = if self.is_live {
            Style::default().fg(ratatui::style::Color::Green).add_modifier(Modifier::BOLD)
        } else if self.follow_mode {
            Style::default().fg(ratatui::style::Color::Yellow)
        } else {
            Style::default().fg(ratatui::style::Color::Gray)
        };

        let filter_indicator = if self.filters.is_active() {
            let priority_str = self.filters.min_priority
                .map(|p| format!("≥{}", priority_to_string(p)))
                .unwrap_or_else(|| "All".to_string());
            let time_str = self.filters.since.as_deref().unwrap_or("");
            format!(" [{} {}]", priority_str, time_str)
        } else {
            "".to_string()
        };

        let header_text = Line::from(vec![
            Span::raw("📜 "),
            Span::styled("Logs: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(&self.service_name, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(live_indicator, live_style),
            Span::styled(filter_indicator, Style::default().fg(ratatui::style::Color::Blue)),
            Span::raw(format!(" ({} lines)", self.lines.len())),
        ]);

        let header = Paragraph::new(header_text)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(header, area);
    }

    fn render_logs(&self, f: &mut Frame, area: Rect) {
        let height = area.height as usize;

        if self.lines.is_empty() {
            let empty_msg = Paragraph::new("No log lines yet...")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(empty_msg, area);
            return;
        }

        // Calculate visible range - show window starting from offset
        // Ensure offset is valid (don't show empty space at bottom if we can avoid it)
        let max_start = self.lines.len().saturating_sub(height);
        let start = self.offset.min(max_start);
        let end = (start + height).min(self.lines.len());

        let visible_lines: Vec<ListItem> = self.lines[start..end]
            .iter()
            .map(|line| {
                let is_new = line.is_live;
                let style = if is_new {
                    Style::default().fg(ratatui::style::Color::Green)
                } else {
                    Style::default()
                };

                let priority_style = line.priority
                    .and_then(priority_color)
                    .unwrap_or(style);

                // Format: [timestamp] message
                let content = if line.raw_line.len() > 100 {
                    // Truncate long lines
                    let truncated = format!("{}...", &line.raw_line[..97]);
                    Line::from(vec![
                        Span::styled(&line.timestamp, Style::default().fg(ratatui::style::Color::Blue)),
                        Span::raw(" "),
                        Span::styled(truncated, priority_style),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(&line.timestamp, Style::default().fg(ratatui::style::Color::Blue)),
                        Span::raw(" "),
                        Span::styled(&line.message, priority_style),
                    ])
                };

                ListItem::new(content)
            })
            .collect();

        let list = List::new(visible_lines)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(list, area);
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let filter_help = if self.filters.is_active() {
            " | p:Priority 1/2/7:Time c:Clear"
        } else {
            " | p:Priority 1/2/7:Time"
        };

        // Follow status with color
        let follow_status = if self.follow_mode { "ON" } else { "OFF" };
        let follow_style = if self.follow_mode {
            Style::default().fg(ratatui::style::Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ratatui::style::Color::Gray)
        };

        let help_line = Line::from(vec![
            Span::raw("↑/k:Up ↓/j:Down g:Top G:Bottom | Follow: "),
            Span::styled(follow_status, follow_style),
            Span::raw(" (t)"),
            Span::raw(filter_help),
            Span::raw(" | [Esc/←]:Back [q]:Quit"),
        ]);

        let footer = Paragraph::new(help_line)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(footer, area);
    }
}

impl From<Action> for LogsAction {
    fn from(action: Action) -> Self {
        match action {
            Action::GoBack => LogsAction::GoBack,
            Action::MoveUp => LogsAction::ScrollUp,
            Action::MoveDown => LogsAction::ScrollDown,
            Action::MoveTop => LogsAction::ScrollTop,
            Action::MoveBottom => LogsAction::ScrollBottom,
            Action::ViewLogs => LogsAction::ToggleFollow,
            Action::ToggleFollow => LogsAction::ToggleFollow,
            Action::ClearSearch => LogsAction::ClearFilters, // 'c' key clears filters
            Action::TogglePriorityFilter => LogsAction::TogglePriorityFilter,
            Action::TimeFilter1h => LogsAction::TimeFilterSince1h,
            Action::TimeFilter24h => LogsAction::TimeFilterSince24h,
            Action::TimeFilter7d => LogsAction::TimeFilterSince7d,
            _ => LogsAction::GoBack, // Default to going back for unhandled actions
        }
    }
}

fn priority_to_string(priority: u8) -> &'static str {
    match priority {
        0 => "emerg",
        1 => "alert",
        2 => "crit",
        3 => "err",
        4 => "warn",
        5 => "notice",
        6 => "info",
        7 => "debug",
        _ => "unknown",
    }
}