//! CLI search functionality - provides command-line search and index operations

use crate::index::FileIndex;
use crate::search::{ContentSearchQuery, ContentSearcher, SearchQuery, Searcher};
use std::path::PathBuf;
use std::time::Instant;

/// Run CLI search command
pub fn run_search(
    path: Option<String>,
    pattern: Option<String>,
    content: Option<String>,
    regex: bool,
    glob: bool,
    case_sensitive: bool,
    _context: usize,
    limit: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine search path
    let search_path = match path {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir()?,
    };

    if !search_path.exists() {
        eprintln!("Error: Path '{}' does not exist", search_path.display());
        return Ok(());
    }

    // Determine search mode
    let is_content_search = content.is_some();
    let search_pattern = pattern.or(content).unwrap_or_default();

    if search_pattern.is_empty() {
        eprintln!("Error: No search pattern provided. Use --pattern or --content");
        return Ok(());
    }

    println!("Searching in: {}", search_path.display());
    println!("Pattern: {}", search_pattern);
    println!("Mode: {}", if is_content_search { "Content" } else { "Filename" });
    println!("---");

    // Build and run index
    let mut index = FileIndex::with_root(&search_path);
    let start = Instant::now();

    #[cfg(windows)]
    let count = index.walk_directory_jwalk(&search_path, 100_000).unwrap_or(0);

    #[cfg(not(windows))]
    let count = index.walk_directory_limited(&search_path, 100_000).unwrap_or(0);

    println!("Indexed {} files in {:?}\n", count, start.elapsed());

    if is_content_search {
        // Content search
        let query = ContentSearchQuery::new(search_pattern)
            .with_case_sensitive(case_sensitive)
            .with_regex(regex);

        let start = Instant::now();
        let results = ContentSearcher::search_files(&query, &[search_path.clone()]);

        println!("Found {} content matches in {:?}\n", results.len(), start.elapsed());

        // Display results
        for m in results.iter().take(limit) {
            println!("{}:{}: {}", m.file_path.display(), m.line_number, m.line_content);
        }
    } else {
        // Filename search
        let mut query = SearchQuery::new(search_pattern)
            .with_case_sensitive(case_sensitive)
            .with_limit(limit);

        if regex {
            query = query.with_regex(true);
        } else if glob {
            query = query.with_glob(true);
        }

        let start = Instant::now();
        let results = Searcher::search(&query, &index);

        println!("Found {} filename matches in {:?}\n", results.len(), start.elapsed());

        // Display results
        for entry in results.iter().take(limit) {
            println!("{}", entry.path.display());
        }
    }

    Ok(())
}

/// Run CLI index command
pub fn run_index(
    path: Option<String>,
    _rebuild: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine index path
    let index_path = match path {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir()?,
    };

    if !index_path.exists() {
        eprintln!("Error: Path '{}' does not exist", index_path.display());
        return Ok(());
    }

    println!("Building index for: {}", index_path.display());

    let mut index = FileIndex::with_root(&index_path);
    let start = Instant::now();

    #[cfg(windows)]
    let count = index.walk_directory_jwalk(&index_path, 1_000_000).unwrap_or(0);

    #[cfg(not(windows))]
    let count = index.walk_directory_limited(&index_path, 1_000_000).unwrap_or(0);

    println!("Indexed {} files in {:?}", count, start.elapsed());

    // Save index to file
    let index_path_str = index_path.to_string_lossy().replace(['\\', '/', ':'], "_");
    let index_file = PathBuf::from(format!("index_{}.json", index_path_str));
    index.save(&index_file)?;
    println!("Index saved to: {}", index_file.display());

    Ok(())
}
