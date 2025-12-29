// Ratatui styling and color palette

use ratatui::style::{Color, Modifier, Style};

// Color palette
pub const PRIMARY: Color = Color::Cyan;
pub const SECONDARY: Color = Color::Blue;
pub const ACCENT: Color = Color::Magenta;
pub const SUCCESS: Color = Color::Green;
pub const WARNING: Color = Color::Yellow;
pub const ERROR: Color = Color::Red;
pub const MUTED: Color = Color::Gray;

// Common styles
pub fn title_style() -> Style {
    Style::default()
        .fg(PRIMARY)
        .add_modifier(Modifier::BOLD)
}

pub fn help_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn active_style() -> Style {
    Style::default().fg(SUCCESS)
}

pub fn failed_style() -> Style {
    Style::default().fg(ERROR)
}

pub fn inactive_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn selected_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD)
}

/// Get color for service state (active_state)
pub fn state_color(state: &str) -> Color {
    match state {
        // Active states - Green
        "active" => SUCCESS,
        "reloading" => SUCCESS,

        // Inactive states - Gray
        "inactive" => MUTED,

        // Transitioning states - Yellow
        "activating" => WARNING,
        "deactivating" => WARNING,

        // Failed states - Red
        "failed" => ERROR,
        "maintenance" => ERROR,

        // Default - White
        _ => Color::White,
    }
}

/// Get color for load state
pub fn load_state_color(state: &str) -> Color {
    match state {
        // Successfully loaded - Green
        "loaded" => SUCCESS,
        "stub" => SUCCESS,

        // Warnings - Yellow
        "masked" => WARNING,
        "bad-setting" => WARNING,

        // Errors - Red
        "not-found" => ERROR,
        "error" => ERROR,
        "merged" => ERROR,

        // Default - White
        _ => Color::White,
    }
}

/// Get color for service result
pub fn result_color(result: &str) -> Color {
    match result {
        // Success - Green
        "success" => SUCCESS,
        "done" => SUCCESS,

        // Warnings - Yellow
        "timeout" => WARNING,
        "start-limit-hit" => WARNING,
        "start-limit" => WARNING,
        "resources" => WARNING,

        // Errors - Red
        "failure" => ERROR,
        "exit-code" => ERROR,
        "signal" => ERROR,
        "core-dump" => ERROR,
        "watchdog" => ERROR,
        "protocol" => ERROR,

        // Default - White
        _ => Color::White,
    }
}

/// Get color for sub state
pub fn sub_state_color(sub_state: &str) -> Color {
    match sub_state {
        // Active/running states - Green
        "running" => SUCCESS,
        "exited" => SUCCESS,
        "plugged" => SUCCESS,
        "mounted" => SUCCESS,
        "listening" => SUCCESS,

        // Transitioning states - Yellow
        "waiting" => WARNING,
        "start" => WARNING,
        "start-pre" => WARNING,
        "start-post" => WARNING,
        "reload" => WARNING,
        "stop" => WARNING,
        "stop-pre" => WARNING,
        "stop-post" => WARNING,
        "stop-sigterm" => WARNING,
        "stop-sigkill" => WARNING,
        "final-sigterm" => WARNING,
        "final-sigkill" => WARNING,
        "auto-restart" => WARNING,
        "condition" => WARNING,
        "activating" => WARNING,
        "deactivating" => WARNING,

        // Inactive/stopped states - Yellow
        "inactive" => WARNING,
        "unmounted" => WARNING,

        // Error/failed states - Red
        "dead" => ERROR,
        "failed" => ERROR,
        "maintenance" => ERROR,

        // Default - White
        _ => Color::White,
    }
}

/// Get icon for service state
pub fn state_icon(state: &str) -> &'static str {
    match state {
        "active" => "●",
        "failed" => "✗",
        "inactive" => "○",
        "activating" | "deactivating" => "◐",
        _ => "?",
    }
}

/// Get colored emoji for service state
pub fn status_emoji(state: &str) -> &'static str {
    match state {
        // Active/running states - Green
        "active" => "🟢",
        "reloading" => "🟢",

        // Inactive/stopped states - Blue
        "inactive" => "🔵",

        // Transitioning states - Orange
        "activating" => "🟠",
        "deactivating" => "🟠",

        // Failed/error states - Red
        "failed" => "🔴",
        "maintenance" => "🔴",

        // Unknown - Gray/White circle
        _ => "⚪",
    }
}

/// Get color for log priority level
pub fn priority_color(priority: u8) -> Option<Style> {
    let color = match priority {
        0 => Color::Red,       // emerg
        1 => Color::Red,       // alert  
        2 => Color::Red,       // crit
        3 => Color::LightRed,  // err
        4 => Color::Yellow,    // warning
        5 => Color::White,     // notice
        6 => Color::Cyan,      // info
        7 => Color::Blue,      // debug
        _ => return None,
    };
    
    Some(Style::default().fg(color))
}
