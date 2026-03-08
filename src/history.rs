//! Search history module - stores and manages search history

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::time::SystemTime;

/// A single search history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    /// The search pattern used
    pub pattern: String,
    /// Whether this was a content search
    pub is_content_search: bool,
    /// Whether regex was used
    pub use_regex: bool,
    /// Case sensitivity
    pub case_sensitive: bool,
    /// When this search was performed
    pub timestamp: SystemTime,
}

/// Search history manager
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchHistory {
    /// List of search history entries (most recent first)
    entries: Vec<SearchHistoryEntry>,
    /// Maximum number of entries to keep
    max_entries: usize,
}

impl Default for SearchHistory {
    fn default() -> Self {
        Self::new(100)
    }
}

impl SearchHistory {
    /// Create a new search history with the specified maximum number of entries
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    /// Add a new search to the history
    pub fn add_search(
        &mut self,
        pattern: String,
        is_content_search: bool,
        use_regex: bool,
        case_sensitive: bool,
    ) {
        // Check if this pattern already exists and remove it
        self.entries.retain(|e| e.pattern != pattern);

        // Add new entry at the beginning
        self.entries.insert(
            0,
            SearchHistoryEntry {
                pattern,
                is_content_search,
                use_regex,
                case_sensitive,
                timestamp: SystemTime::now(),
            },
        );

        // Trim to max size
        if self.entries.len() > self.max_entries {
            self.entries.truncate(self.max_entries);
        }
    }

    /// Get all history entries
    pub fn entries(&self) -> &[SearchHistoryEntry] {
        &self.entries
    }

    /// Get the most recent N entries
    pub fn recent(&self, n: usize) -> Vec<&SearchHistoryEntry> {
        self.entries.iter().take(n).collect()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Save history to a file
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(())
    }

    /// Load history from a file with validation
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut history: SearchHistory = serde_json::from_reader(reader)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Security: Validate and sanitize loaded history entries
        // Remove entries with invalid/empty patterns or excessively long patterns
        history.entries.retain(|entry| {
            !entry.pattern.is_empty() && entry.pattern.len() <= 10_000
        });

        Ok(history)
    }
}

/// Get the default search history file path
pub fn get_history_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("ripgrep-soft");
    path.push("history.json");
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_history_new() {
        let history = SearchHistory::new(10);
        assert!(history.entries().is_empty());
    }

    #[test]
    fn test_search_history_add() {
        let mut history = SearchHistory::new(10);
        history.add_search("test".to_string(), false, false, false);

        let entries = history.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pattern, "test");
    }

    #[test]
    fn test_search_history_deduplicate() {
        let mut history = SearchHistory::new(10);
        history.add_search("test".to_string(), false, false, false);
        history.add_search("other".to_string(), false, false, false);
        history.add_search("test".to_string(), false, false, false);

        let entries = history.entries();
        assert_eq!(entries.len(), 2);
        // Most recent should be first
        assert_eq!(entries[0].pattern, "test");
    }

    #[test]
    fn test_search_history_max_entries() {
        let mut history = SearchHistory::new(3);
        history.add_search("1".to_string(), false, false, false);
        history.add_search("2".to_string(), false, false, false);
        history.add_search("3".to_string(), false, false, false);
        history.add_search("4".to_string(), false, false, false);

        let entries = history.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].pattern, "4");
    }

    #[test]
    fn test_search_history_recent() {
        let mut history = SearchHistory::new(10);
        history.add_search("1".to_string(), false, false, false);
        history.add_search("2".to_string(), false, false, false);
        history.add_search("3".to_string(), false, false, false);

        let recent = history.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].pattern, "3");
        assert_eq!(recent[1].pattern, "2");
    }

    #[test]
    fn test_search_history_entries() {
        let mut history = SearchHistory::new(10);
        history.add_search("test1".to_string(), false, false, false);
        history.add_search("test2".to_string(), true, true, true);

        let entries = history.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].pattern, "test2");
        assert_eq!(entries[1].pattern, "test1");
    }

    #[test]
    fn test_search_history_clear() {
        let mut history = SearchHistory::new(10);
        history.add_search("test1".to_string(), false, false, false);
        history.add_search("test2".to_string(), false, false, false);

        history.clear();

        assert!(history.entries().is_empty());
    }

    #[test]
    fn test_search_history_is_empty() {
        let history = SearchHistory::new(10);
        assert!(history.entries().is_empty());

        let mut history = SearchHistory::new(10);
        history.add_search("test".to_string(), false, false, false);
        assert!(!history.entries().is_empty());
    }

    #[test]
    fn test_search_history_with_options() {
        let mut history = SearchHistory::new(10);
        history.add_search("pattern2".to_string(), false, false, false);
        history.add_search("pattern1".to_string(), true, true, true);

        let entries = history.entries();
        assert_eq!(entries[0].pattern, "pattern1");
        assert_eq!(entries[0].is_content_search, true);
        assert_eq!(entries[0].use_regex, true);
        assert_eq!(entries[0].case_sensitive, true);
    }
}
