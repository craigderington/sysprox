// Error types for sysprox

use thiserror::Error;

/// Result type alias using anyhow::Error
pub type Result<T> = anyhow::Result<T>;

/// Sysprox-specific error types
#[derive(Error, Debug)]
pub enum SysproxError {
    #[error("Failed to connect to systemd D-Bus: {0}")]
    SystemdConnection(String),

    #[error("Failed to fetch service information: {0}")]
    ServiceInfo(String),

    #[error("Failed to control service '{service}': {message}")]
    ServiceControl { service: String, message: String },

    #[error("Failed to read journal logs: {0}")]
    Journal(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Terminal error: {0}")]
    Terminal(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
