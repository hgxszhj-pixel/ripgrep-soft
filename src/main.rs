use clap::Parser;
use ripgrep_soft::{cli::Cli, cli::Commands, logging, search::{ContentSearcher, ContentSearchQuery}};
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
            case_sensitive,
            context,
        } => {
            tracing::info!(
                "Search command - path: {}, pattern: {:?}, content: {:?}, regex: {}, case_sensitive: {}, context: {}",
                path,
                pattern,
                content,
                regex,
                case_sensitive,
                context
            );

            if let Some(content_pattern) = content {
                // Content search mode
                let query = ContentSearchQuery::new(content_pattern)
                    .with_regex(regex)
                    .with_case_sensitive(case_sensitive)
                    .with_max_context(context);

                let search_path = PathBuf::from(&path);
                let matches = ContentSearcher::search_files(&query, &[search_path]);

                if matches.is_empty() {
                    println!("No matches found.");
                } else {
                    for m in matches {
                        println!("{}:{}", m.file_path.display(), m.line_number);
                    }
                }
            } else if let Some(pattern) = pattern {
                // Filename search mode
                println!("Filename search for '{}' not yet implemented", pattern);
            } else {
                println!("Please specify either --pattern or --content for search");
            }
        }
        Commands::Index { path, rebuild } => {
            tracing::info!("Index command - path: {}, rebuild: {}", path, rebuild);
            println!("Index functionality not yet implemented");
        }

    }

    tracing::info!("ripgrep-soft shutting down");
    Ok(())
}
