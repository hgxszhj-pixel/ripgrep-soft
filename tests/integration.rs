//! Integration tests for TurboSearch

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use turbo_search::index::FileIndex;
use turbo_search::search::{SearchQuery, Searcher, SizeFilter};

/// Create a unique temporary test directory
fn create_test_dir(name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("turbo_search_test_{}_{}", name, timestamp));
    fs::create_dir_all(&temp_dir).unwrap();
    temp_dir
}

/// Create test files in directory
fn create_test_files(dir: &PathBuf, files: &[&str]) {
    for file in files {
        let path = dir.join(file);
        File::create(&path).unwrap().write_all(b"test content").unwrap();
    }
}

#[test]
fn test_index_files() {
    let test_dir = create_test_dir("index");
    create_test_files(&test_dir, &["test.txt", "doc.pdf", "img.png", "data.csv"]);
    let mut index = FileIndex::new();

    let count = index.walk_directory_limited(&test_dir, 100).unwrap();

    assert!(count >= 4, "Should index at least 4 files, got {}", count);
    assert!(!index.is_empty(), "Index should not be empty");

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_filename_search_fuzzy() {
    let test_dir = create_test_dir("fuzzy");
    create_test_files(&test_dir, &["test.txt", "document.pdf", "testing.md"]);
    let mut index = FileIndex::new();
    index.walk_directory_limited(&test_dir, 100).unwrap();

    let query = SearchQuery::new("test".to_string());
    let results = Searcher::search(&query, &index);

    assert!(!results.is_empty(), "Should find files matching 'test'");

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_filename_search_glob() {
    let test_dir = create_test_dir("glob");
    create_test_files(&test_dir, &["file.txt", "readme.md", "data.csv", "script.js"]);
    let mut index = FileIndex::new();
    index.walk_directory_limited(&test_dir, 100).unwrap();

    let query = SearchQuery::new("*.txt".to_string()).with_glob(true);
    let results = Searcher::search(&query, &index);

    assert!(!results.is_empty(), "Should find .txt files");
    for entry in &results {
        assert!(entry.name.ends_with(".txt"), "All results should be .txt files");
    }

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_filename_search_regex() {
    let test_dir = create_test_dir("regex");
    create_test_files(&test_dir, &["test1.txt", "test2.pdf", "other.md"]);
    let mut index = FileIndex::new();
    index.walk_directory_limited(&test_dir, 100).unwrap();

    let query = SearchQuery::new(r"test\d+".to_string()).with_regex(true);
    let results = Searcher::search(&query, &index);

    assert!(!results.is_empty(), "Should find files matching regex");

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_search_with_size_filter() {
    let test_dir = create_test_dir("size");
    create_test_files(&test_dir, &["small.txt", "large.bin"]);

    // Make large file actually large
    let large_path = test_dir.join("large.bin");
    let mut file = File::create(&large_path).unwrap();
    for _ in 0..1000 {
        file.write_all(b"x").unwrap();
    }

    let mut index = FileIndex::new();
    index.walk_directory_limited(&test_dir, 100).unwrap();

    // Search with size filter (max 100 bytes)
    let query = SearchQuery::new("".to_string()).with_size_filter(SizeFilter::new(0, 100));
    let results = Searcher::search(&query, &index);

    for entry in &results {
        assert!(entry.size <= 100, "File should be under size limit: {}", entry.size);
    }

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_empty_directory() {
    let test_dir = create_test_dir("empty");
    fs::create_dir_all(&test_dir).unwrap();

    let mut index = FileIndex::new();
    let count = index.walk_directory_limited(&test_dir, 100).unwrap();

    assert_eq!(count, 0, "Empty directory should have 0 files");

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_nonexistent_directory() {
    let test_dir = std::path::PathBuf::from("/nonexistent/path/12345");
    let mut index = FileIndex::new();
    let count = index.walk_directory_limited(&test_dir, 100).unwrap();

    assert_eq!(count, 0, "Nonexistent directory should return 0");
}

#[test]
fn test_index_persistence() {
    let test_dir = create_test_dir("persist");
    create_test_files(&test_dir, &["a.txt", "b.txt", "c.txt"]);

    // Create and save index
    let mut index = FileIndex::with_root(&test_dir);
    index.walk_directory_limited(&test_dir, 100).unwrap();
    let original_count = index.len();

    let save_path = std::env::temp_dir().join("test_index_persist.json");
    index.save(&save_path).unwrap();

    // Load index
    let loaded = FileIndex::load(&save_path).unwrap();
    assert_eq!(loaded.len(), original_count, "Loaded index should have same count");

    // Cleanup
    let _ = fs::remove_file(&save_path);
    let _ = fs::remove_dir_all(&test_dir);
}
