#[cfg(test)]
mod tests {
    use crate::events::{Action, FilterAction};
    use crate::systemd::{Service, ServiceDetail, LogLine, ServiceScope};
    use crate::ui::{DashboardState, DetailState, LogsState, LogsAction, FilterType};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_dashboard_state_creation() {
        let state = DashboardState::new();
        assert!(state.services.is_empty());
        assert!(matches!(state.filter, FilterType::All));
        assert!(state.search_term.is_empty());
        assert!(state.table_state.selected().is_some());
    }

    #[test]
    fn test_dashboard_filtering() {
        let mut state = DashboardState::new();
        
        // Create test services
        state.set_services(vec![
            Service {
                id: "1.service".to_string(),
                name: "active.service".to_string(),
                description: "Active Service".to_string(),
                load_state: "loaded".to_string(),
                active_state: "active".to_string(),
                sub_state: "running".to_string(),
                pid: 1234,
                enabled: true,
                scope: ServiceScope::System,
                loaded_at: chrono::Utc::now(),
            },
            Service {
                id: "2.service".to_string(),
                name: "failed.service".to_string(),
                description: "Failed Service".to_string(),
                load_state: "loaded".to_string(),
                active_state: "failed".to_string(),
                sub_state: "failed".to_string(),
                pid: 0,
                enabled: true,
                scope: ServiceScope::System,
                loaded_at: chrono::Utc::now(),
            },
            Service {
                id: "3.service".to_string(),
                name: "inactive.service".to_string(),
                description: "Inactive Service".to_string(),
                load_state: "loaded".to_string(),
                active_state: "inactive".to_string(),
                sub_state: "dead".to_string(),
                pid: 0,
                enabled: false,
                scope: ServiceScope::System,
                loaded_at: chrono::Utc::now(),
            },
        ]);

        // Test All filter
        assert_eq!(state.filtered_services().len(), 3);

        // Test Running filter
        state.filter = FilterType::Running;
        assert_eq!(state.filtered_services().len(), 1);
        assert_eq!(state.filtered_services()[0].name, "active.service");

        // Test Failed filter
        state.filter = FilterType::Failed;
        assert_eq!(state.filtered_services().len(), 1);
        assert_eq!(state.filtered_services()[0].name, "failed.service");

        // Test Stopped filter
        state.filter = FilterType::Stopped;
        assert_eq!(state.filtered_services().len(), 1);
        assert_eq!(state.filtered_services()[0].name, "inactive.service");
    }

    #[test]
    fn test_dashboard_search() {
        let mut state = DashboardState::new();
        
        state.set_services(vec![
            Service {
                id: "1.service".to_string(),
                name: "nginx.service".to_string(),
                description: "Nginx Web Server".to_string(),
                load_state: "loaded".to_string(),
                active_state: "active".to_string(),
                sub_state: "running".to_string(),
                pid: 1234,
                enabled: true,
                scope: ServiceScope::System,
                loaded_at: chrono::Utc::now(),
            },
            Service {
                id: "2.service".to_string(),
                name: "database.service".to_string(),
                description: "Database Server".to_string(),
                load_state: "loaded".to_string(),
                active_state: "active".to_string(),
                sub_state: "running".to_string(),
                pid: 5678,
                enabled: true,
                scope: ServiceScope::System,
                loaded_at: chrono::Utc::now(),
            },
        ]);

        // Test name search
        state.search_term = "nginx".to_string();
        assert_eq!(state.filtered_services().len(), 1);
        assert_eq!(state.filtered_services()[0].name, "nginx.service");

        // Test description search
        state.search_term = "database".to_string();
        assert_eq!(state.filtered_services().len(), 1);
        assert_eq!(state.filtered_services()[0].name, "database.service");

        // Test case-insensitive search
        state.search_term = "NGINX".to_string();
        assert_eq!(state.filtered_services().len(), 1);

        // Test search with no results
        state.search_term = "nonexistent".to_string();
        assert_eq!(state.filtered_services().len(), 0);
    }

