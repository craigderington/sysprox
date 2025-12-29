// Sysprox - Systemd Service Monitor TUI
// Main entry point

use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;
use sysprox::app::App;
use sysprox::config::Config;
use sysprox::events::{AppEvent, spawn_input_handler, spawn_ticker};
use sysprox::version::build_info;
use tokio::sync::mpsc;

#[derive(Parser, Debug)]
#[command(name = "sysprox")]
#[command(author, about, long_about = None)]
#[command(disable_version_flag = true)]
struct Cli {
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Config file path
    #[arg(short, long)]
    config: Option<String>,

    /// Show version information
    #[arg(short = 'V', long)]
    version: bool,

    /// Show detailed build information
    #[arg(long)]
    build_info: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle version flag
    if cli.version {
        println!("{}", build_info().format_display());
        return Ok(());
    }

    // Handle build info flag
    if cli.build_info {
        println!("{}", build_info().format_display());
        println!("\n{}", build_info().format_build_info());
        return Ok(());
    }

    // Initialize logging to file
    let log_file = std::fs::File::create("/tmp/sysprox.log")?;
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(if cli.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .with_writer(std::sync::Mutex::new(log_file))
        .with_ansi(false) // Disable ANSI colors in log file
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    tracing::info!("Sysprox starting, logging to /tmp/sysprox.log");

    // Run the TUI
    run_tui(cli.config).await?;

    Ok(())
}

async fn run_tui(config_path: Option<String>) -> Result<()> {
    // Load configuration
    let config = Config::load(config_path.map(std::path::PathBuf::from))?;
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create event channel
    let (tx, mut rx) = mpsc::channel::<AppEvent>(100);

    // Spawn input handler
    spawn_input_handler(tx.clone()).await;

    // Spawn ticker for periodic refresh (use config setting)
    spawn_ticker(tx.clone(), Duration::from_secs(config.service_list_refresh_secs)).await;

    // Create app
    let mut app = App::new(tx.clone()).await?;

    // Initial service load
    let services = app.client.list_services().await?;
    tx.send(AppEvent::ServicesLoaded(services)).await.ok();

    // Main event loop
    loop {
        // Clear terminal if full redraw is needed (e.g., after view change)
        if app.needs_full_redraw {
            terminal.clear()?;
            app.needs_full_redraw = false;
        }

        // Render UI
        terminal.draw(|f| app.render(f))?;

        // Handle events
        if let Some(event) = rx.recv().await {
            app.handle_event(event).await?;

            if app.should_quit {
                break;
            }
        }
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!("Sysprox exited. Goodbye!");

    Ok(())
}