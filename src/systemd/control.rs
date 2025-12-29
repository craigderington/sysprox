// Systemd service control operations

use crate::error::{Result, SysproxError};
use zbus::Connection;

/// Service controller for systemd operations
pub struct ServiceController {
    connection: Connection,
}

impl ServiceController {
    /// Create a new service controller with D-Bus connection
    pub async fn new() -> Result<Self> {
        // Connect to system bus - polkit will handle authentication
        let connection = Connection::system()
            .await
            .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        Ok(Self { connection })
    }

    /// Check if we can perform privileged operations
    /// Returns Ok if polkit agent is running OR we're running as root
    fn check_polkit_available() -> Result<()> {
        // If running as root, no need for polkit
        // Check effective user ID without libc dependency
        let is_root = std::env::var("USER").unwrap_or_default() == "root" ||
                      std::env::var("EUID").unwrap_or_default() == "0";

        if is_root {
            return Ok(());
        }

        // Check if a polkit authentication agent is running
        // Common agents: gnome-polkit, lxpolkit, mate-polkit, polkit-kde-agent
        let output = std::process::Command::new("pgrep")
            .arg("-f")
            .arg("polkit.*agent")
            .output();

        if let Ok(output) = output {
            if !output.stdout.is_empty() {
                // Polkit agent is running
                return Ok(());
            }
        }

        // No polkit agent and not root - authentication will be required
        Err(SysproxError::ServiceControl {
            service: "system".to_string(),
            message: "Authentication required. Start a polkit agent (e.g., lxpolkit, polkit-kde-agent), run sysprox with sudo, or run as root.".to_string(),
        }.into())
    }

