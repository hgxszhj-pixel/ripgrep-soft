use crate::index::{FileEntry, FileIndex};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use rayon::prelude::*;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use dashmap::DashMap;

// Global regex cache for better performance - DashMap for lock-free concurrent reads
// Optimization: Bounded cache to prevent memory leaks
lazy_static::lazy_static! {
    static ref REGEX_CACHE: DashMap<String, Arc<regex::Regex>> = DashMap::new();
}

// Maximum number of cached regex patterns
const MAX_REGEX_CACHE_SIZE: usize = 1000;

/// File size filter - min and max size in bytes
/// Uses u64 with sentinel values for efficiency (no Option overhead)
#[derive(Clone, Copy, Debug, Default)]
pub struct SizeFilter {
    pub min_size: u64,      // 0 means no minimum
    pub max_size: u64,      // u64::MAX means no maximum
}

impl SizeFilter {
    /// Create a new SizeFilter with min and max bounds
    pub fn new(min: u64, max: u64) -> Self {
        Self { min_size: min, max_size: max }
    }

    /// Parse size string - handles formats like "1m", "<10k", "1m-10m"
    pub fn from_string(s: &str) -> Option<Self> {
        if s.is_empty() {
            return None;
        }

        let s_lower = s.trim().to_lowercase();
        let (prefix, rest) = if let Some(stripped) = s_lower.strip_prefix('<') {
            ('<', stripped.trim_start())
        } else if let Some(stripped) = s_lower.strip_prefix('>') {
            ('>', stripped.trim_start())
        } else {
            (' ', s_lower.as_str())
        };

        let (num_str, unit) = if let Some(idx) = rest.find(|c: char| !c.is_ascii_digit()) {
            (&rest[..idx], &rest[idx..])
        } else {
            (rest, "")
        };

        let num: u64 = num_str.parse().ok()?;
        let multiplier: u64 = match unit {
            "k" | "kb" => 1024,
            "m" | "mb" => 1024 * 1024,
            "g" | "gb" => 1024 * 1024 * 1024,
            "b" | "" => 1,
            _ => return None,
        };

        let size = num * multiplier;

        match prefix {
            '<' => Some(Self { min_size: 0, max_size: size }),
            '>' => Some(Self { min_size: size, max_size: u64::MAX }),
            _ => Some(Self { min_size: size, max_size: u64::MAX }),
        }
    }

    /// Create a range filter from "min-max" format
    pub fn from_range(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }

        if let Some((min_str, max_str)) = s.split_once('-') {
            let min_size = if min_str.is_empty() {
                0
            } else {
                Self::from_string(min_str)
                    .map(|f| f.min_size.max(f.max_size))
                    .unwrap_or(0)
            };
            let max_size = if max_str.is_empty() {
                u64::MAX
            } else {
                Self::from_string(max_str)
                    .map(|f| f.min_size.max(f.max_size))
                    .unwrap_or(u64::MAX)
            };
            Some(Self { min_size, max_size })
        } else {
            Self::from_string(s)
        }
    }

    /// Check if a file size matches this filter
    pub fn matches(&self, size: u64) -> bool {
        if self.min_size > 0 && size < self.min_size {
            return false;
        }
        if self.max_size < u64::MAX && size > self.max_size {
            return false;
        }
        true
    }
}

pub struct SearchQuery {
    pub pattern: String,
    pub case_sensitive: bool,
    pub regex: bool,
    pub glob: bool,
    pub offset: usize,
    pub limit: usize,
    pub size_filter: SizeFilter,
}

impl SearchQuery {
    pub fn new(pattern: String) -> Self {
        Self {
            pattern,
            case_sensitive: false,
            regex: false,
            glob: false,
            offset: 0,
            limit: 100,
            size_filter: SizeFilter::default(),
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

    pub fn with_glob(mut self, glob: bool) -> Self {
        self.glob = glob;
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_size_filter(mut self, size_filter: SizeFilter) -> Self {
        self.size_filter = size_filter;
        self
    }
}

pub struct Searcher;

impl Searcher {
    pub fn search<'a>(query: &SearchQuery, index: &'a FileIndex) -> Vec<&'a FileEntry> {
        if query.pattern.is_empty() {
            return Vec::new();
        }

        if query.glob {
            Self::glob_search(query, index)
        } else if query.regex {
            Self::regex_search(query, index)
        } else {
            Self::fuzzy_search(query, index)
        }
    }

    /// Glob pattern search (e.g., *.mp4, *.txt, document?.pdf)
    fn glob_search<'a>(query: &SearchQuery, index: &'a FileIndex) -> Vec<&'a FileEntry> {
        use glob::Pattern;

        let case_sensitive = query.case_sensitive;

        // Optimization: Pre-create pattern once outside the loop
        let pattern = if case_sensitive {
            match Pattern::new(&query.pattern) {
                Ok(p) => Some(p),
                Err(_) => return Vec::new(),
            }
        } else {
            let pattern_lower = query.pattern.to_lowercase();
            Pattern::new(&pattern_lower).ok()
        };

