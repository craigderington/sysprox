// Systemd integration module

pub mod client;
pub mod control;
pub mod journal;
pub mod metrics;
pub mod models;
pub mod resilience;

#[cfg(test)]
mod tests;

pub use client::SystemdClient;
pub use control::ServiceController;
pub use journal::{JournalReader, LogLine};
pub use metrics::{MetricsCollector, MetricsSnapshot, ServiceMetricsCollection, SystemMetrics};
pub use models::{Metrics, Service, ServiceDetail, ServiceScope, ServiceStatus, ServiceStatusExtended};
pub use resilience::{ConnectionManager, RecoveryStrategy, SystemdHealthChecker};

// Re-export for tests
#[cfg(test)]
pub use metrics::calculate_cpu_percent;