    #[test]
    fn test_dashboard_navigation() {
        let mut state = DashboardState::new();
        state.set_services(vec![
            Service {
                id: "1.service".to_string(),
                name: "service1.service".to_string(),
                description: "Service 1".to_string(),
                load_state: "loaded".to_string(),
                active_state: "active".to_string(),
                sub_state: "running".to_string(),
                pid: 1234,
                enabled: true,
                scope: ServiceScope::System,
                loaded_at: chrono::Utc::now(),
            },
            Service {
                id: "2.service".to_string(),
                name: "service2.service".to_string(),
                description: "Service 2".to_string(),
                load_state: "loaded".to_string(),
                active_state: "active".to_string(),
                sub_state: "running".to_string(),
                pid: 5678,
                enabled: true,
                scope: ServiceScope::System,
                loaded_at: chrono::Utc::now(),
            },
        ]);

        // Initial selection should be first item
        assert_eq!(state.table_state.selected(), Some(0));

        // Move down
        state.handle_action(Action::MoveDown);
        assert_eq!(state.table_state.selected(), Some(1));

        // Move up
        state.handle_action(Action::MoveUp);
        assert_eq!(state.table_state.selected(), Some(0));

        // Move to bottom
        state.handle_action(Action::MoveBottom);
        assert_eq!(state.table_state.selected(), Some(1));

        // Move to top
        state.handle_action(Action::MoveTop);
        assert_eq!(state.table_state.selected(), Some(0));
    }

    #[test]
    fn test_dashboard_select() {
        let mut state = DashboardState::new();
        state.set_services(vec![
            Service {
                id: "1.service".to_string(),
                name: "test.service".to_string(),
                description: "Test Service".to_string(),
                load_state: "loaded".to_string(),
                active_state: "active".to_string(),
                sub_state: "running".to_string(),
                pid: 1234,
                enabled: true,
                scope: ServiceScope::System,
                loaded_at: chrono::Utc::now(),
            },
        ]);

        // Test select action
        let selected = state.handle_action(Action::Select);
        assert_eq!(selected, Some("test.service".to_string()));
    }

    #[test]
    fn test_dashboard_filter_actions() {
        let mut state = DashboardState::new();
        
        // Test All filter action
        state.handle_action(Action::ToggleFilter(FilterAction::All));
        assert!(matches!(state.filter, FilterType::All));

        // Test Running filter action
        state.handle_action(Action::ToggleFilter(FilterAction::Running));
        assert!(matches!(state.filter, FilterType::Running));

        // Test Failed filter action
        state.handle_action(Action::ToggleFilter(FilterAction::Failed));
        assert!(matches!(state.filter, FilterType::Failed));

        // Test Stopped filter action
        state.handle_action(Action::ToggleFilter(FilterAction::Stopped));
        assert!(matches!(state.filter, FilterType::Stopped));
    }

    #[test]
    fn test_dashboard_stats() {
        let mut state = DashboardState::new();
        state.set_services(vec![
            Service {
                id: "1.service".to_string(),
                name: "active.service".to_string(),
                description: "Active Service".to_string(),
                load_state: "loaded".to_string(),
                active_state: "active".to_string(),
                sub_state: "running".to_string(),
                pid: 1234,
                enabled: true,
                scope: ServiceScope::System,
                loaded_at: chrono::Utc::now(),
            },
            Service {
                id: "2.service".to_string(),
                name: "inactive.service".to_string(),
                description: "Inactive Service".to_string(),
                load_state: "loaded".to_string(),
                active_state: "inactive".to_string(),
                sub_state: "dead".to_string(),
                pid: 0,
                enabled: false,
                scope: ServiceScope::System,
                loaded_at: chrono::Utc::now(),
            },
            Service {
                id: "3.service".to_string(),
                name: "failed.service".to_string(),
                description: "Failed Service".to_string(),
                load_state: "loaded".to_string(),
                active_state: "failed".to_string(),
                sub_state: "failed".to_string(),
                pid: 0,
                enabled: true,
                scope: ServiceScope::System,
                loaded_at: chrono::Utc::now(),
            },
        ]);

        let (total, active, inactive, failed) = state.get_stats();
        assert_eq!(total, 3);
        assert_eq!(active, 1);
        assert_eq!(inactive, 1);
        assert_eq!(failed, 1);
    }

    #[test]
    fn test_detail_state_creation() {
        let state = DetailState::new();
        assert!(state.loading);
        assert!(state.detail.is_none());
        assert!(state.confirmation_dialog.is_none());
    }

    #[test]
    fn test_detail_state_set_detail() {
        let mut state = DetailState::new();

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

        state.set_detail(detail);
        
        assert!(!state.loading);
        assert!(state.detail.is_some());
        assert_eq!(state.detail.as_ref().unwrap().main_pid, 1234);
    }