        let pattern = match pattern {
            Some(p) => p,
            None => return Vec::new(),
        };

        index.entries()
            .iter()
            .filter(|entry| {
                let name = &entry.name;
                if case_sensitive {
                    pattern.matches(name)
                } else {
                    // For case-insensitive, match against pre-created lowercase pattern
                    let name_lower = name.to_lowercase();
                    pattern.matches(&name_lower)
                }
            })
            .take(query.limit + query.offset)
            .skip(query.offset)
            .collect()
    }

    fn fuzzy_search<'a>(query: &SearchQuery, index: &'a FileIndex) -> Vec<&'a FileEntry> {
        let mut matcher = SkimMatcherV2::default();
        if query.case_sensitive {
            matcher = matcher.respect_case();
        } else {
            matcher = matcher.ignore_case();
        }
        let case_sensitive = query.case_sensitive;
        let pattern = &query.pattern;

        // Pre-compute lowercase pattern once (outside loop) - memory optimization
        let search_pattern = if case_sensitive {
            None
        } else {
            Some(pattern.to_lowercase())
        };

        // Collect matches with scores
        // Optimization: Use Cow<str> to avoid unnecessary allocations in case-insensitive search
        let pattern_str = pattern.as_str();
        let mut matches: Vec<(i64, &FileEntry)> = index
            .entries()
            .iter()
            .filter_map(|entry| {
                // For case-insensitive, lowercase name for matching
                let name_ref = if search_pattern.is_some() {
                    // Use to_lowercase() only once per entry, store in owned String
                    std::borrow::Cow::Owned(entry.name.to_lowercase())
                } else {
                    std::borrow::Cow::Borrowed(&entry.name)
                };

                let pattern_to_use = search_pattern.as_deref().unwrap_or(pattern_str);
                matcher.fuzzy_match(&name_ref, pattern_to_use)
                    .map(|score| (score, entry))
            })
            .collect();

        // Sort by score descending (best match first)
        matches.sort_by(|a, b| b.0.cmp(&a.0));

        // Return only the entries, sorted by score
        matches.into_iter().map(|(_, entry)| entry).collect()
    }

    fn regex_search<'a>(query: &SearchQuery, index: &'a FileIndex) -> Vec<&'a FileEntry> {
        // Use cached regex for better performance
        let re = match Self::get_cached_regex(&query.pattern, query.case_sensitive) {
            Some(r) => r,
            None => return Vec::new(),
        };

        index
            .entries()
            .iter()
            .filter(|entry| re.is_match(&entry.name))
            .collect()
    }

    /// Get or compile a regex pattern with caching using DashMap (lock-free concurrent reads)
    fn get_cached_regex(pattern: &str, case_sensitive: bool) -> Option<Arc<regex::Regex>> {
        let case_flag = if case_sensitive { "" } else { "(?i)" };
        // Use same format as original: case_flag + pattern (e.g., (?i)test\d+\.txt)
        let cache_key = format!("{}{}", case_flag, pattern);

        // Try to get from cache first - DashMap allows concurrent reads without locking
        if let Some(entry) = REGEX_CACHE.get(&cache_key) {
            return Some(entry.clone());
        }

        // Compile and cache if not found
        if let Ok(regex) = regex::Regex::new(&cache_key) {
            let arc_regex = Arc::new(regex);

            // Optimization: Enforce cache size limit to prevent memory leaks
            if REGEX_CACHE.len() >= MAX_REGEX_CACHE_SIZE {
                // Clear oldest entries (first 10% of cache)
                let keys_to_remove: Vec<_> = REGEX_CACHE.iter()
                    .take(MAX_REGEX_CACHE_SIZE / 10)
                    .map(|r| r.key().clone())
                    .collect();
                for key in keys_to_remove {
                    REGEX_CACHE.remove(&key);
                }
            }

            REGEX_CACHE.insert(cache_key, arc_regex.clone());
            return Some(arc_regex);
        }

        None
    }
}

pub struct ContentSearchQuery {
    pub pattern: String,
    pub case_sensitive: bool,
    pub regex: bool,
    pub max_context: usize,
    pub size_filter: SizeFilter,
}

