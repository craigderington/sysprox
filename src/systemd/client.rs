// Systemd D-Bus client using zbus

use crate::error::{Result, SysproxError};
use crate::systemd::{ConnectionManager, Metrics, Service, ServiceDetail, ServiceScope, ServiceStatusExtended};
use chrono::Utc;
use zbus::Connection;

/// Systemd D-Bus client with resilient connection handling
pub struct SystemdClient {
    pub(crate) connection: Connection,
    user_connection: Option<Connection>,
    connection_manager: ConnectionManager,
}

impl SystemdClient {
    /// Create a new systemd client connected to the system bus
    pub async fn new() -> Result<Self> {
        let connection_manager = ConnectionManager::default();

        let connection = connection_manager
            .connect_systemd()
            .await?;

        // Try to connect to user bus (may fail if no session)
        let user_connection = Connection::session().await.ok();

        Ok(Self {
            connection,
            user_connection,
            connection_manager,
        })
    }

    /// Create a new systemd client with custom retry settings
    pub async fn new_with_retry(max_retries: usize, retry_delay: std::time::Duration) -> Result<Self> {
        let connection_manager = ConnectionManager::new(
            max_retries,
            retry_delay,
            std::time::Duration::from_secs(5),
        );

        let connection = connection_manager
            .connect_systemd()
            .await?;

        // Try to connect to user bus (may fail if no session)
        let user_connection = Connection::session().await.ok();

        Ok(Self {
            connection,
            user_connection,
            connection_manager,
        })
    }

    /// List all systemd service units from all scopes (system + user)
    pub async fn list_services(&self) -> Result<Vec<Service>> {
        let mut all_services = Vec::new();

        // Get system services
        let system_services = self.list_services_by_scope(ServiceScope::System).await?;
        all_services.extend(system_services);

        // Get user services if available
        if self.user_connection.is_some() {
            if let Ok(user_services) = self.list_services_by_scope(ServiceScope::User).await {
                all_services.extend(user_services);
            }
        }

        Ok(all_services)
    }

    /// List systemd service units from a specific scope
    pub async fn list_services_by_scope(&self, scope: ServiceScope) -> Result<Vec<Service>> {
        self.connection_manager.with_retry("list_services_by_scope", || async {
            // Select the appropriate connection
            let connection = match scope {
                ServiceScope::System => &self.connection,
                ServiceScope::User => self.user_connection.as_ref()
                    .ok_or_else(|| SysproxError::SystemdConnection("User bus not available".to_string()))?,
            };

            // Call systemd's ListUnits method via D-Bus
            let proxy = zbus::Proxy::new(
                connection,
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
            )
            .await
            .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

            // ListUnits returns array of (name, description, load_state, active_state, sub_state,
            //                             followed, unit_path, job_id, job_type, job_path)
            #[allow(clippy::type_complexity)]
            let units: Vec<(
                String, // name
                String, // description
                String, // load_state
                String, // active_state
                String, // sub_state
                String, // followed
                zbus::zvariant::OwnedObjectPath, // unit_path
                u32,    // job_id
                String, // job_type
                zbus::zvariant::OwnedObjectPath, // job_path
            )> = proxy
                .call("ListUnits", &())
                .await
                .map_err(|e| SysproxError::ServiceInfo(e.to_string()))?;

            let mut services = Vec::new();
            let mut service_names = std::collections::HashSet::new();

            // First, add all loaded units
            for (name, description, load_state, active_state, sub_state, _, _, _, _, _) in units {
                // Filter for service units only
                if name.ends_with(".service") {
                    service_names.insert(name.clone());
                    // Check if service is enabled
                    let is_enabled = self.check_service_enabled_in_scope(&name, scope).await.unwrap_or(false);

                    services.push(Service {
                        id: format!("{}:{}", scope.label(), name),
                        name,
                        description,
                        load_state,
                        active_state,
                        sub_state,
                        pid: 0, // Will be filled by get_service_detail if needed
                        enabled: is_enabled,
                        scope,
                        loaded_at: Utc::now(), // TODO: Get actual load time
                    });
                }
            }

            // Then, add unloaded service files (for user scope)
            if scope == ServiceScope::User {
                let unit_files: Vec<(String, String)> = proxy
                    .call("ListUnitFiles", &())
                    .await
                    .unwrap_or_default();

                for (path, state) in unit_files {
                    // Extract service name from path
                    if let Some(name) = path.split('/').next_back() {
                        if name.ends_with(".service") && !service_names.contains(name) {
                            let is_enabled = state == "enabled";
                            services.push(Service {
                                id: format!("{}:{}", scope.label(), name),
                                name: name.to_string(),
                                description: "(not loaded)".to_string(),
                                load_state: "not-loaded".to_string(),
                                active_state: "inactive".to_string(),
                                sub_state: "dead".to_string(),
                                pid: 0,
                                enabled: is_enabled,
                                scope,
                                loaded_at: Utc::now(),
                            });
                        }
                    }
                }
            }

            Ok(services)
        }).await
    }

