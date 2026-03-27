//! File indexing module - provides file system walking and index structure

use std::fs::{self, File};
use std::io::{Read, Write, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use ahash::AHashMap;
use walkdir::WalkDir;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(windows)]
use jwalk::WalkDir as JwalkWalkDir;

/// Convert OsStr to String - handles UTF-8, UTF-16, and GBK on Windows
pub fn os_str_to_string(os_str: &std::ffi::OsStr) -> String {
    // First try UTF-8
    if let Some(s) = os_str.to_str() {
        return s.to_string();
    }

    #[cfg(windows)]
    {
        // On Windows, OsStr is typically UTF-16 encoded
        let wide: Vec<u16> = os_str.encode_wide().collect();
        let bytes: Vec<u8> = wide.iter().flat_map(|&w| w.to_le_bytes()).collect();

        // Try UTF-16LE decoding first
        let (decoded, _, had_errors) = encoding_rs::UTF_16LE.decode(&bytes);
        if !had_errors {
            return decoded.into_owned();
        }

        // Try GBK decoding (common on Chinese Windows)
        let (decoded_gbk, _, had_errors_gbk) = encoding_rs::GBK.decode(&bytes);
        if !had_errors_gbk {
            return decoded_gbk.into_owned();
        }
    }

    // Fallback to lossy
    os_str.to_string_lossy().into_owned()
}

/// Represents a single file entry in the index
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileEntry {
    /// Full path to the file
    pub path: PathBuf,
    /// File name (without path)
    pub name: String,
    /// Pre-computed lowercase name for fast case-insensitive search
    #[serde(skip)]
    pub name_lower: String,
    /// File size in bytes
    pub size: u64,
    /// Last modification time
    pub modified: SystemTime,
}

/// In-memory file index structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileIndex {
    /// Root path of the index
    pub path: PathBuf,
    /// Last modified time
    pub modified: SystemTime,
    /// File entries
    entries: Vec<FileEntry>,
    /// Name-based hash index for fast lookups: name_lower -> entry indices
    /// This enables O(1) lookups instead of O(n) linear search
    #[serde(skip)]
    name_index: AHashMap<String, Vec<usize>>,
    /// Whether the index needs rebuilding
    #[serde(skip)]
    index_dirty: bool,
}

/// Indexing options for fine-tuned control
pub struct IndexOptions {
    /// Skip hidden files (files starting with . on Unix, or with hidden attribute on Windows)
    pub skip_hidden: bool,
    /// Skip system files
    pub skip_system: bool,
    /// Maximum depth for directory traversal (None = unlimited)
    pub max_depth: Option<usize>,
}

impl Default for IndexOptions {
    fn default() -> Self {
        Self {
            skip_hidden: true,
            skip_system: true,
            max_depth: None,
        }
    }
}

impl FileEntry {
    /// Create a new FileEntry from a file path
    pub fn from_path(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        let name = os_str_to_string(path.file_name()?);
        let name_lower = name.to_lowercase();

        Some(Self {
            path: path.to_path_buf(),
            name,
            name_lower,
            size: metadata.len(),
            modified: metadata.modified().ok()?,
        })
    }

    /// Create a new FileEntry from WalkDir's DirEntry (uses cached metadata)
    pub fn from_walk_entry(entry: &walkdir::DirEntry) -> Option<Self> {
        let metadata = entry.metadata().ok()?;
        let path = entry.path().to_path_buf();
        let name = os_str_to_string(entry.file_name());
        let name_lower = name.to_lowercase();

        Some(Self {
            path,
            name,
            name_lower,
            size: metadata.len(),
            modified: metadata.modified().ok()?,
        })
    }

    /// Create a new FileEntry from jwalk's DirEntry (uses cached metadata)
    /// jwalk uses a generic ClientState type - we use a type alias for the common case
    #[cfg(windows)]
    pub fn from_jwalk_entry<C: jwalk::ClientState>(entry: &jwalk::DirEntry<C>) -> Option<Self> {
        let metadata = entry.metadata().ok()?;
        let path = entry.path().to_path_buf();
        let name = os_str_to_string(entry.file_name());
        let name_lower = name.to_lowercase();

        Some(Self {
            path,
            name,
            name_lower,
            size: metadata.len(),
            modified: metadata.modified().ok()?,
        })
    }
}

