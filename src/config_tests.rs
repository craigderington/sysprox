#[cfg(test)]
mod tests {
    use crate::config::*;
    use crate::error::Result;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.theme, "dark");
        assert!(config.icons);
        assert!(config.vim_mode);
        assert!(!config.show_inactive);
        assert!(!config.show_disabled);
        assert_eq!(config.log_lines, 100);
        assert!(config.log_follow_by_default);
        assert_eq!(config.log_priority, "info");
        assert_eq!(config.metrics_refresh_secs, 2);
        assert_eq!(config.service_list_refresh_secs, 5);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            theme: "light".to_string(),
            icons: false,
            vim_mode: false,
            show_inactive: true,
            show_disabled: true,
            log_lines: 200,
            log_follow_by_default: false,
            log_priority: "debug".to_string(),
            metrics_refresh_secs: 1,
            service_list_refresh_secs: 3,
        };

        // Test serialization
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("light"));
        assert!(yaml.contains("debug"));

        // Test deserialization
        let deserialized: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(deserialized.theme, "light");
        assert_eq!(deserialized.log_priority, "debug");
        assert_eq!(deserialized.metrics_refresh_secs, 1);
    }

    #[test]
    fn test_config_default_path() {
        let path = Config::default_path();
        assert!(path.is_ok());
        
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains(".config"));
        assert!(path.to_string_lossy().contains("sysprox"));
        assert!(path.to_string_lossy().contains("config.yaml"));
    }

    #[test]
    fn test_config_load_missing() -> Result<()> {
        // Test loading non-existent config (should return defaults)
        let config = Config::load(Some("/nonexistent/config.yaml".into()))?;
        assert_eq!(config.theme, "dark"); // Should be default
        
        Ok(())
    }

    #[test]
    fn test_config_save_load() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_config.yaml");

        // Create custom config
        let original_config = Config {
            theme: "custom".to_string(),
            ..Config::default()
        };

        // Save config
        original_config.save(config_path.clone())?;

        // Load config
        let loaded_config = Config::load(Some(config_path.clone()))?;

        // Verify it matches
        assert_eq!(loaded_config.theme, "custom");
        assert_eq!(loaded_config.icons, original_config.icons);
        assert_eq!(loaded_config.vim_mode, original_config.vim_mode);

        // Cleanup
        std::fs::remove_file(config_path)?;

        Ok(())
    }
}