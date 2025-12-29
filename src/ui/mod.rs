// UI module - TUI components

pub mod dashboard;
pub mod detail;
pub mod logs;
pub mod help;
pub mod new_service;
pub mod styles;

#[cfg(test)]
mod tests;

pub use dashboard::{DashboardState, FilterType};
pub use detail::{DetailAction, DetailState};
pub use logs::{LogsAction, LogsState};
pub use help::HelpState;
pub use new_service::NewServiceForm;
pub use styles::*;