    /// Get detailed information for a specific service
    pub async fn get_service_detail(&self, service_name: &str) -> Result<ServiceDetail> {
        self.connection_manager.with_retry("get_service_detail", || async {
            // First get basic service info from ListUnits
            let services = self.list_services().await?;
            let service = services
                .into_iter()
                .find(|s| s.name == service_name)
                .ok_or_else(|| {
                    SysproxError::ServiceInfo(format!("Service '{}' not found", service_name))
                })?;

            // Get unit properties
            let props = self.get_unit_properties(service_name).await?;

            // Extract properties with safe unwrapping
            let main_pid = props
                .get("MainPID")
                .and_then(|v| v.downcast_ref::<u32>().ok())
                .unwrap_or(0);

            let control_pid = props
                .get("ControlPID")
                .and_then(|v| v.downcast_ref::<u32>().ok())
                .unwrap_or(0);

            let load_path = props
                .get("FragmentPath")
                .and_then(|v| v.downcast_ref::<String>().ok())
                .unwrap_or_default();

            let memory_current = props
                .get("MemoryCurrent")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .unwrap_or(0);

            let memory_limit = props
                .get("MemoryLimit")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .unwrap_or(u64::MAX);

            let cpu_usage_nsec = props
                .get("CPUUsageNSec")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .unwrap_or(0);

            tracing::debug!("CPUUsageNSec for {}: {} ns", service_name, cpu_usage_nsec);

            let tasks_current = props
                .get("TasksCurrent")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .unwrap_or(0);

            let tasks_max = props
                .get("TasksMax")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .unwrap_or(0);

            let n_restarts = props
                .get("NRestarts")
                .and_then(|v| v.downcast_ref::<u32>().ok())
                .unwrap_or(0);

            let result = props
                .get("Result")
                .and_then(|v| v.downcast_ref::<String>().ok())
                .unwrap_or_default();

            // Extract dependencies (these are arrays of strings)
            let wants = extract_string_array(&props, "Wants");
            let wanted_by = extract_string_array(&props, "WantedBy");
            let after = extract_string_array(&props, "After");
            let before = extract_string_array(&props, "Before");

            // Extract service configuration details
            let service_type = props
                .get("Type")
                .and_then(|v| v.downcast_ref::<String>().ok())
                .unwrap_or_default();

            let restart = props
                .get("Restart")
                .and_then(|v| v.downcast_ref::<String>().ok())
                .unwrap_or_default();

            let user = props
                .get("User")
                .and_then(|v| v.downcast_ref::<String>().ok())
                .unwrap_or_default();

            let group = props
                .get("Group")
                .and_then(|v| v.downcast_ref::<String>().ok())
                .unwrap_or_default();

            let working_directory = props
                .get("WorkingDirectory")
                .and_then(|v| v.downcast_ref::<String>().ok())
                .unwrap_or_default();

            let environment = extract_string_array(&props, "Environment");

            // Extract ExecStart command
            let exec_main_start = if let Some(exec_start_value) = props.get("ExecStart") {
                // Convert to string and try to extract the path
                let exec_str = format!("{:?}", exec_start_value);
                tracing::debug!("ExecStart raw for {}: {}", service_name, &exec_str.chars().take(200).collect::<String>());

                // Try multiple patterns to extract the command path:
                // 1. Look for "path: "/path/to/cmd""
                if let Some(start_idx) = exec_str.find("path: \"") {
                    if let Some(end_idx) = exec_str[start_idx + 7..].find('"') {
                        exec_str[start_idx + 7..start_idx + 7 + end_idx].to_string()
                    } else {
                        "N/A".to_string()
                    }
                }
                // 2. Look for any path starting with "/" (typical Unix executable paths)
                else if let Some(start_idx) = exec_str.find("\"/") {
                    // Find the closing quote
                    if let Some(end_idx) = exec_str[start_idx + 1..].find('"') {
                        exec_str[start_idx + 1..start_idx + 1 + end_idx].to_string()
                    } else {
                        "N/A".to_string()
                    }
                }
                // 3. Fallback: Show "N/A" instead of ugly debug output
                else {
                    "N/A".to_string()
                }
            } else {
                "N/A".to_string()
            };

            tracing::debug!("ExecStart parsed for {}: {}", service_name, exec_main_start);

            // Extract timestamps (systemd returns microseconds since epoch)
            // Use epoch (Jan 1, 1970) as fallback instead of Utc::now() to avoid uptime=0 issues
            let active_enter_micros_raw = props
                .get("ActiveEnterTimestamp")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .unwrap_or(0);

            tracing::debug!("ActiveEnterTimestamp for {}: {} micros", service_name, active_enter_micros_raw);

            let active_enter_time = if active_enter_micros_raw > 0 {
                chrono::DateTime::from_timestamp(
                    (active_enter_micros_raw / 1_000_000) as i64,
                    ((active_enter_micros_raw % 1_000_000) * 1000) as u32
                ).unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap())
            } else {
                chrono::DateTime::from_timestamp(0, 0).unwrap()
            };

            tracing::debug!("Parsed active_enter_time for {}: {}", service_name, active_enter_time);

            let active_exit_time = props
                .get("ActiveExitTimestamp")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .and_then(|micros| {
                    if micros > 0 {
                        chrono::DateTime::from_timestamp(
                            (micros / 1_000_000) as i64,
                            ((micros % 1_000_000) * 1000) as u32
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());

            let inactive_enter_time = props
                .get("InactiveEnterTimestamp")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .and_then(|micros| {
                    if micros > 0 {
                        chrono::DateTime::from_timestamp(
                            (micros / 1_000_000) as i64,
                            ((micros % 1_000_000) * 1000) as u32
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());

            let state_change_time = props
                .get("StateChangeTimestamp")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .and_then(|micros| {
                    if micros > 0 {
                        chrono::DateTime::from_timestamp(
                            (micros / 1_000_000) as i64,
                            ((micros % 1_000_000) * 1000) as u32
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());

            // Create ServiceDetail
            let detail = ServiceDetail {
                service,
                main_pid,
                control_pid,
                load_path,
                exec_main_start,
                exec_main_status: String::new(),
                memory_current,
                memory_limit,
                cpu_usage_nsec,
                tasks_current,
                tasks_max,
                n_restarts,
                active_enter_time,
                active_exit_time,
                inactive_enter_time,
                state_change_time,
                result,
                wants,
                wanted_by,
                after,
                before,
                service_type,
                restart,
                user,
                group,
                working_directory,
                environment,
            };

            Ok(detail)
        }).await
    }

    /// Get unit properties from systemd
    async fn get_unit_properties(
        &self,
        unit_name: &str,
    ) -> Result<std::collections::HashMap<String, zbus::zvariant::OwnedValue>> {
        self.connection_manager.with_retry("get_unit_properties", || async {
            let proxy = zbus::Proxy::new(
                &self.connection,
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
            )
            .await
            .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

            // LoadUnit loads the unit into memory if needed and returns the object path
            // This works for both loaded and not-loaded units, unlike GetUnit
            let unit_path: zbus::zvariant::OwnedObjectPath = proxy
                .call("LoadUnit", &(unit_name,))
                .await
                .map_err(|e| SysproxError::ServiceInfo(e.to_string()))?;

            // GetAll on the Properties interface
            let props_proxy = zbus::fdo::PropertiesProxy::builder(&self.connection)
                .destination("org.freedesktop.systemd1")?
                .path(unit_path.as_str())?
                .build()
                .await
                .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

            // Get all properties from the Service interface
            use zbus::zvariant::Optional;
            let props = props_proxy
                .get_all(Optional::default())  // Get all properties
                .await
                .map_err(|e| SysproxError::ServiceInfo(e.to_string()))?;

            Ok(props)
        }).await
    }

    /// Get current metrics for a service
    pub async fn get_service_metrics(&self, service_name: &str) -> Result<Metrics> {
        self.connection_manager.with_retry("get_service_metrics", || async {
            let props = self.get_unit_properties(service_name).await?;

            let cpu_usage_nsec = props
                .get("CPUUsageNSec")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .unwrap_or(0);

            let memory_current = props
                .get("MemoryCurrent")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .unwrap_or(0);

            let tasks_current = props
                .get("TasksCurrent")
                .and_then(|v| v.downcast_ref::<u64>().ok())
                .unwrap_or(0);

            let n_restarts = props
                .get("NRestarts")
                .and_then(|v| v.downcast_ref::<u32>().ok())
                .unwrap_or(0);

            Ok(Metrics {
                cpu_usage_nsec,
                memory_current,
                tasks_current,
                n_restarts,
            })
        }).await
    }

    /// Check if a service is enabled (starts on boot)
    pub async fn is_service_enabled(&self, service_name: &str) -> Result<bool> {
        self.connection_manager.with_retry("is_service_enabled", || async {
            let proxy = zbus::Proxy::new(
                &self.connection,
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
            )
            .await
            .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

            // GetUnitFileState returns the unit file state
            let state: String = proxy
                .call("GetUnitFileState", &(service_name,))
                .await
                .map_err(|e| SysproxError::ServiceInfo(format!("Failed to get unit file state: {}", e)))?;

            Ok(state == "enabled" || state == "enabled-runtime")
        }).await
    }

    /// Get all enabled services
    pub async fn list_enabled_services(&self) -> Result<Vec<String>> {
        self.connection_manager.with_retry("list_enabled_services", || async {
            let proxy = zbus::Proxy::new(
                &self.connection,
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
            )
            .await
            .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

            // ListUnitFiles returns array of unit file info
            let unit_files: Vec<(String, String)> = proxy
                .call("ListUnitFiles", &(&[] as &[&str], false))
                .await
                .map_err(|e| SysproxError::ServiceInfo(format!("Failed to list unit files: {}", e)))?;

            let enabled_services = unit_files
                .into_iter()
                .filter_map(|(path, state)| {
                    if path.ends_with(".service") && (state == "enabled" || state == "enabled-runtime") {
                        Some(path)
                    } else {
                        None
                    }
                })
                .collect();

            Ok(enabled_services)
        }).await
    }

    /// Get service status with enable/disable information
    pub async fn get_service_status_extended(&self, service_name: &str) -> Result<ServiceStatusExtended> {
        let service = self.get_service_detail(service_name).await?;
        let is_enabled = self.is_service_enabled(service_name).await?;

        Ok(ServiceStatusExtended {
            service,
            is_enabled,
        })
    }

    /// Check if service is enabled in a specific scope
    async fn check_service_enabled_in_scope(&self, service_name: &str, scope: ServiceScope) -> Result<bool> {
        let connection = match scope {
            ServiceScope::System => &self.connection,
            ServiceScope::User => self.user_connection.as_ref()
                .ok_or_else(|| SysproxError::SystemdConnection("User bus not available".to_string()))?,
        };

        let proxy = zbus::Proxy::new(
            connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .map_err(|e| SysproxError::SystemdConnection(e.to_string()))?;

        let state: String = proxy
            .call("GetUnitFileState", &(service_name,))
            .await
            .map_err(|e| SysproxError::ServiceInfo(format!("Failed to get unit file state: {}", e)))?;

        Ok(state == "enabled" || state == "enabled-runtime")
    }
}

/// Helper to extract string arrays from D-Bus properties
fn extract_string_array(
    props: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
    key: &str,
) -> Vec<String> {
    props
        .get(key)
        .and_then(|v| v.downcast_ref::<zbus::zvariant::Array>().ok())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.downcast_ref::<String>().ok())
                .collect()
        })
        .unwrap_or_default()
}

// Make SystemdClient cloneable for spawning tasks
impl Clone for SystemdClient {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            user_connection: self.user_connection.clone(),
            connection_manager: self.connection_manager.clone(),
        }
    }
}
