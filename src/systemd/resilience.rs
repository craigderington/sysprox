// Resilient D-Bus connection handling with retry logic

use crate::error::{Result, SysproxError};
use std::time::Duration;
use tokio::time::sleep;
use zbus::Connection;

/// Connection manager with automatic retry and reconnection
#[derive(Debug, Clone)]
pub struct ConnectionManager {
    max_retries: usize,
    retry_delay: Duration,
    connection_timeout: Duration,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay: Duration::from_millis(500),
            connection_timeout: Duration::from_secs(5),
        }
    }
}

impl ConnectionManager {
    /// Create a new connection manager with custom settings
    pub fn new(max_retries: usize, retry_delay: Duration, connection_timeout: Duration) -> Self {
        Self {
            max_retries,
            retry_delay,
            connection_timeout,
        }
    }

    /// Establish a systemd connection with retry logic
    pub async fn connect_systemd(&self) -> Result<Connection> {
        self.with_retry("systemd connection", || async {
            let conn = tokio::time::timeout(
                self.connection_timeout,
                Connection::system()
            ).await
            .map_err(|_| {
                SysproxError::SystemdConnection("Connection timeout".to_string())
            })?
            .map_err(|e| {
                SysproxError::SystemdConnection(format!("Failed to connect: {}", e))
            })?;
            
            Ok(conn)
        }).await
    }

    /// Execute an operation with automatic retry
    pub async fn with_retry<F, T, Fut>(&self, operation_name: &str, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 1..=self.max_retries {
            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        tracing::info!("Operation '{}' succeeded on attempt {}", operation_name, attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    tracing::warn!("Operation '{}' failed on attempt {}: {}", operation_name, attempt, error);
                    last_error = Some(error);

                    // Don't retry on certain error types
                    if self.should_not_retry(last_error.as_ref().unwrap()) {
                        break;
                    }

                    // Wait before retry (except on last attempt)
                    if attempt < self.max_retries {
                        tracing::debug!("Retrying in {:?}...", self.retry_delay);
                        sleep(self.retry_delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            SysproxError::SystemdConnection("No error recorded during retry".to_string()).into()
        }))
    }

    /// Check if an error should not be retried
    fn should_not_retry(&self, error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();
        
        // Don't retry on permission errors
        if error_str.contains("permission denied") || error_str.contains("access denied") {
            return true;
        }

        // Don't retry on not found errors
        if error_str.contains("not found") || error_str.contains("no such file") {
            return true;
        }

        // Don't retry on authentication errors
        if error_str.contains("authentication") || error_str.contains("auth") {
            return true;
        }

        // Don't retry on invalid arguments
        if error_str.contains("invalid argument") || error_str.contains("invalid name") {
            return true;
        }

        false
    }

    /// Test if systemd is available and responsive
    pub async fn test_systemd_availability(&self) -> Result<bool> {
        match self.connect_systemd().await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::debug!("Systemd availability test failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Wait for systemd to become available
    pub async fn wait_for_systemd(&self, max_wait: Duration) -> Result<()> {
        let start_time = std::time::Instant::now();
        let mut consecutive_failures = 0;
        let max_consecutive_failures = 3;

        while start_time.elapsed() < max_wait {
            match self.test_systemd_availability().await {
                Ok(true) => {
                    if consecutive_failures > 0 {
                        tracing::info!("Systemd is now available after {} failures", consecutive_failures);
                    }
                    return Ok(());
                }
                Ok(false) => {
                    consecutive_failures += 1;
                    if consecutive_failures >= max_consecutive_failures {
                        return Err(SysproxError::SystemdConnection(
                            format!("Systemd not available after {} consecutive attempts", consecutive_failures)
                        ).into());
                    }
                }
                Err(e) => {
                    tracing::warn!("Error testing systemd availability: {}", e);
                }
            }

            sleep(Duration::from_secs(1)).await;
        }

        Err(SysproxError::SystemdConnection(
            format!("Systemd not available after {:?}", max_wait)
        ).into())
    }
}

/// Health checker for systemd connection
pub struct SystemdHealthChecker {
    connection: Connection,
    check_interval: Duration,
}

impl SystemdHealthChecker {
    /// Create a new health checker
    pub fn new(connection: Connection, check_interval: Duration) -> Self {
        Self {
            connection,
            check_interval,
        }
    }

    /// Check if the connection is still healthy
    pub async fn is_healthy(&self) -> bool {
        // Try a simple D-Bus call to test connection
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await;

        match proxy {
            Ok(proxy) => {
                // Try a simple call
                proxy.call::<_, _, ()>("ListJobs", &()).await.is_ok()
            }
            Err(_) => false,
        }
    }

    /// Start background health monitoring
    pub async fn start_monitoring(&self, mut shutdown_rx: tokio::sync::oneshot::Receiver<()>) -> Result<()> {
        let connection = self.connection.clone();
        let interval = self.check_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let proxy = zbus::Proxy::new(
                            &connection,
                            "org.freedesktop.systemd1",
                            "/org/freedesktop/systemd1",
                            "org.freedesktop.systemd1.Manager",
                        ).await;

                        if let Ok(proxy) = proxy {
                            if proxy.call::<_, _, ()>("ListJobs", &()).await.is_err() {
                                tracing::warn!("Systemd connection health check failed");
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        tracing::info!("Health monitoring shutdown");
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}

/// Error recovery strategies
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// Retry immediately
    Immediate,
    /// Exponential backoff
    ExponentialBackoff { base_delay: Duration, max_delay: Duration, multiplier: f64 },
    /// Fixed delay
    FixedDelay(Duration),
}

/// Default recovery strategy with exponential backoff
impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self::ExponentialBackoff {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
        }
    }
}

impl RecoveryStrategy {
    /// Get delay for a specific attempt
    pub fn delay(&self, attempt: usize) -> Duration {
        match self {
            RecoveryStrategy::Immediate => Duration::from_millis(0),
            RecoveryStrategy::FixedDelay(duration) => *duration,
            RecoveryStrategy::ExponentialBackoff { base_delay, max_delay, multiplier } => {
                let delay_ms = base_delay.as_millis() as f64 * multiplier.powi(attempt as i32 - 1);
                let delay = Duration::from_millis(delay_ms as u64);
                delay.min(*max_delay)
            }
        }
    }
}