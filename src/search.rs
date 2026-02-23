use crate::index::{FileEntry, FileIndex};

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
