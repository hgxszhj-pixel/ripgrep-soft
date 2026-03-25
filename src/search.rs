use crate::index::{FileEntry, FileIndex};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use rayon::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Seek};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use dashmap::DashMap;
use lru::LruCache;

// Global regex cache for better performance - DashMap for lock-free concurrent reads
// Optimization: Bounded cache to prevent memory leaks
lazy_static::lazy_static! {
    static ref REGEX_CACHE: DashMap<String, Arc<regex::Regex>> = DashMap::new();
}

// Maximum number of cached regex patterns
const MAX_REGEX_CACHE_SIZE: usize = 1000;

// Search result cache - LRU cache for search results
// Optimization: Use parking_lot Mutex (faster than std::sync::Mutex) for better performance
// Note: LruCache::get() requires &mut, so we use a fast mutex instead of RwLock
lazy_static::lazy_static! {
    static ref SEARCH_CACHE: parking_lot::Mutex<LruCache<u64, Arc<Vec<FileEntry>>>> =
        parking_lot::Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())); // Cache up to 100 queries
}

// Cache version to invalidate when index changes
static CACHE_VERSION: AtomicU64 = AtomicU64::new(0);

// Generate cache key from search parameters
fn generate_cache_key(
    pattern: &str,
    case_sensitive: bool,
    regex: bool,
    glob: bool,
    size_filter: &SizeFilter,
    index_version: u64,
    index_key: (usize, usize),
) -> u64 {
    let mut hasher = DefaultHasher::new();
    pattern.hash(&mut hasher);
    case_sensitive.hash(&mut hasher);
    regex.hash(&mut hasher);
    glob.hash(&mut hasher);
    size_filter.min_size.hash(&mut hasher);
    size_filter.max_size.hash(&mut hasher);
    index_version.hash(&mut hasher);
    // Include index-specific data (length + pointer) to avoid cache collisions
    index_key.0.hash(&mut hasher);
    index_key.1.hash(&mut hasher);
    hasher.finish()
}