impl ContentSearchQuery {
    pub fn new(pattern: String) -> Self {
        Self {
            pattern,
            case_sensitive: false,
            regex: false,
            max_context: 0,
            size_filter: SizeFilter::default(),
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

    pub fn with_size_filter(mut self, size_filter: SizeFilter) -> Self {
        self.size_filter = size_filter;
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
    pub fn search_files(query: &ContentSearchQuery, paths: &[PathBuf]) -> Vec<ContentMatch> {
        if query.pattern.is_empty() {
            return Vec::new();
        }

        // Optimization: Collect all file paths first for parallel processing
        let mut file_paths = Vec::new();

        for path in paths {
            if path.is_dir() {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if entry_path.is_file() {
                            file_paths.push(entry_path);
                        }
                    }
                }
            } else if path.is_file() {
                file_paths.push(path.clone());
            }
        }

        // Optimization: Use rayon for parallel file search when there are multiple files
        if file_paths.len() > 10 {
            // Parallel search for better performance with many files
            file_paths
                .par_iter()
                .flat_map(|path| Self::search_file(query, path))
                .collect()
        } else {
            // Sequential search for small number of files (less overhead)
            file_paths
                .iter()
                .flat_map(|path| Self::search_file(query, path))
                .collect()
        }
    }

    fn search_file(query: &ContentSearchQuery, path: &Path) -> Vec<ContentMatch> {
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
                    line_number: line_number + 1,
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

        match std::io::Read::read(&mut reader, &mut buffer) {
            Ok(n) => buffer[..n].contains(&0),
            Err(_) => false,
        }
    }

    fn substring_match(pattern: &str, line: &str, case_sensitive: bool) -> Option<String> {
        // Optimization: Avoid unnecessary allocations and duplicate find() calls
        if case_sensitive {
            // Direct case-sensitive search - no allocation needed
            line.find(pattern).map(|start| line[start..start + pattern.len()].to_string())
        } else {
            // Case-insensitive: pre-compute lowercase pattern once
            let pattern_lower = pattern.to_lowercase();
            let line_lower = line.to_lowercase();
            line_lower.find(&pattern_lower).map(|start| line[start..start + pattern.len()].to_string())
        }
    }

    fn regex_match(pattern: &str, line: &str, case_sensitive: bool) -> Option<String> {
        // Use cached regex for better performance
        let re = Searcher::get_cached_regex(pattern, case_sensitive)?;

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

    #[test]
    fn test_content_search_query_new() {
        let query = ContentSearchQuery::new("test".to_string());
        assert_eq!(query.pattern, "test");
        assert!(!query.case_sensitive);
        assert!(!query.regex);
        assert_eq!(query.max_context, 0);
    }

    #[test]
    fn test_content_search_query_builder() {
        let query = ContentSearchQuery::new("test".to_string())
            .with_case_sensitive(true)
            .with_regex(true)
            .with_max_context(3);

        assert!(query.case_sensitive);
        assert!(query.regex);
        assert_eq!(query.max_context, 3);
    }

    #[test]
    fn test_content_search_empty_pattern() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("ripgrep_test_content.txt");
        std::fs::write(&test_path, "test content here").unwrap();

        let query = ContentSearchQuery::new("".to_string());
        let results = ContentSearcher::search_files(&query, &[test_path.clone()]);

        assert!(results.is_empty());

        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_content_search_case_insensitive() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("ripgrep_test_case.txt");
        std::fs::write(
            &test_path,
            "Test CONTENT here\nAnother LINE\ntest content again",
        )
        .unwrap();

        let query = ContentSearchQuery::new("test".to_string());
        let results = ContentSearcher::search_files(&query, &[test_path.clone()]);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].line_number, 1);
        assert_eq!(results[1].line_number, 3);

        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_content_search_case_sensitive() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("ripgrep_test_sensitive.txt");
        std::fs::write(&test_path, "Test CONTENT\ntest content").unwrap();

        let query = ContentSearchQuery::new("Test".to_string()).with_case_sensitive(true);
        let results = ContentSearcher::search_files(&query, &[test_path.clone()]);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line_number, 1);

        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_content_search_regex() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("ripgrep_test_regex.txt");
        std::fs::write(&test_path, "test123 abc\ntest456 def\nother content").unwrap();

        let query = ContentSearchQuery::new(r"test\d+".to_string()).with_regex(true);
        let results = ContentSearcher::search_files(&query, &[test_path.clone()]);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].line_number, 1);
        assert_eq!(results[1].line_number, 2);

        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_content_search_line_number_tracking() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("ripgrep_test_lines.txt");
        std::fs::write(&test_path, "line 1\nline 2\nline 3 with match\nline 4").unwrap();

        let query = ContentSearchQuery::new("match".to_string());
        let results = ContentSearcher::search_files(&query, &[test_path.clone()]);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line_number, 3);
        assert!(results[0].line_content.contains("line 3 with match"));

        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_content_search_binary_detection() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("ripgrep_test_binary.bin");

        let binary_content: Vec<u8> = vec![0x00, 0x01, 0x02, b't', b'e', b's', b't'];
        std::fs::write(&test_path, binary_content).unwrap();

        let query = ContentSearchQuery::new("test".to_string());
        let results = ContentSearcher::search_files(&query, &[test_path.clone()]);

        assert!(results.is_empty());

        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_content_search_text_file() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("ripgrep_test_text.txt");
        std::fs::write(&test_path, "This is a text file\nWith some content").unwrap();

        let query = ContentSearchQuery::new("text".to_string());
        let results = ContentSearcher::search_files(&query, &[test_path.clone()]);

        assert_eq!(results.len(), 1);

        std::fs::remove_file(test_path).ok();
    }
}
