// Systemd service data models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// ServiceScope represents whether a service is system-level or user-level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceScope {
    /// System-level service (requires root/sudo)
    System,
    /// User-level service (runs in user session)
    User,
}

impl ServiceScope {
    /// Get display label for the scope
    pub fn label(&self) -> &'static str {
        match self {
            ServiceScope::System => "system",
            ServiceScope::User => "user",
        }
    }

    /// Get systemctl flag for this scope
    pub fn systemctl_flag(&self) -> Option<&'static str> {
        match self {
            ServiceScope::System => None,
            ServiceScope::User => Some("--user"),
        }
    }
}

/// Service represents a systemd service unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub pid: u32,
    pub enabled: bool,
    pub scope: ServiceScope,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub loaded_at: DateTime<Utc>,
}

impl Service {
    /// Returns true if the service is currently active/running
    pub fn is_active(&self) -> bool {
        self.active_state == "active"
    }

    /// Returns true if the service failed
    pub fn is_failed(&self) -> bool {
        self.active_state == "failed"
    }

    /// Returns true if the service is inactive/stopped
    pub fn is_inactive(&self) -> bool {
        self.active_state == "inactive"
    }

    /// Returns true if the service is in a transitioning state
    pub fn is_transitioning(&self) -> bool {
        self.active_state == "activating" || self.active_state == "deactivating"
    }

    /// Returns the service's current state as a user-friendly string
    pub fn status_text(&self) -> &'static str {
        match self.active_state.as_str() {
            "active" => "Running",
            "inactive" => "Stopped",
            "failed" => "Failed",
            "activating" => "Starting",
            "deactivating" => "Stopping",
            _ => "Unknown",
        }
    }
}

/// Extended service status with enable/disable information
#[derive(Debug, Clone)]
pub struct ServiceStatusExtended {
    pub service: ServiceDetail,
    pub is_enabled: bool,
}

impl ServiceStatusExtended {
    /// Returns a summary status string
    pub fn status_summary(&self) -> String {
        let status = self.service.service.status_text();
        let enabled_status = if self.is_enabled { "Enabled" } else { "Disabled" };
        format!("{} ({})", status, enabled_status)
    }

    /// Returns status icon for display
    pub fn status_icon(&self) -> &'static str {
        if !self.is_enabled {
            "○" // Circle for disabled
        } else {
            match self.service.service.active_state.as_str() {
                "active" => "●",     // Filled circle for active
                "failed" => "✗",     // X for failed
                "inactive" => "○",   // Empty circle for inactive
                "activating" => "◐", // Half-filled for starting
                "deactivating" => "◐", // Half-filled for stopping
                _ => "?",
            }
        }
    }
}

/// ServiceStatus represents the current status with runtime metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub service: Service,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub tasks_count: u32,
    pub restart_count: u32,
    #[serde(with = "serde_duration")]
    pub uptime: Duration,
}

/// ServiceDetail represents detailed information about a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDetail {
    pub service: Service,
    pub main_pid: u32,
    pub control_pid: u32,
    pub load_path: String,
    pub exec_main_start: String,
    pub exec_main_status: String,
    pub memory_current: u64,
    pub memory_limit: u64,
    pub cpu_usage_nsec: u64,
    pub tasks_current: u64,
    pub tasks_max: u64,
    pub n_restarts: u32,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub active_enter_time: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub active_exit_time: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub inactive_enter_time: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub state_change_time: DateTime<Utc>,
    pub result: String,
    pub wants: Vec<String>,
    pub wanted_by: Vec<String>,
    pub after: Vec<String>,
    pub before: Vec<String>,

    // Service configuration details
    pub service_type: String,
    pub restart: String,
    pub user: String,
    pub group: String,
    pub working_directory: String,
    pub environment: Vec<String>,
}

impl ServiceDetail {
    /// Calculate uptime based on active_enter_time
    pub fn uptime(&self) -> Option<Duration> {
        if self.service.is_active() {
            // Check if active_enter_time is epoch (0) - means timestamp wasn't set
            let epoch = chrono::DateTime::from_timestamp(0, 0).unwrap();
            if self.active_enter_time == epoch {
                tracing::debug!("Service {} has epoch timestamp, returning None for uptime", self.service.name);
                return None;
            }

            let now = Utc::now();
            let duration = now.signed_duration_since(self.active_enter_time);
            let uptime = duration.to_std().ok();
            tracing::debug!("Service {} uptime: {:?} (active_enter_time: {}, now: {})",
                self.service.name, uptime, self.active_enter_time, now);
            uptime
        } else {
            tracing::debug!("Service {} is not active, returning None for uptime", self.service.name);
            None
        }
    }

    /// Format memory usage as human-readable string
    pub fn memory_usage_formatted(&self) -> String {
        format_bytes(self.memory_current)
    }

    /// Calculate memory usage percentage if limit is set
    pub fn memory_usage_percent(&self) -> Option<f64> {
        if self.memory_limit > 0 && self.memory_limit != u64::MAX {
            Some((self.memory_current as f64 / self.memory_limit as f64) * 100.0)
        } else {
            None
        }
    }
}

/// Metrics snapshot for a service
#[derive(Debug, Clone, Copy, Default)]
pub struct Metrics {
    pub cpu_usage_nsec: u64,
    pub memory_current: u64,
    pub tasks_current: u64,
    pub n_restarts: u32,
}



// Helper module for Duration serialization
mod serde_duration {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Format bytes as human-readable string (e.g., "45.2 MB")
fn format_bytes(bytes: u64) -> String {
    use byte_unit::{Byte, UnitType};

    let byte = Byte::from_u64(bytes);
    byte.get_appropriate_unit(UnitType::Binary).to_string()
}
