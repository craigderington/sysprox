// Metrics collection and calculation

use crate::error::{Result, SysproxError};
use crate::systemd::Metrics;
use zbus::Connection;

/// Service metrics collector with history tracking
pub struct MetricsCollector {
    connection: Connection,
    history: std::collections::HashMap<String, Vec<MetricsSnapshot>>,
    max_history: usize,
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metrics: Metrics,
    pub cpu_percent: f64,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub async fn new() -> Result<Self> {
        let connection = Connection::system()
            .await
            .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        Ok(Self {
            connection,
            history: std::collections::HashMap::new(),
            max_history: 300, // Keep 5 minutes of data (assuming 1s intervals)
        })
    }

    /// Get current metrics for a service
    pub async fn get_service_metrics(&mut self, service_name: &str) -> Result<MetricsSnapshot> {
        let service_path = self.get_service_path(service_name).await?;
        let props = self.get_service_properties(&service_path).await?;

        let current_metrics = Metrics {
            cpu_usage_nsec: props.get("CPUUsageNSec")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .unwrap_or(0),
            memory_current: props.get("MemoryCurrent")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .unwrap_or(0),
            tasks_current: props.get("TasksCurrent")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .unwrap_or(0),
            n_restarts: props.get("NRestarts")
                .and_then(|v| v.downcast_ref::<u32>().ok())
                .unwrap_or(0),
        };

        let timestamp = chrono::Utc::now();
        let cpu_percent = self.calculate_cpu_percentage(service_name, &current_metrics);

        let snapshot = MetricsSnapshot {
            timestamp,
            metrics: current_metrics,
            cpu_percent,
        };

        // Store in history
        self.update_history(service_name.to_string(), snapshot.clone());

        Ok(snapshot)
    }

    /// Get historical metrics for a service
    pub fn get_metrics_history(&self, service_name: &str) -> &[MetricsSnapshot] {
        self.history.get(service_name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get CPU percentage for recent interval
    pub fn get_current_cpu_percent(&self, service_name: &str) -> f64 {
        self.history.get(service_name)
            .and_then(|snapshots| snapshots.iter().last())
            .map(|snapshot| snapshot.cpu_percent)
            .unwrap_or(0.0)
    }

    /// Get memory usage in MB
    pub fn get_memory_mb(&self, service_name: &str) -> f64 {
        self.history.get(service_name)
            .and_then(|snapshots| snapshots.iter().last())
            .map(|snapshot| snapshot.metrics.memory_current as f64 / 1024.0 / 1024.0)
            .unwrap_or(0.0)
    }

    /// Get system-wide metrics for context
    pub async fn get_system_metrics(&mut self) -> Result<SystemMetrics> {
        // Get CPU load from /proc/loadavg
        let load_avg = std::fs::read_to_string("/proc/loadavg")
            .map(|s| {
                s.split_whitespace()
                    .next()
                    .unwrap_or("0.0")
                    .parse::<f64>()
                    .unwrap_or(0.0)
            })
            .unwrap_or(0.0);

        // Get memory info from /proc/meminfo
        let memory_info = self.get_memory_info().await?;

        Ok(SystemMetrics {
            load_average: load_avg,
            total_memory: memory_info.total,
            used_memory: memory_info.used,
            free_memory: memory_info.free,
            swap_total: memory_info.swap_total,
            swap_used: memory_info.swap_used,
        })
    }

    /// Clear history for a service
    pub fn clear_history(&mut self, service_name: &str) {
        self.history.remove(service_name);
    }

    /// Get service object path from systemd
    async fn get_service_path(&self, service_name: &str) -> Result<zbus::zvariant::OwnedObjectPath> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        let path: zbus::zvariant::OwnedObjectPath = proxy
            .call("GetUnit", &(service_name,))
            .await
            .map_err(|e| SysproxError::ServiceInfo(format!("Failed to get unit path: {}", e)))?;

        Ok(path)
    }

    /// Get service properties from D-Bus
    async fn get_service_properties(
        &self,
        service_path: &zbus::zvariant::OwnedObjectPath,
    ) -> Result<std::collections::HashMap<String, zbus::zvariant::OwnedValue>> {
        let props_proxy = zbus::fdo::PropertiesProxy::builder(&self.connection)
            .destination("org.freedesktop.systemd1")?
            .path(service_path.as_str())?
            .build()
            .await
            .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        use zbus::zvariant::Optional;
        let props = props_proxy
            .get_all(Optional::default())
            .await
            .map_err(|e| SysproxError::ServiceInfo(format!("Failed to get properties: {}", e)))?;

        Ok(props)
    }

    /// Calculate CPU percentage based on historical data
    fn calculate_cpu_percentage(&mut self, service_name: &str, current: &Metrics) -> f64 {
        if let Some(snapshots) = self.history.get(service_name) {
            if let Some(prev_snapshot) = snapshots.iter().last() {
                let prev = &prev_snapshot.metrics;
                if current.cpu_usage_nsec > prev.cpu_usage_nsec {
                    let cpu_delta_ns = current.cpu_usage_nsec - prev.cpu_usage_nsec;
                    let time_delta_secs = (chrono::Utc::now() - prev_snapshot.timestamp).num_seconds() as f64;
                    
                    if time_delta_secs > 0.0 {
                        let cpu_delta_secs = cpu_delta_ns as f64 / 1_000_000_000.0;
                        return (cpu_delta_secs / time_delta_secs) * 100.0;
                    }
                }
            }
        }
        0.0
    }

    /// Update history with new snapshot
    fn update_history(&mut self, service_name: String, snapshot: MetricsSnapshot) {
        let history = self.history.entry(service_name).or_default();
        history.push(snapshot);

        // Keep only the most recent entries
        if history.len() > self.max_history {
            history.drain(0..history.len() - self.max_history);
        }
    }

    /// Read memory information from /proc/meminfo
    async fn get_memory_info(&self) -> Result<MemoryInfo> {
        let content = std::fs::read_to_string("/proc/meminfo")
            .map_err(|e| SysproxError::ServiceInfo(format!("Failed to read /proc/meminfo: {}", e)))?;

        let mut info = MemoryInfo::default();

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let value = parts[1].parse::<u64>().unwrap_or(0);
                match parts[0] {
                    "MemTotal:" => info.total = value * 1024, // Convert KB to bytes
                    "MemFree:" => info.free = value * 1024,
                    "MemAvailable:" => info.available = value * 1024,
                    "SwapTotal:" => info.swap_total = value * 1024,
                    "SwapFree:" => {
                        info.swap_used = info.swap_total.saturating_sub(value * 1024);
                    }
                    _ => {}
                }
            }
        }

