use clap::Parser;
use ripgrep_soft::{cli::Cli, cli::Commands, logging};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let log_dir = PathBuf::from("logs");
    logging::init_logging(Some(log_dir))?;

    tracing::info!("ripgrep-soft starting up");

    match cli.command {
        Commands::Search {
            path,
            pattern,
            content,
            regex,
        } => {
            tracing::info!(
                "Search command - path: {}, pattern: {:?}, content: {:?}, regex: {}",
                path,
                pattern,
                content,
                regex
            );
            println!("Search functionality not yet implemented");
        }
        Commands::Index { path, rebuild } => {
            tracing::info!("Index command - path: {}, rebuild: {}", path, rebuild);
            println!("Index functionality not yet implemented");
        }

    }

    tracing::info!("ripgrep-soft shutting down");
    Ok(())
}