    /// Start a service unit
    pub async fn start_service(&self, service_name: &str) -> Result<()> {
        self.validate_service_name(service_name)?;
        Self::check_polkit_available()?;

        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        // StartUnit returns the job object path
        // This will trigger polkit authentication dialog if privileges are needed
        let _job_path: zbus::zvariant::OwnedObjectPath = proxy
            .call("StartUnit", &(service_name, "replace"))
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("Access denied") || error_msg.contains("Authentication") {
                    SysproxError::ServiceControl {
                        service: service_name.to_string(),
                        message: "Access denied. Authentication required - ensure polkit agent is running or use sudo.".to_string(),
                    }
                } else {
                    SysproxError::ServiceControl {
                        service: service_name.to_string(),
                        message: format!("Failed to start: {}", e),
                    }
                }
            })?;

        Ok(())
    }

    /// Stop a service unit
    pub async fn stop_service(&self, service_name: &str) -> Result<()> {
        self.validate_service_name(service_name)?;
        Self::check_polkit_available()?;

        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        let _job_path: zbus::zvariant::OwnedObjectPath = proxy
            .call("StopUnit", &(service_name, "replace"))
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("Access denied") || error_msg.contains("Authentication") {
                    SysproxError::ServiceControl {
                        service: service_name.to_string(),
                        message: "Access denied. Authentication required - ensure polkit agent is running or use sudo.".to_string(),
                    }
                } else {
                    SysproxError::ServiceControl {
                        service: service_name.to_string(),
                        message: format!("Failed to stop: {}", e),
                    }
                }
            })?;

        Ok(())
    }

    /// Restart a service unit
    pub async fn restart_service(&self, service_name: &str) -> Result<()> {
        self.validate_service_name(service_name)?;
        Self::check_polkit_available()?;

        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        let _job_path: zbus::zvariant::OwnedObjectPath = proxy
            .call("RestartUnit", &(service_name, "replace"))
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("Access denied") || error_msg.contains("Authentication") {
                    SysproxError::ServiceControl {
                        service: service_name.to_string(),
                        message: "Access denied. Authentication required - ensure polkit agent is running or use sudo.".to_string(),
                    }
                } else {
                    SysproxError::ServiceControl {
                        service: service_name.to_string(),
                        message: format!("Failed to restart: {}", e),
                    }
                }
            })?;

        Ok(())
    }

    /// Reload a service unit (if supported)
    pub async fn reload_service(&self, service_name: &str) -> Result<()> {
        self.validate_service_name(service_name)?;
        Self::check_polkit_available()?;

        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        let _job_path: zbus::zvariant::OwnedObjectPath = proxy
            .call("ReloadUnit", &(service_name, "replace"))
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("Access denied") || error_msg.contains("Authentication") {
                    SysproxError::ServiceControl {
                        service: service_name.to_string(),
                        message: "Access denied. Authentication required - ensure polkit agent is running or use sudo.".to_string(),
                    }
                } else {
                    SysproxError::ServiceControl {
                        service: service_name.to_string(),
                        message: format!("Failed to reload: {}", e),
                    }
                }
            })?;

        Ok(())
    }

    /// Enable a service unit (creates symlinks)
    pub async fn enable_service(&self, service_name: &str) -> Result<()> {
        self.validate_service_name(service_name)?;
        Self::check_polkit_available()?;

        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        // EnableUnitFiles returns (changes, carries_install_info)
        let (_changes, _carries_install_info): (Vec<(String, String, String)>, bool) = proxy
            .call("EnableUnitFiles", &(&[service_name][..], false, true))
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("Access denied") || error_msg.contains("Authentication") {
                    SysproxError::ServiceControl {
                        service: service_name.to_string(),
                        message: "Access denied. Authentication required - ensure polkit agent is running or use sudo.".to_string(),
                    }
                } else {
                    SysproxError::ServiceControl {
                        service: service_name.to_string(),
                        message: format!("Failed to enable: {}", e),
                    }
                }
            })?;

        // Reload systemd daemon to apply changes
        self.reload_daemon().await?;

        Ok(())
    }

    /// Disable a service unit (removes symlinks)
    pub async fn disable_service(&self, service_name: &str) -> Result<()> {
        self.validate_service_name(service_name)?;
        Self::check_polkit_available()?;

        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        // DisableUnitFiles returns (changes, carries_install_info)
        let (_changes, _carries_install_info): (Vec<(String, String, String)>, bool) = proxy
            .call("DisableUnitFiles", &(&[service_name][..], false))
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("Access denied") || error_msg.contains("Authentication") {
                    SysproxError::ServiceControl {
                        service: service_name.to_string(),
                        message: "Access denied. Authentication required - ensure polkit agent is running or use sudo.".to_string(),
                    }
                } else {
                    SysproxError::ServiceControl {
                        service: service_name.to_string(),
                        message: format!("Failed to disable: {}", e),
                    }
                }
            })?;

        // Reload systemd daemon to apply changes
        self.reload_daemon().await?;

        Ok(())
    }

    /// Check if a service unit is enabled
    pub async fn is_service_enabled(&self, service_name: &str) -> Result<bool> {
        self.validate_service_name(service_name)?;
        
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        // GetUnitFileState returns the state as a string
        let state: String = proxy
            .call("GetUnitFileState", &(service_name,))
            .await
            .map_err(|e| SysproxError::ServiceInfo(format!("Failed to get enabled state: {}", e)))?;

        Ok(state == "enabled" || state == "enabled-runtime")
    }

    /// Get the list of dependencies for a service
    pub async fn get_dependencies(&self, service_name: &str, dependency_type: &str) -> Result<Vec<String>> {
        self.validate_service_name(service_name)?;
        
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        let dependencies: Vec<String> = proxy
            .call("GetDependencies", &(service_name, dependency_type))
            .await
            .map_err(|e| SysproxError::ServiceInfo(format!("Failed to get dependencies: {}", e)))?;

        Ok(dependencies)
    }

    /// Reload the systemd daemon configuration
    async fn reload_daemon(&self) -> Result<()> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        proxy
            .call::<_, _, ()>("Reload", &())
            .await
            .map_err(|e| SysproxError::ServiceControl {
                service: "systemd".to_string(),
                message: format!("Failed to reload daemon: {}", e),
            })?;

        Ok(())
    }

    /// Validate service name format and prevent injection
    pub(crate) fn validate_service_name(&self, service_name: &str) -> Result<()> {
        if service_name.is_empty() {
            return Err(anyhow::anyhow!("Service name cannot be empty"));
        }

        // Basic validation: no path traversal, no null bytes, reasonable length
        if service_name.contains("..") || service_name.contains('\0') || service_name.len() > 256 {
            return Err(anyhow::anyhow!("Invalid service name format"));
        }

        // Ensure it ends with .service for safety
        if !service_name.ends_with(".service") {
            return Err(anyhow::anyhow!("Service name must end with .service"));
        }

        Ok(())
    }
}

// Make ServiceController cloneable for async tasks
impl Clone for ServiceController {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
        }
    }
}
