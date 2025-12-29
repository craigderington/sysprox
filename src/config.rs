// Configuration management

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub theme: String,
    pub icons: bool,
    pub vim_mode: bool,
    pub show_inactive: bool,
    pub show_disabled: bool,
    pub log_lines: usize,
    pub log_follow_by_default: bool,
    pub log_priority: String,
    pub metrics_refresh_secs: u64,
    pub service_list_refresh_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            icons: true,
            vim_mode: true,
            show_inactive: false,
            show_disabled: false,
            log_lines: 100,
            log_follow_by_default: true,
            log_priority: "info".to_string(),
            metrics_refresh_secs: 2,
            service_list_refresh_secs: 5,
        }
    }
}

impl Config {
    /// Get default config path: ~/.config/sysprox/config.yaml
    pub fn default_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        Ok(config_dir.join("sysprox").join("config.yaml"))
    }

    /// Load config from path, falling back to defaults if not found
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let config_path = path.unwrap_or_else(|| Self::default_path().unwrap_or_default());

        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)?;
            let config: Config = serde_yaml::from_str(&contents)?;
            Ok(config)
        } else {
            // Return defaults if no config file exists
            Ok(Self::default())
        }
    }

    /// Save config to path
    pub fn save(&self, path: PathBuf) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }
}
