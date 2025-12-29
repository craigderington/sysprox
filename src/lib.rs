// Sysprox - Systemd Service Monitor TUI
// Library root

pub mod app;
pub mod config;
pub mod error;
pub mod events;
pub mod systemd;
pub mod ui;
pub mod version;

// Test modules (only compiled during tests)
#[cfg(test)]
mod app_tests;
#[cfg(test)]
mod config_tests;