    #[test]
    fn test_detail_confirmation_dialog() {
        let mut state = DetailState::new();
        
        // Show confirmation dialog
        state.show_confirmation(
            "test.service".to_string(),
            "restart".to_string(),
            "Restart service 'test.service'?".to_string(),
        );
        
        assert!(state.confirmation_dialog.is_some());
        {
            let dialog = state.confirmation_dialog.as_ref().unwrap();
            assert_eq!(dialog.service, "test.service");
            assert_eq!(dialog.operation, "restart");
            assert_eq!(dialog.message, "Restart service 'test.service'?");
            assert!(!dialog.confirmed);
        }

        // Test confirmation
        assert!(state.confirm_action());
        assert!(state.confirmation_dialog.as_ref().unwrap().confirmed);

        // Hide dialog
        state.hide_confirmation();
        assert!(state.confirmation_dialog.is_none());
    }

    #[test]
    fn test_logs_state() {
        let mut state = LogsState::new("test.service".to_string());
        assert_eq!(state.service_name, "test.service");
        assert!(state.lines.is_empty());
        assert_eq!(state.offset, 0);
        assert!(state.follow_mode);

        // Add log lines
        state.add_line(LogLine {
            timestamp: "2024-01-01 12:00:00".to_string(),
            priority: Some(6),
            message: "Line 1".to_string(),
            raw_line: "Line 1".to_string(),
            is_live: false,
        });
        state.add_line(LogLine {
            timestamp: "2024-01-01 12:00:01".to_string(),
            priority: Some(6),
            message: "Line 2".to_string(),
            raw_line: "Line 2".to_string(),
            is_live: false,
        });
        state.add_line(LogLine {
            timestamp: "2024-01-01 12:00:02".to_string(),
            priority: Some(6),
            message: "Line 3".to_string(),
            raw_line: "Line 3".to_string(),
            is_live: false,
        });

        assert_eq!(state.lines.len(), 3);

        // Test scrolling
        // Follow mode should keep us near the bottom
        assert!(state.follow_mode);

        // Scroll up
        state.handle_action(LogsAction::ScrollUp);
        assert!(state.offset > 0);
        assert!(!state.follow_mode);

        // Scroll to bottom
        state.handle_action(LogsAction::ScrollBottom);
        assert!(state.follow_mode);
    }

    #[test]
    fn test_styles() {
        assert_eq!(state_color("active"), SUCCESS);
        assert_eq!(state_color("failed"), ERROR);
        assert_eq!(state_color("inactive"), MUTED);
        assert_eq!(state_color("activating"), WARNING);

        assert_eq!(state_icon("active"), "●");
        assert_eq!(state_icon("failed"), "✗");
        assert_eq!(state_icon("inactive"), "○");
        assert_eq!(state_icon("activating"), "◐");
    }

    #[test]
    fn test_dashboard_rendering() {
        let mut state = DashboardState::new();
        state.set_services(vec![
            Service {
                id: "1.service".to_string(),
                name: "test.service".to_string(),
                description: "Test Service".to_string(),
                load_state: "loaded".to_string(),
                active_state: "active".to_string(),
                sub_state: "running".to_string(),
                pid: 1234,
                enabled: true,
                scope: ServiceScope::System,
                loaded_at: chrono::Utc::now(),
            },
        ]);

        // Create a test backend and terminal
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // This should not panic
        let _ = terminal.draw(|f| {
            state.render(f, f.area(), true);
        });

        // Basic verification that drawing worked
        let backend = terminal.backend();
        let buffer = backend.buffer();
        assert!(buffer.area.width > 0 && buffer.area.height > 0);
    }

    #[test]
    fn test_filter_type_label() {
        assert_eq!(FilterType::All.label(), "All");
        assert_eq!(FilterType::Running.label(), "Running");
        assert_eq!(FilterType::Stopped.label(), "Stopped");
        assert_eq!(FilterType::Failed.label(), "Failed");
    }

    #[test]
    fn test_filter_type_equality() {
        assert_eq!(FilterType::All, FilterType::All);
        assert_ne!(FilterType::All, FilterType::Running);
        
        let filter1 = FilterType::Failed;
        let filter2 = FilterType::Failed;
        assert_eq!(filter1, filter2);
    }

    // Import styles for testing
    use crate::ui::styles::*;
}