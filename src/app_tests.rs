#[cfg(test)]
mod tests {
    use crate::app::*;
    use crate::error::Result;
    use crate::events::{Action, AppEvent, FilterAction, key_event_to_action};
    use crate::systemd::{Service, ServiceScope};
    use crate::ui::{DashboardState, DetailState, LogsState};
    use chrono::Utc;

    #[tokio::test]
    async fn test_app_creation() -> Result<()> {
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        let app = App::new(tx).await?;
        assert!(!app.should_quit);
        Ok(())
    }

    #[tokio::test]
    async fn test_event_handling() -> Result<()> {
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        let mut app = App::new(tx).await?;

        // Test service loading
        let services = vec![Service {
            id: "test.service".to_string(),
            name: "test.service".to_string(),
            description: "Test Service".to_string(),
            load_state: "loaded".to_string(),
            active_state: "active".to_string(),
            sub_state: "running".to_string(),
            pid: 1234,
            enabled: true,
            scope: ServiceScope::System,
            loaded_at: Utc::now(),
        }];

        app.handle_event(AppEvent::ServicesLoaded(services)).await?;
        assert!(matches!(app.view, View::Dashboard(_)));

        Ok(())
    }

    #[test]
    fn test_action_conversions() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        // Test quit actions
        let quit_action = key_event_to_action(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(quit_action, Action::Quit);

        let ctrl_c_action = key_event_to_action(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(ctrl_c_action, Action::Quit);

        // Test navigation
        let up_action = key_event_to_action(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(up_action, Action::MoveUp);

        let k_action = key_event_to_action(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(k_action, Action::MoveUp);

        let down_action = key_event_to_action(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(down_action, Action::MoveDown);

        let j_action = key_event_to_action(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(j_action, Action::MoveDown);

        // Test service control
        let start_action = key_event_to_action(KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT));
        assert_eq!(start_action, Action::StartService);

        let stop_action = key_event_to_action(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
        assert_eq!(stop_action, Action::StopService);

        // Test filtering
        let all_action = key_event_to_action(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(all_action, Action::ToggleFilter(FilterAction::All));

        let running_action = key_event_to_action(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        assert_eq!(running_action, Action::ToggleFilter(FilterAction::Running));
    }

    #[test]
    fn test_filter_action_equality() {
        assert_eq!(FilterAction::All, FilterAction::All);
        assert_ne!(FilterAction::All, FilterAction::Running);
        
        let filter1 = FilterAction::Failed;
        let filter2 = FilterAction::Failed;
        assert_eq!(filter1, filter2);
    }

    #[tokio::test]
    async fn test_event_channel() -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);

        // Test sending different event types
        assert!(tx.send(AppEvent::Quit).await.is_ok());
        assert!(tx.send(AppEvent::StatusMessage("Test".to_string())).await.is_ok());

        // Test receiving events
        if let Some(event) = rx.recv().await {
            match event {
                AppEvent::Quit => {
                    // Expected
                }
                AppEvent::StatusMessage(msg) => {
                    assert_eq!(msg, "Test");
                }
                _ => panic!("Unexpected event type"),
            }
        } else {
            panic!("No event received");
        }

        Ok(())
    }

    #[test]
    fn test_service_helper() {
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
            loaded_at: Utc::now(),
        };

        assert!(service.is_active());
        assert!(!service.is_failed());
        assert!(!service.is_inactive());
        assert!(!service.is_transitioning());

        let failed_service = Service {
            active_state: "failed".to_string(),
            ..service.clone()
        };
        assert!(failed_service.is_failed());
        assert!(!failed_service.is_active());

        let activating_service = Service {
            active_state: "activating".to_string(),
            ..service.clone()
        };
        assert!(activating_service.is_transitioning());
    }

    #[tokio::test]
    async fn test_app_lifecycle() -> Result<()> {
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        let mut app = App::new(tx).await?;

        // Test initial state
        assert!(!app.should_quit);
        assert!(matches!(app.view, View::Dashboard(_)));

        // Test quit event
        app.handle_event(AppEvent::Quit).await?;
        assert!(app.should_quit);

        Ok(())
    }

    #[test]
    fn test_view_states() {
        let dashboard = View::Dashboard(DashboardState::new());
        let detail = View::Detail(Box::new(DetailState::new()));
        let logs = View::Logs(LogsState::new("test.service".to_string()));

        assert!(matches!(dashboard, View::Dashboard(_)));
        assert!(matches!(detail, View::Detail(_)));
        assert!(matches!(logs, View::Logs(_)));
    }
}