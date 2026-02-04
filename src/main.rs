//! Hazelnut TUI Application
//!
//! Terminal user interface for managing file organization rules.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "hazelnut")]
#[command(author, version, about = "Terminal-based automated file organizer")]
struct Cli {
    /// Path to config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Start the TUI (default)
    Ui,

    /// List all rules
    List,

    /// Validate config file
    Check {
        /// Path to config file to validate
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Run rules once without watching (dry-run by default)
    Run {
        /// Actually perform actions (not just dry-run)
        #[arg(long)]
        apply: bool,

        /// Target directory to process
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    /// Show daemon status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("HAZELNUT_LOG").unwrap_or_else(|_| log_level.to_string()),
        ))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    match cli.command {
        None | Some(Commands::Ui) => {
            hazelnut::app::run(cli.config).await?;
        }
        Some(Commands::List) => {
            let config = hazelnut::Config::load(cli.config.as_deref())?;
            println!("Rules:");
            for (i, rule) in config.rules.iter().enumerate() {
                let status = if rule.enabled { "✓" } else { "✗" };
                println!("  {} [{}] {}", status, i + 1, rule.name);
            }
        }
        Some(Commands::Check {
            config: config_path,
        }) => {
            let path = config_path.or(cli.config);
            match hazelnut::Config::load(path.as_deref()) {
                Ok(config) => {
                    println!("✓ Config is valid");
                    println!("  {} watch paths", config.watches.len());
                    println!("  {} rules", config.rules.len());
                }
                Err(e) => {
                    eprintln!("✗ Config error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Run { apply, dir }) => {
            let config = hazelnut::Config::load(cli.config.as_deref())?;
            let engine = hazelnut::RuleEngine::new(config.rules);

            let dirs: Vec<_> = if let Some(d) = dir {
                vec![d]
            } else {
                config.watches.iter().map(|w| w.path.clone()).collect()
            };

            for dir in dirs {
                println!("Processing: {}", dir.display());
                let entries = std::fs::read_dir(&dir)?;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file()
                        && let Some(action) = engine.evaluate(&path)?
                    {
                        if apply {
                            println!("  Applying: {} -> {:?}", path.display(), action);
                            action.execute(&path)?;
                        } else {
                            println!("  [dry-run] {} -> {:?}", path.display(), action);
                        }
                    }
                }
            }
        }
        Some(Commands::Status) => {
            // TODO: Connect to daemon and get status
            println!("Daemon status: not implemented yet");
        }
    }

    Ok(())
}
