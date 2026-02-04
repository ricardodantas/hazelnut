//! Hazelnut Daemon (hazelnutd)
//!
//! Background service that watches directories and applies rules.

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "hazelnutd")]
#[command(author, version, about = "Hazelnut background daemon")]
struct Cli {
    /// Path to config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<std::path::PathBuf>,

    /// Run in foreground (don't daemonize)
    #[arg(short, long)]
    foreground: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Start the daemon
    Start,

    /// Stop the daemon
    Stop,

    /// Restart the daemon
    Restart,

    /// Show daemon status
    Status,

    /// Reload configuration
    Reload,

    /// Run in foreground (for debugging)
    Run,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("HAZELNUT_LOG").unwrap_or_else(|_| "info".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    match cli.command {
        Commands::Start => {
            println!("Starting hazelnut daemon...");
            if cli.foreground {
                run_daemon(cli.config).await?;
            } else {
                // TODO: Daemonize
                println!("Daemonization not implemented yet. Use --foreground or 'hazelnutd run'");
            }
        }
        Commands::Stop => {
            println!("Stopping hazelnut daemon...");
            // TODO: Send stop signal via IPC
        }
        Commands::Restart => {
            println!("Restarting hazelnut daemon...");
            // TODO: Stop then start
        }
        Commands::Status => {
            println!("Daemon status: not running");
            // TODO: Check via IPC
        }
        Commands::Reload => {
            println!("Reloading configuration...");
            // TODO: Send reload signal via IPC
        }
        Commands::Run => {
            run_daemon(cli.config).await?;
        }
    }

    Ok(())
}

async fn run_daemon(config_path: Option<std::path::PathBuf>) -> Result<()> {
    use tokio::signal;
    use tracing::info;

    let config = hazelnut::Config::load(config_path.as_deref())?;
    info!(
        "Loaded config with {} watch paths and {} rules",
        config.watches.len(),
        config.rules.len()
    );

    let engine = hazelnut::RuleEngine::new(config.rules);
    let mut watcher = hazelnut::Watcher::new(engine)?;

    for watch in &config.watches {
        info!("Watching: {}", watch.path.display());
        watcher.watch(&watch.path, watch.recursive)?;
    }

    info!("Daemon running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    info!("Shutting down...");

    Ok(())
}