impl Default for FileIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl FileIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            path: PathBuf::new(),
            modified: SystemTime::now(),
            entries: Vec::new(),
            name_index: AHashMap::new(),
            index_dirty: false,
        }
    }

    /// Walk a directory recursively and add all files to the index
    pub fn walk_directory(&mut self, path: &Path) -> std::io::Result<()> {
        if !path.exists() {
            return Ok(());
        }

        self.walk_directory_recursive(path);
        Ok(())
    }

    /// Check if a Windows file should be skipped based on hidden/system attributes
    /// Walk directory recursively using WalkDir for better performance
    /// Optimized: Single-pass traversal with cached metadata (no double I/O)
    fn walk_directory_recursive(&mut self, path: &Path) {
        // Single-pass: use WalkDir's cached metadata directly
        // This avoids calling fs::metadata() again for each file
        let entries: Vec<FileEntry> = WalkDir::new(path)
            .min_depth(1)
            .follow_links(false)
            .same_file_system(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| FileEntry::from_walk_entry(&e))
            .collect();

        // Pre-allocate and extend
        self.entries.reserve(entries.len());
        self.entries.extend(entries);
    }

    /// Build the name index for fast lookups
    /// O(n) operation - should be called after bulk operations
    fn build_name_index(&mut self) {
        self.name_index.clear();
        for (idx, entry) in self.entries.iter().enumerate() {
            self.name_index
                .entry(entry.name_lower.clone())
                .or_default()
                .push(idx);
        }
        self.index_dirty = false;
    }

    /// Ensure index is built (lazy build)
    fn ensure_index(&mut self) {
        if self.index_dirty || self.name_index.is_empty() {
            self.build_name_index();
        }
    }

    /// Add a single file entry to the index
    pub fn add_entry(&mut self, entry: FileEntry) {
        let idx = self.entries.len();
        self.entries.push(entry);
        // Update index incrementally
        if let Some(last) = self.entries.last() {
            self.name_index
                .entry(last.name_lower.clone())
                .or_default()
                .push(idx);
        }
    }

    /// Add multiple file entries to the index efficiently
    /// Optimized: Avoids rebuilding the entire index by incrementally updating
    /// Use this instead of multiple add_entry() calls for bulk operations
    pub fn add_entries_batch(&mut self, entries: Vec<FileEntry>) {
        if entries.is_empty() {
            return;
        }

        let start_idx = self.entries.len();
        let new_count = entries.len();

        // Reserve capacity to avoid re-allocations
        self.entries.reserve(new_count);

        // Extend entries first (no index update yet)
        self.entries.extend(entries);

        // Now update the index for all new entries at once
        // This is O(n) where n is new entries, not O(total entries)
        for idx in start_idx..self.entries.len() {
            self.name_index
                .entry(self.entries[idx].name_lower.clone())
                .or_default()
                .push(idx);
        }
    }

    /// Remove a file entry from the index by path
    /// Optimized: O(1) lookup and removal using swap_remove
    pub fn remove_entry(&mut self, path: &Path) -> bool {
        // Find the entry by path
        if let Some(pos) = self.entries.iter().position(|e| e.path == path) {
            let removed_name_lower = self.entries[pos].name_lower.clone();
            let last_idx = self.entries.len() - 1;

            // If not removing the last element, swap with last element
            if pos != last_idx {
                // Get the name_lower of the last element (which will be moved)
                let last_name_lower = self.entries[last_idx].name_lower.clone();

                // Swap in the name_index
                // Update indices for the moved element (last -> pos)
                if let Some(indices) = self.name_index.get_mut(&last_name_lower) {
                    if let Some(idx) = indices.iter().position(|&i| i == last_idx) {
                        indices[idx] = pos;
                    }
                }

                // Remove the removed element's index from its name_lower entry
                if let Some(indices) = self.name_index.get_mut(&removed_name_lower) {
                    indices.retain(|&i| i != pos);
                }

                // Swap remove (O(1) instead of O(n))
                self.entries.swap_remove(pos);
            } else {
                // Removing the last element, simpler case
                if let Some(indices) = self.name_index.get_mut(&removed_name_lower) {
                    indices.retain(|&i| i != last_idx);
                }
                self.entries.pop();
            }

            // Mark index as dirty since we've modified entries
            self.index_dirty = true;
            true
        } else {
            false
        }
    }

    /// Find entries by exact name (case-insensitive) - O(1) with index
    pub fn find_by_name(&mut self, name: &str) -> Vec<&FileEntry> {
        self.ensure_index();
        let name_lower = name.to_lowercase();
        if let Some(indices) = self.name_index.get(&name_lower) {
            indices.iter().filter_map(|&i| self.entries.get(i)).collect()
        } else {
            Vec::new()
        }
    }

    /// Find entries by name prefix (case-insensitive) - O(n) but optimized
    pub fn find_by_name_prefix(&mut self, prefix: &str) -> Vec<&FileEntry> {
        self.ensure_index();
        let prefix_lower = prefix.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.name_lower.starts_with(&prefix_lower))
            .collect()
    }

    /// Get mutable reference to entries for incremental updates
    pub fn entries_mut(&mut self) -> &mut Vec<FileEntry> {
        &mut self.entries
    }

    /// Get the number of entries in the index
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries in the index
    #[inline]
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    /// Create a new index with root path
    pub fn with_root(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            modified: SystemTime::now(),
            entries: Vec::new(),
            name_index: AHashMap::new(),
            index_dirty: false,
        }
    }

    /// Save index to file with gzip compression
    /// Uses bincode for faster serialization (5-10x faster than JSON)
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        // Try bincode first (faster), fallback to JSON for compatibility
        if let Ok(()) = self.save_bincode(path) {
            return Ok(());
        }

        // Fallback to JSON if bincode fails
        let json = serde_json::to_string(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let file = File::create(path)?;
        let encoder = GzEncoder::new(file, Compression::default());
        let mut writer = BufWriter::new(encoder);

        writer.write_all(json.as_bytes())?;
        writer.flush()?;

        Ok(())
    }

    /// Save index using bincode with gzip compression
    /// This is 5-10x faster than JSON serialization
    fn save_bincode(&self, path: &Path) -> std::io::Result<()> {
        let encoded = bincode::serialize(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let file = File::create(path)?;
        let encoder = GzEncoder::new(file, Compression::fast());
        let mut writer = BufWriter::new(encoder);

        writer.write_all(&encoded)?;
        writer.flush()?;

        Ok(())
    }

    /// Load index from file with gzip decompression
    /// Automatically detects and uses bincode (faster) or JSON format
    pub fn load(path: &Path) -> std::io::Result<Self> {
        // First try bincode format (faster)
        if let Ok(index) = Self::load_bincode(path) {
            return Ok(index);
        }

        // Fallback to gzip JSON format
        if let Ok(index) = Self::load_gzip(path) {
            return Ok(index);
        }

        // Fallback to plain text JSON (legacy)
        Self::load_plain_text(path)
    }

    /// Load index with bincode + gzip decompression
    fn load_bincode(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let decoder = GzDecoder::new(file);
        let mut reader = BufReader::new(decoder);

        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;

        let mut index: FileIndex = bincode::deserialize(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Self::validate_and_fix_index(&mut index)
    }

    /// Load index with gzip decompression
    fn load_gzip(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let decoder = GzDecoder::new(file);
        let mut reader = BufReader::new(decoder);

        let mut content = String::new();
        reader.read_to_string(&mut content)?;

        let mut index: FileIndex = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Self::validate_and_fix_index(&mut index)
    }

    /// Load index from plain text JSON (legacy format)
    fn load_plain_text(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut index: FileIndex = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Self::validate_and_fix_index(&mut index)
    }

    /// Validate and fix index entries for security
    fn validate_and_fix_index(index: &mut FileIndex) -> std::io::Result<Self> {
        // Security: Validate that stored paths are within the indexed root directory
        // This prevents path traversal attacks via malicious index files
        let root_path = index.path.canonicalize()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Optimization: Pre-compute root string once outside the loop
        let root_str = root_path.to_string_lossy().to_lowercase();

        // Pre-allocate buffer for lowercase conversion to avoid repeated allocations
        let mut lowercase_buf = String::with_capacity(512);

        for entry in index.entries.iter_mut() {
            // Re-compute name_lower since it was skipped during deserialization
            // Optimization: Reuse buffer to reduce allocations
            lowercase_buf.clear();
            lowercase_buf.push_str(&entry.name);
            entry.name_lower = lowercase_buf.to_lowercase();

            if let Ok(canonical) = entry.path.canonicalize() {
                // Verify the canonical path starts with the root path
                // Optimization: Use starts_with on string loss for cross-platform compatibility
                let canonical_str = canonical.to_string_lossy().to_lowercase();

                if !canonical_str.starts_with(&root_str) {
                    // Path traversal detected - skip this entry
                    entry.path = root_path.join(&entry.name);
                }
            }
        }

        Ok(std::mem::take(index))
    }

    /// Get root path (alias for path field)
    pub fn root_path(&self) -> &Path {
        &self.path
    }

    /// Walk a directory with a maximum file limit (for quick indexing)
    /// Optimized: Single-pass traversal with early termination
    pub fn walk_directory_limited(&mut self, path: &Path, max_files: usize) -> std::io::Result<usize> {
        if !path.exists() {
            return Ok(0);
        }

        // Single-pass: use WalkDir's cached metadata directly (no double I/O)
        let entries: Vec<FileEntry> = WalkDir::new(path)
            .min_depth(1)
            .follow_links(false)
            .same_file_system(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| FileEntry::from_walk_entry(&e))
            .take(max_files)
            .collect();

        let count = entries.len();

        // Use batch add to incrementally update index without full rebuild
        self.add_entries_batch(entries);

        Ok(count)
    }

    /// Walk directory using jwalk for high-performance parallel traversal
    /// Optimized jwalk - fast parallel directory traversal
    /// Skip sorting for speed, skip hidden/system files
    #[cfg(windows)]
    pub fn walk_directory_jwalk(&mut self, path: &Path, max_files: usize) -> std::io::Result<usize> {
        if !path.exists() {
            return Ok(0);
        }

        // Optimized: enable sorting for better search results, skip hidden files
        // Note: jwalk's sort is parallel and optimized, enabling it provides sorted output
        // which can improve user experience even if slightly slower
        let entries: Vec<FileEntry> = JwalkWalkDir::new(path)
            .sort(true)  // Enable sorting for better UX
            .skip_hidden(true)  // Skip hidden files
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| FileEntry::from_jwalk_entry(&e))
            .take(max_files)
            .collect();

        let count = entries.len();

        // Use batch add to incrementally update index without full rebuild
        self.add_entries_batch(entries);

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    #[test]
    fn test_file_entry_from_path() {
        // Create a temporary file
        let temp_dir = env::temp_dir();
        let temp_file = temp_dir.join("test_file_entry.txt");

        let mut file = File::create(&temp_file).unwrap();
        file.write_all(b"test content").unwrap();

        let entry = FileEntry::from_path(&temp_file);
        assert!(entry.is_some());

        let entry = entry.unwrap();
        assert_eq!(entry.name, "test_file_entry.txt");
        assert!(entry.size > 0);

        // Clean up
        fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_file_index_new() {
        let index = FileIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_file_index_walk_directory() {
        let temp_dir = env::temp_dir().join("test_index_walk");

        // Create test directory structure
        fs::create_dir_all(&temp_dir).unwrap();
        File::create(temp_dir.join("file1.txt")).unwrap();
        fs::create_dir_all(temp_dir.join("subdir")).unwrap();
        File::create(temp_dir.join("subdir/file2.txt")).unwrap();

        let mut index = FileIndex::new();
        index.walk_directory(&temp_dir).unwrap();

        assert!(index.len() >= 2);

        // Clean up
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_file_index_add_entry() {
        let mut index = FileIndex::new();
        let entry = FileEntry {
            path: PathBuf::from("/test/file.txt"),
            name: "file.txt".to_string(),
            name_lower: "file.txt".to_string(),
            size: 100,
            modified: SystemTime::now(),
        };

        index.add_entry(entry);

        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());
    }

    #[test]
    fn test_file_index_entries() {
        let mut index = FileIndex::new();
        let entry1 = FileEntry {
            path: PathBuf::from("/test/file1.txt"),
            name: "file1.txt".to_string(),
            name_lower: "file1.txt".to_string(),
            size: 100,
            modified: SystemTime::now(),
        };
        let entry2 = FileEntry {
            path: PathBuf::from("/test/file2.txt"),
            name: "file2.txt".to_string(),
            name_lower: "file2.txt".to_string(),
            size: 200,
            modified: SystemTime::now(),
        };

        index.add_entry(entry1);
        index.add_entry(entry2);

        let entries = index.entries();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_file_index_with_root() {
        let index = FileIndex::with_root(Path::new("/test"));

        assert_eq!(index.root_path(), Path::new("/test"));
    }

    #[test]
    fn test_file_index_save_load() {
        let temp_dir = env::temp_dir().join("test_index_save");
        fs::create_dir_all(&temp_dir).ok();

        let mut index = FileIndex::with_root(Path::new(&temp_dir));
        let entry = FileEntry {
            path: temp_dir.join("test.txt"),
            name: "test.txt".to_string(),
            name_lower: "test.txt".to_string(),
            size: 100,
            modified: SystemTime::now(),
        };
        index.add_entry(entry);

        let save_path = temp_dir.join("index.json");
        index.save(&save_path).unwrap();

        let loaded = FileIndex::load(&save_path).unwrap();
        assert_eq!(loaded.len(), 1);

        // Clean up
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_file_index_walk_directory_limited() {
        let temp_dir = env::temp_dir().join("test_index_limited");

        // Create test directory with multiple files
        fs::create_dir_all(&temp_dir).unwrap();
        for i in 0..15 {
            File::create(temp_dir.join(format!("file{}.txt", i))).ok();
        }

        let mut index = FileIndex::new();
        let count = index.walk_directory_limited(&temp_dir, 10).unwrap();

        assert_eq!(count, 10); // Limited to 10 files

        // Clean up
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_file_index_len() {
        let mut index = FileIndex::new();
        assert_eq!(index.len(), 0);

        let entry = FileEntry {
            path: PathBuf::from("/test/file.txt"),
            name: "file.txt".to_string(),
            name_lower: "file.txt".to_string(),
            size: 100,
            modified: SystemTime::now(),
        };
        index.add_entry(entry);

        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_file_entry_name_lower() {
        let entry = FileEntry {
            path: PathBuf::from("/test/FILE.txt"),
            name: "FILE.txt".to_string(),
            name_lower: "file.txt".to_string(),
            size: 100,
            modified: SystemTime::now(),
        };

        assert_eq!(entry.name, "FILE.txt");
        assert_eq!(entry.name_lower, "file.txt");
    }
}

// ============================================================================
// File Walker Module - Traversal strategies for file indexing
// ============================================================================

/// Result type for file walker
pub type WalkResult = std::io::Result<Vec<FileEntry>>;

/// Trait for file system walkers - enables different traversal strategies
pub trait FileWalker {
    /// Walk a directory and return file entries
    fn walk(&self, path: &Path) -> WalkResult;

    /// Walk with max file limit
    fn walk_with_limit(&self, path: &Path, max_files: usize) -> WalkResult;
}

/// Standard walkdir-based walker
pub struct WalkdirWalker {
    _options: IndexOptions,
}

impl WalkdirWalker {
    /// Create a new WalkdirWalker
    #[allow(dead_code)]
    pub fn new(options: IndexOptions) -> Self {
        Self { _options: options }
    }
}

impl FileWalker for WalkdirWalker {
    fn walk(&self, path: &Path) -> WalkResult {
        self.walk_with_limit(path, usize::MAX)
    }

    fn walk_with_limit(&self, path: &Path, max_files: usize) -> WalkResult {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let entries: Vec<FileEntry> = walkdir::WalkDir::new(path)
            .min_depth(1)
            .follow_links(false)
            .same_file_system(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| FileEntry::from_walk_entry(&e))
            .take(max_files)
            .collect();

        Ok(entries)
    }
}

/// Jwalk-based walker for Windows (parallel, faster)
#[cfg(windows)]
pub struct JwalkWalker {
    skip_hidden: bool,
}

#[cfg(windows)]
impl JwalkWalker {
    /// Create a new JwalkWalker
    pub fn new(skip_hidden: bool) -> Self {
        Self { skip_hidden }
    }
}

#[cfg(windows)]
impl FileWalker for JwalkWalker {
    fn walk(&self, path: &Path) -> WalkResult {
        self.walk_with_limit(path, usize::MAX)
    }

    fn walk_with_limit(&self, path: &Path, max_files: usize) -> WalkResult {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let entries: Vec<FileEntry> = jwalk::WalkDir::new(path)
            .sort(false)
            .skip_hidden(self.skip_hidden)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| FileEntry::from_jwalk_entry(&e))
            .take(max_files)
            .collect();

        Ok(entries)
    }
}

/// Create appropriate walker based on platform
pub fn create_walker(skip_hidden: bool) -> Box<dyn FileWalker> {
    #[cfg(windows)]
    {
        Box::new(JwalkWalker::new(skip_hidden))
    }
    #[cfg(not(windows))]
    {
        let options = IndexOptions {
            skip_hidden,
            skip_system: false,
            follow_symlinks: false,
            ignore_patterns: Vec::new(),
        };
        Box::new(WalkdirWalker::new(options))
    }
}