/// Invalidate search cache - call when index changes
pub fn invalidate_search_cache() {
    CACHE_VERSION.fetch_add(1, Ordering::SeqCst);
    SEARCH_CACHE.lock().clear();
}

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
    /// Search the index with caching support
    /// Returns owned entries for caching purposes
    pub fn search(query: &SearchQuery, index: &FileIndex) -> Vec<FileEntry> {
        if query.pattern.is_empty() {
            return Vec::new();
        }

        // Include index-specific data in cache key to avoid collisions between different indexes
        // Use index pointer as unique identifier (stable within a search session)
        let index_ptr = index.entries().as_ptr() as usize;
        let index_key = (index.entries().len(), index_ptr);

        // Try to get from cache first
        let cache_key = generate_cache_key(
            &query.pattern,
            query.case_sensitive,
            query.regex,
            query.glob,
            &query.size_filter,
            CACHE_VERSION.load(Ordering::SeqCst),
            index_key,
        );

        // Use parking_lot Mutex (faster than std::sync::Mutex)
        {
            let mut cache = SEARCH_CACHE.lock();
            if let Some(cached_results) = cache.get(&cache_key) {
                // Return cloned Arc reference - cheap clone of Arc pointer
                return (**cached_results).clone();
            }
        }

        // Perform the actual search
        let results = if query.glob {
            Self::glob_search_owned(query, index)
        } else if query.regex {
            Self::regex_search_owned(query, index)
        } else {
            Self::fuzzy_search_owned(query, index)
        };

        // Store in cache with Arc for zero-copy sharing
        SEARCH_CACHE.lock().put(cache_key, Arc::new(results.clone()));

        results
    }

    /// Glob pattern search (e.g., *.mp4, *.txt, document?.pdf)
    fn glob_search_owned(query: &SearchQuery, index: &FileIndex) -> Vec<FileEntry> {
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
                if case_sensitive {
                    pattern.matches(&entry.name)
                } else {
                    pattern.matches(&entry.name_lower)
                }
            })
            .take(query.limit + query.offset)
            .skip(query.offset).cloned()
            .collect()
    }

    /// Fuzzy search - returns owned entries for caching
    fn fuzzy_search_owned(query: &SearchQuery, index: &FileIndex) -> Vec<FileEntry> {
        let mut matcher = SkimMatcherV2::default();
        if query.case_sensitive {
            matcher = matcher.respect_case();
        } else {
            matcher = matcher.ignore_case();
        }
        let case_sensitive = query.case_sensitive;
        let pattern = &query.pattern;
        let limit = query.limit + query.offset;

        // Early termination threshold: stop after finding enough good matches
        // This significantly improves performance for large indexes
        // For high-quality matches (score > 0), stop after finding 10x the needed results
        let high_quality_threshold = limit * 10;

        // Pre-compute lowercase pattern once (outside loop) - memory optimization
        let search_pattern = if case_sensitive {
            None
        } else {
            Some(pattern.to_lowercase())
        };

        let entries = index.entries();
        let entry_count = entries.len();

        // For very large indexes (>50000 entries), use parallel search with early termination
        // For medium indexes (5000-50000), use sequential with early termination
        // For small indexes (<5000), use sequential without early termination
        let mut matches: Vec<(i64, FileEntry)> = if entry_count > 50000 {
            // Parallel search for very large indexes
            // Use collect with limit to cap memory usage
            entries
                .par_iter()
                .filter_map(|entry| {
                    let name_ref = if search_pattern.is_some() {
                        std::borrow::Cow::Borrowed(&entry.name_lower)
                    } else {
                        std::borrow::Cow::Borrowed(&entry.name)
                    };
                    let pattern_to_use = search_pattern.as_deref().unwrap_or(pattern.as_str());
                    matcher.fuzzy_match(&name_ref, pattern_to_use)
                        .map(|score| (score, entry.clone()))
                })
                .collect()
        } else if entry_count > 5000 {
            // Sequential search with early termination for medium indexes
            // This is faster than parallel for medium datasets due to early exit
            let pattern_str = pattern.as_str();
            let mut found_count = 0;
            entries
                .iter()
                .filter_map(|entry| {
                    // Early termination: if we have enough high-quality matches, stop
                    if found_count >= high_quality_threshold {
                        return None;
                    }

                    let name_ref = if search_pattern.is_some() {
                        std::borrow::Cow::Borrowed(&entry.name_lower)
                    } else {
                        std::borrow::Cow::Borrowed(&entry.name)
                    };
                    let pattern_to_use = search_pattern.as_deref().unwrap_or(pattern_str);
                    if let Some(score) = matcher.fuzzy_match(&name_ref, pattern_to_use) {
                        // Count high-quality matches (score > 0 indicates good match)
                        if score > 0 {
                            found_count += 1;
                        }
                        Some((score, entry.clone()))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            // Sequential for small indexes without early termination overhead
            let pattern_str = pattern.as_str();
            entries
                .iter()
                .filter_map(|entry| {
                    let name_ref = if search_pattern.is_some() {
                        std::borrow::Cow::Borrowed(&entry.name_lower)
                    } else {
                        std::borrow::Cow::Borrowed(&entry.name)
                    };
                    let pattern_to_use = search_pattern.as_deref().unwrap_or(pattern_str);
                    matcher.fuzzy_match(&name_ref, pattern_to_use)
                        .map(|score| (score, entry.clone()))
                })
                .collect()
        };

        // Use partial sort for better performance - only need top N results
        if matches.len() > limit {
            // Use select_nth_unstable for O(n) partial sort
            let mid = matches.len() - limit;
            matches.select_nth_unstable_by(mid, |a, b| b.0.cmp(&a.0));

            // Only sort the top 'limit' elements
            let (top_matches, _) = matches.split_at_mut(mid);
            top_matches.sort_by(|a, b| b.0.cmp(&a.0));

            // Return top N with offset
            top_matches.iter()
                .skip(query.offset)
                .take(query.limit)
                .map(|(_, entry)| entry.clone())
                .collect()
        } else {
            // For small result sets, full sort is fine
            matches.sort_by(|a, b| b.0.cmp(&a.0));
            matches.into_iter()
                .skip(query.offset)
                .take(query.limit)
                .map(|(_, entry)| entry)
                .collect()
        }
    }

    /// Regex search - returns owned entries for caching
    fn regex_search_owned(query: &SearchQuery, index: &FileIndex) -> Vec<FileEntry> {
        // Use cached regex for better performance
        let re = match Self::get_cached_regex(&query.pattern, query.case_sensitive) {
            Some(r) => r,
            None => return Vec::new(),
        };

        index
            .entries()
            .iter()
            .filter(|entry| re.is_match(&entry.name)).cloned()
            .collect()
    }

    /// Get or compile a regex pattern with caching using DashMap (lock-free concurrent reads)
    fn get_cached_regex(pattern: &str, case_sensitive: bool) -> Option<Arc<regex::Regex>> {
        let case_flag = if case_sensitive { "" } else { "(?i)" };
        // Use same format as original: case_flag + pattern (e.g., (?i)test\d+\.txt)
        let cache_key = format!("{case_flag}{pattern}");

        // Try to get from cache first - DashMap allows concurrent reads without locking
        if let Some(entry) = REGEX_CACHE.get(&cache_key) {
            return Some(entry.clone());
        }

        // Compile and cache if not found
        if let Ok(regex) = regex::Regex::new(&cache_key) {
            let arc_regex = Arc::new(regex);

            // Optimization: Enforce cache size limit to prevent memory leaks
            // Only clean when cache is 90% full (reduce cleanup frequency)
            if REGEX_CACHE.len() >= MAX_REGEX_CACHE_SIZE * 9 / 10 {
                // Clear oldest 20% of cache entries (more efficient than 10%)
                let keys_to_remove: Vec<_> = REGEX_CACHE.iter()
                    .take(MAX_REGEX_CACHE_SIZE / 5)
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
        // Optimization: Skip files larger than 10MB to avoid memory issues
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.len() > 10 * 1024 * 1024 {
                return Vec::new(); // Skip large files
            }
        }

        // Optimization: Open file once, check binary, then seek back to start
        let mut file = match fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let mut buffer = [0u8; 8192];

        // Check if binary by reading first chunk
        match std::io::Read::read(&mut file, &mut buffer) {
            Ok(n) => {
                if buffer[..n].contains(&0) {
                    return Vec::new(); // Binary file - skip
                }
            }
            Err(_) => return Vec::new(),
        }

        // Seek back to start for line-by-line reading
        if file.seek(std::io::SeekFrom::Start(0)).is_err() {
            return Vec::new();
        }

        let reader = BufReader::new(file);
        let mut matches = Vec::new();

        // Default max matches per file to avoid excessive results
        let max_matches_per_file = 100;

        let pattern = if query.case_sensitive {
            query.pattern.clone()
        } else {
            query.pattern.to_lowercase()
        };

        for (line_number, line_result) in reader.lines().enumerate() {
            // Early termination: stop if we have enough matches
            if matches.len() >= max_matches_per_file {
                break;
            }

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

    fn substring_match(pattern: &str, line: &str, case_sensitive: bool) -> Option<String> {
        // Optimization: Accept pre-computed lowercase pattern to avoid repeated allocation
        if case_sensitive {
            // Direct case-sensitive search - no allocation needed
            line.find(pattern).map(|start| line[start..start + pattern.len()].to_string())
        } else {
            // Case-insensitive: pattern already lowercase, just convert line once
            let line_lower = line.to_lowercase();
            line_lower.find(pattern).map(|start| line[start..start + pattern.len()].to_string())
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
    use std::sync::atomic::{AtomicBool, Ordering};

    // Track if cache has been cleared for tests
    static TESTS_INITIALIZED: AtomicBool = AtomicBool::new(false);

    // Initialize test environment - clear cache once at start
    fn init_tests() {
        if !TESTS_INITIALIZED.load(Ordering::SeqCst) {
            invalidate_search_cache();
            TESTS_INITIALIZED.store(true, Ordering::SeqCst);
        }
    }

    fn create_test_entry(name: &str) -> FileEntry {
        FileEntry {
            path: PathBuf::from(format!("/test/{}", name)),
            name: name.to_string(),
            name_lower: name.to_lowercase(),
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
    fn test_glob_search_basic() {
        let mut index = FileIndex::new();
        index.add_entry(create_test_entry("document.pdf"));
        index.add_entry(create_test_entry("image.png"));
        index.add_entry(create_test_entry("photo.jpg"));
        index.add_entry(create_test_entry("data.txt"));

        // Test simple glob pattern
        let query = SearchQuery::new("*.txt".to_string()).with_glob(true);
        let results = Searcher::search(&query, &index);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "data.txt");
    }

    #[test]
    fn test_glob_search_multiple_extensions() {
        let mut index = FileIndex::new();
        index.add_entry(create_test_entry("file1.txt"));
        index.add_entry(create_test_entry("file2.pdf"));
        index.add_entry(create_test_entry("file3.txt"));
        index.add_entry(create_test_entry("file4.md"));

        // Test glob with multiple matches
        let query = SearchQuery::new("*.txt".to_string()).with_glob(true);
        let results = Searcher::search(&query, &index);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_glob_search_case_insensitive() {
        let mut index = FileIndex::new();
        index.add_entry(create_test_entry("Document.TXT"));
        index.add_entry(create_test_entry("FILE.txt"));
        index.add_entry(create_test_entry("file.pdf"));

        // Default: case-insensitive glob
        let query = SearchQuery::new("*.txt".to_string()).with_glob(true);
        let results = Searcher::search(&query, &index);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_glob_search_case_sensitive() {
        let mut index = FileIndex::new();
        index.add_entry(create_test_entry("Document.TXT"));
        index.add_entry(create_test_entry("FILE.txt"));
        index.add_entry(create_test_entry("file.txt"));

        // Case-sensitive glob
        let query = SearchQuery::new("*.TXT".to_string())
            .with_glob(true)
            .with_case_sensitive(true);
        let results = Searcher::search(&query, &index);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Document.TXT");
    }

    #[test]
    fn test_glob_search_question_mark() {
        let mut index = FileIndex::new();
        index.add_entry(create_test_entry("file1.txt"));
        index.add_entry(create_test_entry("file2.txt"));
        index.add_entry(create_test_entry("file10.txt"));

        // ? matches single character
        let query = SearchQuery::new("file?.txt".to_string()).with_glob(true);
        let results = Searcher::search(&query, &index);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_glob_search_invalid_pattern() {
        let mut index = FileIndex::new();
        index.add_entry(create_test_entry("test.txt"));

        // Invalid glob pattern should return empty results
        let query = SearchQuery::new("[invalid".to_string()).with_glob(true);
        let results = Searcher::search(&query, &index);

        assert!(results.is_empty());
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