        info.used = info.total.saturating_sub(info.free);

        Ok(info)
    }
}

/// System-wide metrics
#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    pub load_average: f64,
    pub total_memory: u64,
    pub used_memory: u64,
    pub free_memory: u64,
    pub swap_total: u64,
    pub swap_used: u64,
}

impl SystemMetrics {
    /// Memory usage percentage
    pub fn memory_usage_percent(&self) -> f64 {
        if self.total_memory > 0 {
            (self.used_memory as f64 / self.total_memory as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Memory available in MB
    pub fn available_mb(&self) -> f64 {
        self.free_memory as f64 / 1024.0 / 1024.0
    }

    /// Total memory in MB
    pub fn total_mb(&self) -> f64 {
        self.total_memory as f64 / 1024.0 / 1024.0
    }

    /// Used memory in MB
    pub fn used_mb(&self) -> f64 {
        self.used_memory as f64 / 1024.0 / 1024.0
    }
}

/// Memory information from /proc/meminfo
#[derive(Debug, Clone, Default)]
struct MemoryInfo {
    total: u64,
    used: u64,
    free: u64,
    available: u64,
    swap_total: u64,
    swap_used: u64,
}

/// Calculate CPU percentage from two metric snapshots (legacy function)
pub fn calculate_cpu_percent(prev: &Metrics, curr: &Metrics, interval_secs: f64) -> f64 {
    if interval_secs <= 0.0 || curr.cpu_usage_nsec <= prev.cpu_usage_nsec {
        return 0.0;
    }

    let cpu_delta_ns = curr.cpu_usage_nsec - prev.cpu_usage_nsec;
    let cpu_delta_secs = cpu_delta_ns as f64 / 1_000_000_000.0;

    // Percentage based on interval
    (cpu_delta_secs / interval_secs) * 100.0
}

/// Get service metrics from systemd (legacy function - use MetricsCollector instead)
pub async fn get_service_metrics(_service_name: &str) -> Result<crate::systemd::models::Metrics> {
    // This is now implemented in MetricsCollector::get_service_metrics
    Err(SysproxError::ServiceInfo(
        "Use MetricsCollector::get_service_metrics instead".to_string(),
    ).into())
}

/// Trait for service metrics collection
pub trait ServiceMetricsCollection {
    fn collect_metrics(&mut self, service_name: &str) -> impl std::future::Future<Output = Result<MetricsSnapshot>> + Send;
    fn get_history(&self, service_name: &str) -> &[MetricsSnapshot];
}

// Implement the trait for MetricsCollector
impl ServiceMetricsCollection for MetricsCollector {
    fn collect_metrics(&mut self, service_name: &str) -> impl std::future::Future<Output = Result<MetricsSnapshot>> + Send {
        self.get_service_metrics(service_name)
    }

    fn get_history(&self, service_name: &str) -> &[MetricsSnapshot] {
        self.get_metrics_history(service_name)
    }
}

// Make MetricsCollector cloneable for async tasks
impl Clone for MetricsCollector {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            history: std::collections::HashMap::new(),
            max_history: self.max_history,
        }
    }
}
