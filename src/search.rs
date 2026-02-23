use crate::index::{FileEntry, FileIndex};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub struct SearchQuery {
    pub pattern: String,
    pub case_sensitive: bool,
    pub regex: bool,
}

impl SearchQuery {
    pub fn new(pattern: String) -> Self {
        Self {
            pattern,
            case_sensitive: false,
            regex: false,
        }
    }

    pub fn with_case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }

    pub fn with_regex(mut self, regex: bool) -> Self {
        self.regex = regex;
        self
    }
}

pub struct Searcher;

impl Searcher {
    pub fn search<'a>(query: &SearchQuery, index: &'a FileIndex) -> Vec<&'a FileEntry> {
        if query.pattern.is_empty() {
            return Vec::new();
        }

        if query.regex {
            Self::regex_search(query, index)
        } else {
            Self::fuzzy_search(query, index)
        }
    }

    fn fuzzy_search<'a>(query: &SearchQuery, index: &'a FileIndex) -> Vec<&'a FileEntry> {
        let pattern = if query.case_sensitive {
            query.pattern.clone()
        } else {
            query.pattern.to_lowercase()
        };

        index
            .entries()
            .iter()
            .filter(|entry| {
                let name = if query.case_sensitive {
                    entry.name.clone()
                } else {
                    entry.name.to_lowercase()
                };
                name.contains(&pattern)
            })
            .collect()
    }

    fn regex_search<'a>(query: &SearchQuery, index: &'a FileIndex) -> Vec<&'a FileEntry> {
        let case_flag = if query.case_sensitive { "" } else { "(?i)" };
        let pattern = format!("{}{}", case_flag, query.pattern);

        let re = match regex::Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        index
            .entries()
            .iter()
            .filter(|entry| re.is_match(&entry.name))
            .collect()
    }
}

// Content search types

pub struct ContentSearchQuery {
    pub pattern: String,
    pub case_sensitive: bool,
    pub regex: bool,
    pub max_context: usize,
}

impl ContentSearchQuery {
    pub fn new(pattern: String) -> Self {
        Self {
            pattern,
            case_sensitive: false,
            regex: false,
            max_context: 0,
        }
    }

    pub fn with_case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }

    pub fn with_regex(mut self, regex: bool) -> Self {
        self.regex = regex;
        self
    }

    pub fn with_max_context(mut self, max_context: usize) -> Self {
        self.max_context = max_context;
        self
    }
}

pub struct ContentMatch {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
    pub matched_text: String,
}

pub struct ContentSearcher;

impl ContentSearcher {
    pub fn search_files(
        query: &ContentSearchQuery,
        paths: &[PathBuf],
    ) -> Vec<ContentMatch> {
        if query.pattern.is_empty() {
            return Vec::new();
        }

        let mut matches = Vec::new();

        for path in paths {
            if path.is_dir() {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if entry_path.is_file() {
                            matches.extend(Self::search_file(query, &entry_path));
                        }
                    }
                }
            } else if path.is_file() {
                matches.extend(Self::search_file(query, path));
            }
        }

        matches
    }

    fn search_file(query: &ContentSearchQuery, path: &Path) -> Vec<ContentMatch> {
        // Skip binary files
        if Self::is_binary_file(path) {
            return Vec::new();
        }

        let file = match fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let reader = BufReader::new(file);
        let mut matches = Vec::new();

        let pattern = if query.case_sensitive {
            query.pattern.clone()
        } else {
            query.pattern.to_lowercase()
        };

        for (line_number, line_result) in reader.lines().enumerate() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => continue,
            };

            let line_matches = if query.regex {
                Self::regex_match(&query.pattern, &line, query.case_sensitive)
            } else {
                Self::substring_match(&pattern, &line, query.case_sensitive)
            };

            if let Some(matched_text) = line_matches {
                matches.push(ContentMatch {
                    file_path: path.to_path_buf(),
                    line_number: line_number + 1, // 1-indexed
                    line_content: line,
                    matched_text,
                });
            }
        }

        matches
    }

    fn is_binary_file(path: &Path) -> bool {
        let file = match fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return false,
        };

        let mut reader = BufReader::new(file);
        let mut buffer = [0u8; 8192];

        // Read up to 8KB to check for binary content
        match std::io::Read::read(&mut reader, &mut buffer) {
            Ok(n) => {
                // Check for null bytes in the first 8KB
                buffer[..n].iter().any(|&b| b == 0)
            }
            Err(_) => false,
        }
    }

    fn substring_match(pattern: &str, line: &str, case_sensitive: bool) -> Option<String> {
        let search_line = if case_sensitive {
            line.to_string()
        } else {
            line.to_lowercase()
        };

        if search_line.contains(pattern) {
            // Find the matched text in original line
            if case_sensitive {
                if let Some(start) = line.find(pattern) {
                    return Some(line[start..start + pattern.len()].to_string());
                }
            } else {
                // For case-insensitive, find position in lowercase and extract from original
                if let Some(start) = search_line.find(pattern) {
                    return Some(line[start..start + pattern.len()].to_string());
                }
            }
        }
        None
    }

    fn regex_match(pattern: &str, line: &str, case_sensitive: bool) -> Option<String> {
        let case_flag = if case_sensitive { "" } else { "(?i)" };
        let full_pattern = format!("{}{}", case_flag, pattern);

        let re = match regex::Regex::new(&full_pattern) {
            Ok(r) => r,
            Err(_) => return None,
        };

        if let Some(m) = re.find(line) {
            return Some(m.as_str().to_string());
        }

        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_entry(name: &str) -> FileEntry {
        FileEntry {
            path: PathBuf::from(format!("/test/{}", name)),
            name: name.to_string(),
            size: 100,
            modified: std::time::SystemTime::now(),
        }
    }

    #[test]
    fn test_search_query_new() {
        let query = SearchQuery::new("test".to_string());
        assert_eq!(query.pattern, "test");
        assert!(!query.case_sensitive);
        assert!(!query.regex);
    }

    #[test]
    fn test_fuzzy_search_case_insensitive() {
        let mut index = FileIndex::new();
        index.add_entry(create_test_entry("test.txt"));
        index.add_entry(create_test_entry("TEST.txt"));
        index.add_entry(create_test_entry("another.txt"));

        let query = SearchQuery::new("test".to_string());
        let results = Searcher::search(&query, &index);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_fuzzy_search_case_sensitive() {
        let mut index = FileIndex::new();
        index.add_entry(create_test_entry("test.txt"));
        index.add_entry(create_test_entry("TEST.txt"));

        let query = SearchQuery::new("test".to_string()).with_case_sensitive(true);
        let results = Searcher::search(&query, &index);

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_regex_search() {
        let mut index = FileIndex::new();
        index.add_entry(create_test_entry("test123.txt"));
        index.add_entry(create_test_entry("test456.txt"));
        index.add_entry(create_test_entry("other.txt"));

        let query = SearchQuery::new(r"test\d+\.txt".to_string()).with_regex(true);
        let results = Searcher::search(&query, &index);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_empty_pattern() {
        let index = FileIndex::new();
        let query = SearchQuery::new("".to_string());
        let results = Searcher::search(&query, &index);

        assert!(results.is_empty());
    }
}
