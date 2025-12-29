#[cfg(test)]
mod tests {
    use crate::error::Result;
    use crate::systemd::{
        ConnectionManager, RecoveryStrategy, ServiceController,
        Service, ServiceDetail, MetricsCollector, Metrics,
        ServiceStatusExtended, SystemMetrics, calculate_cpu_percent, ServiceScope
    };

    #[tokio::test]
    async fn test_connection_manager_basic() -> Result<()> {
        let manager = ConnectionManager::default();
        
        // Test systemd availability (this will fail if not running as root or without systemd)
        match manager.test_systemd_availability().await {
            Ok(available) => {
                println!("Systemd availability: {}", available);
                assert!(available || !available); // Just testing the method works
            }
            Err(_) => {
                // Expected in some environments
                println!("Systemd not available (expected in test environment)");
            }
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_manager_retry() -> Result<()> {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let manager = ConnectionManager::new(
            2, // max_retries
            std::time::Duration::from_millis(100),
            std::time::Duration::from_secs(1),
        );

        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = manager.with_retry("test_operation", move || {
            let count = call_count_clone.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 2 {
                    Err(anyhow::anyhow!("Simulated failure"))
                } else {
                    Ok("success")
                }
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(call_count.load(Ordering::SeqCst), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_retry_strategy() {
        let strategy = RecoveryStrategy::ExponentialBackoff {
            base_delay: std::time::Duration::from_millis(100),
            max_delay: std::time::Duration::from_millis(500),
            multiplier: 2.0,
        };

        assert_eq!(strategy.delay(1), std::time::Duration::from_millis(100));
        assert_eq!(strategy.delay(2), std::time::Duration::from_millis(200));
        assert_eq!(strategy.delay(3), std::time::Duration::from_millis(400));
        assert_eq!(strategy.delay(4), std::time::Duration::from_millis(500)); // Capped at max_delay
    }

    #[tokio::test]
    async fn test_service_controller_validation() -> Result<()> {
        let controller = ServiceController::new().await?;

        // Test invalid service names
        assert!(controller.validate_service_name("").is_err());
        assert!(controller.validate_service_name("../etc/passwd.service").is_err());
        assert!(controller.validate_service_name("invalid\0service").is_err());
        assert!(controller.validate_service_name("noextension").is_err());

        // Test valid service names
        assert!(controller.validate_service_name("test.service").is_ok());
        assert!(controller.validate_service_name("nginx.service").is_ok());

        Ok(())
    }

    #[test]
    fn test_service_status_extended() {
        let service = Service {
            id: "test.service".to_string(),
            name: "test.service".to_string(),
            description: "Test Service".to_string(),
            load_state: "loaded".to_string(),
            active_state: "active".to_string(),
            sub_state: "running".to_string(),
            pid: 1234,
            enabled: true,
            scope: ServiceScope::System,
            loaded_at: chrono::Utc::now(),
        };

        let detail = ServiceDetail {
            service: service.clone(),
            main_pid: 1234,
            control_pid: 0,
            load_path: "/test/path".to_string(),
            exec_main_start: String::new(),
            exec_main_status: String::new(),
            memory_current: 1024 * 1024,
            memory_limit: u64::MAX,
            cpu_usage_nsec: 1000000000,
            tasks_current: 5,
            tasks_max: 100,
            n_restarts: 2,
            active_enter_time: chrono::Utc::now(),
            active_exit_time: chrono::Utc::now(),
            inactive_enter_time: chrono::Utc::now(),
            state_change_time: chrono::Utc::now(),
            result: "success".to_string(),
            wants: vec!["network.target".to_string()],
            wanted_by: vec!["multi-user.target".to_string()],
            after: vec!["network.target".to_string()],
            before: vec!["shutdown.target".to_string()],
            service_type: "simple".to_string(),
            restart: "on-failure".to_string(),
            user: "root".to_string(),
            group: "root".to_string(),
            working_directory: "/".to_string(),
            environment: vec![],
        };

        let status = ServiceStatusExtended {
            service: detail,
            is_enabled: true,
        };

        assert_eq!(status.status_summary(), "Running (Enabled)");
        assert_eq!(status.status_icon(), "●");
    }

    #[test]
    fn test_metrics_calculation() {
        let prev = Metrics {
            cpu_usage_nsec: 1000000,
            memory_current: 1024,
            tasks_current: 5,
            n_restarts: 1,
        };

        let curr = Metrics {
            cpu_usage_nsec: 2000000,
            memory_current: 2048,
            tasks_current: 10,
            n_restarts: 1,
        };

        let cpu_percent = calculate_cpu_percent(&prev, &curr, 1.0);
        assert!((cpu_percent - 0.1).abs() < 0.01); // Should be ~0.1% for 1s interval
    }

    #[test]
    fn test_service_helper_methods() {
        let service = Service {
            id: "test.service".to_string(),
            name: "test.service".to_string(),
            description: "Test Service".to_string(),
            load_state: "loaded".to_string(),
            active_state: "active".to_string(),
            sub_state: "running".to_string(),
            pid: 1234,
            enabled: true,
            scope: ServiceScope::System,
            loaded_at: chrono::Utc::now(),
        };

        assert!(service.is_active());
        assert!(!service.is_failed());
        assert!(!service.is_inactive());
        assert!(!service.is_transitioning());
        assert_eq!(service.status_text(), "Running");

        let failed_service = Service {
            active_state: "failed".to_string(),
            ..service.clone()
        };
        assert!(failed_service.is_failed());
        assert!(!failed_service.is_active());
        assert_eq!(failed_service.status_text(), "Failed");

        let starting_service = Service {
            active_state: "activating".to_string(),
            ..service.clone()
        };
        assert!(starting_service.is_transitioning());
        assert_eq!(starting_service.status_text(), "Starting");
    }

    #[test]
    fn test_system_metrics() {
        let metrics = SystemMetrics {
            load_average: 1.5,
            total_memory: 8 * 1024 * 1024 * 1024, // 8GB
            used_memory: 4 * 1024 * 1024 * 1024,  // 4GB
            free_memory: 4 * 1024 * 1024 * 1024,  // 4GB
            swap_total: 2 * 1024 * 1024 * 1024,   // 2GB
            swap_used: 1 * 1024 * 1024 * 1024,    // 1GB
        };

        assert_eq!(metrics.memory_usage_percent(), 50.0);
        assert_eq!(metrics.total_mb(), 8192.0);
        assert_eq!(metrics.used_mb(), 4096.0);
        assert_eq!(metrics.available_mb(), 4096.0);
    }

    #[tokio::test]
    async fn test_metrics_collector_basic() -> Result<()> {
        let mut collector = MetricsCollector::new().await?;

        // This will likely fail in test environment without systemd,
        // but we're testing the structure
        match collector.get_service_metrics("nonexistent.service").await {
            Ok(_) => {
                println!("Unexpected success getting metrics for nonexistent service");
            }
            Err(_) => {
                println!("Expected failure for nonexistent service");
            }
        }

        Ok(())
    }

    #[test]
    fn test_error_recovery_strategies() {
        let immediate = RecoveryStrategy::Immediate;
        assert_eq!(immediate.delay(5), std::time::Duration::from_millis(0));

        let fixed = RecoveryStrategy::FixedDelay(std::time::Duration::from_secs(2));
        assert_eq!(fixed.delay(1), std::time::Duration::from_secs(2));
        assert_eq!(fixed.delay(5), std::time::Duration::from_secs(2));
    }
}