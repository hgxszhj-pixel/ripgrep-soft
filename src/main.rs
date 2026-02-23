use clap::Parser;
use ripgrep_soft::{
    cli::Cli,
    cli::Commands,
    index::{FileIndex, FileEntry},
    search::{SearchQuery, Searcher, ContentSearcher, ContentSearchQuery},
    logging,
};
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
                let search_path = PathBuf::from(&path);
                
                // Build index from search path
                let mut index = FileIndex::new();
                if search_path.is_dir() {
                    if let Err(e) = index.walk_directory(&search_path) {
                        tracing::error!("Failed to walk directory: {}", e);
                        println!("Error: Failed to read directory: {}", e);
                        return Ok(());
                    }
                } else if search_path.is_file() {
                    if let Some(entry) = FileEntry::from_path(&search_path) {
                        index.add_entry(entry);
                    }
                }
                
                // Create search query
                let query = SearchQuery::new(pattern)
                    .with_regex(regex)
                    .with_case_sensitive(case_sensitive);
                
                // Perform search
                let results = Searcher::search(&query, &index);
                
                if results.is_empty() {
                    println!("No matches found.");
                } else {
                    println!("Found {} matches:", results.len());
                    for entry in results {
                        println!("  {}", entry.path.display());
                    }
                }
            } else {
                println!("Please specify either --pattern or --content for search");
            }
        }
        Commands::Index { path, rebuild } => {
            tracing::info!("Index command - path: {}, rebuild: {}", path, rebuild);
            
            let search_path = PathBuf::from(&path);
            
            if !search_path.exists() {
                println!("Error: Path does not exist: {}", path);
                return Ok(());
            }
            
            println!("Building index for: {}", path);
            
            let mut index = FileIndex::new();
            
            if search_path.is_dir() {
                match index.walk_directory(&search_path) {
                    Ok(()) => {
                        println!("Index built successfully!");
                        println!("Total files indexed: {}", index.len());
                    }
                    Err(e) => {
                        tracing::error!("Failed to build index: {}", e);
                        println!("Error: Failed to build index: {}", e);
                    }
                }
            } else {
                if let Some(entry) = FileEntry::from_path(&search_path) {
                    index.add_entry(entry);
                    println!("Index built successfully!");
                    println!("Total files indexed: {}", index.len());
                } else {
                    println!("Error: Could not read file: {}", path);
                }
            }
        }

    }

    tracing::info!("ripgrep-soft shutting down");
    Ok(())
}
