// Event handling for the TUI application

use crate::systemd::{JournalReader, LogLine, Service, ServiceDetail};
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use tokio::sync::mpsc;

/// Application events
#[derive(Debug)]
pub enum AppEvent {
    /// Services loaded from systemd
    ServicesLoaded(Vec<Service>),

    /// Service detail loaded
    ServiceDetailLoaded(Box<ServiceDetail>),

    /// Log line received
    LogLine(String),

    /// Parsed log line with metadata
    LogLineParsed(LogLine),

    /// Journal reader started (keep alive)
    JournalReaderStarted(JournalReader),

    /// Periodic tick for refresh
    Tick,

    /// User input event
    Input(CrosstermEvent),

    /// Error occurred
    Error(anyhow::Error),

    /// Request to quit
    Quit,

    /// Service operation completed
    ServiceOperationCompleted { service: String, operation: String, success: bool },

    /// Confirmation dialog request
    RequestConfirmation { service: String, operation: String, message: String },

    /// Status message for user feedback
    StatusMessage(String),

    /// User service created successfully
    ServiceCreated { name: String },

    /// User service creation failed
    ServiceCreationFailed { error: String },

    /// Show help
    ShowHelp,
}

/// User actions derived from input events
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,
    Select,
    GoBack,
    ToggleFilter(FilterAction),
    ToggleScope,
    Search(String),
    ClearSearch,
    ViewLogs,
    ToggleFollow,
    TogglePriorityFilter,
    TimeFilter1h,
    TimeFilter24h,
    TimeFilter7d,
    Refresh,
    ConfirmAction,
    CancelAction,

    ShowHelp,
    // Service control actions
    StartService,
    StopService,
    RestartService,
    EnableService,
    DisableService,
    ReloadService,

    // Service creation
    CreateService,
    NewService,
    SubmitNewService,
    Back,

    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterAction {
    All,
    Running,
    Stopped,
    Failed,
}

/// Convert keyboard input to actions
pub fn key_event_to_action(key: KeyEvent) -> Action {
    match (key.code, key.modifiers) {
        // Quit
        (KeyCode::Char('q'), KeyModifiers::NONE) => Action::Quit,
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Action::Quit,

        // Navigation
        (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => Action::MoveUp,
        (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => Action::MoveDown,
        (KeyCode::Char('g'), KeyModifiers::NONE) => Action::MoveTop,
        (KeyCode::Char('G'), KeyModifiers::SHIFT) => Action::MoveBottom,

        // Selection
        (KeyCode::Enter, _) => Action::Select,
        (KeyCode::Esc, _) | (KeyCode::Left, _) => Action::GoBack,

        // Filtering
        (KeyCode::Char('a'), KeyModifiers::NONE) => Action::ToggleFilter(FilterAction::All),
        (KeyCode::Char('r'), KeyModifiers::NONE) => Action::ToggleFilter(FilterAction::Running),
        (KeyCode::Char('s'), KeyModifiers::NONE) => Action::ToggleFilter(FilterAction::Stopped),
        (KeyCode::Char('f'), KeyModifiers::NONE) => Action::ToggleFilter(FilterAction::Failed),
        (KeyCode::Char('m'), KeyModifiers::NONE) => Action::ToggleScope,

        // Service control
        (KeyCode::Char('S'), KeyModifiers::SHIFT) => Action::StartService,
        (KeyCode::Char('T'), KeyModifiers::SHIFT) => Action::StopService,
        (KeyCode::Char('R'), KeyModifiers::SHIFT) => Action::RestartService,
        (KeyCode::Char('E'), KeyModifiers::SHIFT) => Action::EnableService,
        (KeyCode::Char('D'), KeyModifiers::SHIFT) => Action::DisableService,
        (KeyCode::Char('L'), KeyModifiers::SHIFT) => Action::ReloadService,

        // Confirmation
        (KeyCode::Char('y'), KeyModifiers::NONE) => Action::ConfirmAction,
        (KeyCode::Char('n'), KeyModifiers::NONE) => Action::CancelAction,

        // Other actions
        (KeyCode::Char('l'), KeyModifiers::NONE) => Action::ViewLogs,
        (KeyCode::Char('t'), KeyModifiers::NONE) => Action::ToggleFollow,
        (KeyCode::Char('c'), KeyModifiers::NONE) => Action::ClearSearch,
        (KeyCode::Char('N'), KeyModifiers::SHIFT) => Action::NewService,
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => Action::SubmitNewService,
        (KeyCode::F(5), _) => Action::Refresh,
        (KeyCode::Char('/'), KeyModifiers::NONE) => Action::Search(String::new()),

        // Log filtering actions
        (KeyCode::Char('p'), KeyModifiers::NONE) => Action::TogglePriorityFilter,
        (KeyCode::Char('1'), KeyModifiers::NONE) => Action::TimeFilter1h,
        (KeyCode::Char('2'), KeyModifiers::NONE) => Action::TimeFilter24h,
        (KeyCode::Char('7'), KeyModifiers::NONE) => Action::TimeFilter7d,

        (KeyCode::Char('?'), KeyModifiers::NONE) => Action::ShowHelp,

        _ => Action::None,
    }
}

/// Spawn input event handler task
pub async fn spawn_input_handler(tx: mpsc::Sender<AppEvent>) {
    tokio::spawn(async move {
        loop {
            if crossterm::event::poll(Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(event) = crossterm::event::read() {
                    if tx.send(AppEvent::Input(event)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
}

/// Spawn periodic tick task
pub async fn spawn_ticker(tx: mpsc::Sender<AppEvent>, interval: Duration) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(interval);
        loop {
            interval.tick().await;
            if tx.send(AppEvent::Tick).await.is_err() {
                break;
            }
        }
    });
}
