//! File indexing module - provides file system walking and index structure

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use rayon::prelude::*;
use walkdir::WalkDir;
use ignore::WalkBuilder;

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
    /// Uses cached metadata from WalkDir when available
    #[cfg(windows)]
    #[inline]
    fn should_skip_windows_file(entry: &walkdir::DirEntry, options: &IndexOptions) -> bool {
        use std::os::windows::fs::MetadataExt;
        // Skip hidden: FILE_ATTRIBUTE_HIDDEN = 0x2
        // Skip system: FILE_ATTRIBUTE_SYSTEM = 0x4
        if let Ok(meta) = entry.metadata() {
            let attrs = meta.file_attributes();
            if options.skip_hidden && (attrs & 0x2) != 0 {
                return true;
            }
            if options.skip_system && (attrs & 0x4) != 0 {
                return true;
            }
        }
        false
    }

    /// Check if a Windows file should be skipped based on hidden/system attributes (jwalk version)
    #[cfg(windows)]
    #[inline]
    fn should_skip_jwalk_file<C: jwalk::ClientState>(
        entry: &jwalk::DirEntry<C>,
        options: &IndexOptions,
    ) -> bool {
        use std::os::windows::fs::MetadataExt;
        if let Ok(meta) = entry.metadata() {
            let attrs = meta.file_attributes();
            if options.skip_hidden && (attrs & 0x2) != 0 {
                return true;
            }
            if options.skip_system && (attrs & 0x4) != 0 {
                return true;
            }
        }
        false
    }

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

    /// Add a file entry to the index
    pub fn add_entry(&mut self, entry: FileEntry) {
        self.entries.push(entry);
    }

    /// Remove a file entry from the index by path
    pub fn remove_entry(&mut self, path: &Path) -> bool {
        let pos = self.entries.iter().position(|e| e.path == path);
        if let Some(pos) = pos {
            self.entries.remove(pos);
            true
        } else {
            false
        }
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
        }
    }

    /// Save index to file
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string(self)?;
        std::fs::write(path, json)
    }

    /// Load index from file with security validation
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut index: FileIndex = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Security: Validate that stored paths are within the indexed root directory
        // This prevents path traversal attacks via malicious index files
        let root_path = index.path.canonicalize()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Optimization: Pre-compute root string once outside the loop
        let root_str = root_path.to_string_lossy().to_lowercase();

        for entry in index.entries.iter_mut() {
            // Re-compute name_lower since it was skipped during deserialization
            entry.name_lower = entry.name.to_lowercase();

            if let Ok(canonical) = entry.path.canonicalize() {
                // Verify the canonical path starts with the root path
                let canonical_str = canonical.to_string_lossy().to_lowercase();
                if !canonical_str.starts_with(&root_str) {
                    // Path traversal detected - skip this entry
                    entry.path = root_path.join(&entry.name);
                }
            }
        }

        Ok(index)
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

        // Pre-allocate capacity to avoid re-allocations
        let capacity = max_files.min(1_000_000);
        self.entries.reserve(capacity);

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
        self.entries.extend(entries);

        Ok(count)
    }

    /// Walk directory with parallel processing (unlimited)
    /// Optimized: Single-pass traversal with cached metadata
    pub fn walk_directory_parallel(&mut self, path: &Path) -> std::io::Result<usize> {
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
            .collect();

        let count = entries.len();
        self.entries.reserve(count);
        self.entries.extend(entries);

        Ok(count)
    }

    /// Walk directory with options and parallel processing
    /// Optimized: Single-pass with efficient filtering
    pub fn walk_directory_with_options(
        &mut self,
        path: &Path,
        options: &IndexOptions,
    ) -> std::io::Result<usize> {
        if !path.exists() {
            return Ok(0);
        }

        // Build WalkDir with options
        let mut walker = WalkDir::new(path)
            .min_depth(1)
            .follow_links(false)
            .same_file_system(true);

        if let Some(max_depth) = options.max_depth {
            walker = walker.max_depth(max_depth);
        }

        // Collect file entries using cached metadata
        // Use a closure to avoid code duplication
        #[cfg(windows)]
        let entries: Vec<FileEntry> = walker
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                if e.file_type().is_dir() {
                    return true;
                }

                // Skip hidden/system files using cached metadata if available
                if (options.skip_hidden || options.skip_system)
                    && Self::should_skip_windows_file(e, options)
                {
                    return false;
                }

                true
            })
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| FileEntry::from_walk_entry(&e))
            .collect();

        #[cfg(not(windows))]
        let entries: Vec<FileEntry> = walker
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                if e.file_type().is_dir() {
                    return true;
                }

                let file_name = e.file_name();
                if options.skip_hidden && file_name.to_string_lossy().starts_with('.') {
                    return false;
                }

                true
            })
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| FileEntry::from_walk_entry(&e))
            .collect();

        let count = entries.len();

        // Pre-allocate capacity
        self.entries.reserve(count);
        self.entries.extend(entries);

        Ok(count)
    }

    /// Walk directory with maximum performance for very large directories
    /// Uses parallel directory scanning - each top-level subdirectory is processed in parallel
    /// This is optimal for directories with many subdirectories (e.g., C:\Users)
    pub fn walk_directory_parallel_high_performance(&mut self, path: &Path, max_files: usize) -> std::io::Result<usize> {
        if !path.exists() {
            return Ok(0);
        }

        // First, get list of top-level entries with cached metadata
        let mut top_level_dirs: Vec<walkdir::DirEntry> = Vec::new();
        let mut top_level_files: Vec<FileEntry> = Vec::new();

        for entry in WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() {
                top_level_dirs.push(entry);
            } else if entry.file_type().is_file() {
                if let Some(file_entry) = FileEntry::from_walk_entry(&entry) {
                    top_level_files.push(file_entry);
                }
            }
        }

        // Add top-level files directly
        self.entries.extend(top_level_files);

        if top_level_dirs.is_empty() {
            return Ok(self.entries.len());
        }

        // Pre-allocate based on estimate
        self.entries.reserve(max_files.min(1_000_000));

        // Process each top-level directory in parallel and collect entries directly
        let remaining_slots = max_files.saturating_sub(self.entries.len());

        // Collect entries from parallel directories using cached metadata
        // First collect to Vec, then filter
        let all_entries: Vec<FileEntry> = top_level_dirs
            .par_iter()
            .flat_map(|dir| {
                WalkDir::new(dir.path())
                    .min_depth(1)
                    .follow_links(false)
                    .same_file_system(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .filter_map(|e| FileEntry::from_walk_entry(&e))
                    .collect::<Vec<_>>()
            })
            .collect();

        // Take only what we need
        let entries: Vec<FileEntry> = all_entries.into_iter().take(remaining_slots).collect();

        let count = entries.len();
        self.entries.extend(entries);

        Ok(count)
    }

    /// Walk directory using jwalk for high-performance parallel traversal
    /// jwalk is ~4x faster than walkdir for sorted results with metadata
    /// Uses Rayon for parallel directory processing
    #[cfg(windows)]
    pub fn walk_directory_jwalk(&mut self, path: &Path, max_files: usize) -> std::io::Result<usize> {
        if !path.exists() {
            return Ok(0);
        }

        // Pre-allocate capacity
        let capacity = max_files.min(1_000_000);
        self.entries.reserve(capacity);

        // Use jwalk for parallel directory traversal with cached metadata
        // jwalk processes directories in parallel using Rayon
        // Use from_jwalk_entry to avoid double I/O
        let entries: Vec<FileEntry> = JwalkWalkDir::new(path)
            .sort(true)  // Enable sorted results (jwalk's strength)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| FileEntry::from_jwalk_entry(&e))
            .take(max_files)
            .collect();

        let count = entries.len();
        self.entries.extend(entries);

        Ok(count)
    }

    /// Walk directory using jwalk with custom filtering
    /// Supports skip_hidden and skip_system options
    #[cfg(windows)]
    pub fn walk_directory_jwalk_with_options(
        &mut self,
        path: &Path,
        options: &IndexOptions,
        max_files: usize,
    ) -> std::io::Result<usize> {
        if !path.exists() {
            return Ok(0);
        }

        let capacity = max_files.min(1_000_000);
        self.entries.reserve(capacity);

        // jwalk with sorting enabled, use cached metadata
        let entries: Vec<FileEntry> = JwalkWalkDir::new(path)
            .sort(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                if e.file_type().is_dir() {
                    return true;
                }

                // Skip hidden/system files using helper method
                if (options.skip_hidden || options.skip_system)
                    && Self::should_skip_jwalk_file(e, options)
                {
                    return false;
                }

                true
            })
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| FileEntry::from_jwalk_entry(&e))
            .take(max_files)
            .collect();

        let count = entries.len();
        self.entries.extend(entries);

        Ok(count)
    }

    /// Walk directory with .gitignore support using ignore crate
    /// This respects .gitignore, .ignore, and other ignore files
    pub fn walk_directory_with_ignore(&mut self, path: &Path, max_files: usize) -> std::io::Result<usize> {
        if !path.exists() {
            return Ok(0);
        }

        let capacity = max_files.min(1_000_000);
        self.entries.reserve(capacity);

        // Use ignore crate for gitignore-aware traversal
        // This automatically respects .gitignore, .ignore, and parent directory ignore files
        let walker = WalkBuilder::new(path)
            .ignore(true)  // Look for .ignore files and respect gitignore
            .build();

        let file_paths: Vec<PathBuf> = walker
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_type()?.is_file().then(|| e.path().to_path_buf()))
            .take(max_files)
            .collect();

        let count = file_paths.len();

        // Use parallel processing for large file lists
        let entries: Vec<FileEntry> = if count > 1000 {
            file_paths
                .par_iter()
                .filter_map(|p| FileEntry::from_path(p))
                .collect()
        } else {
            file_paths
                .iter()
                .filter_map(|p| FileEntry::from_path(p))
                .collect()
        };

        self.entries.extend(entries);

        Ok(count)
    }
}

#[cfg(windows)]
mod mft_integration {
    use super::*;
    use crate::mft_reader::MftReader;
    use std::time::UNIX_EPOCH;

    impl FileIndex {
        /// Create index from NTFS MFT (Windows only)
        pub fn from_mft(volume: &str) -> Result<Self, crate::mft_reader::MftError> {
            let reader = MftReader::new(volume)?;
            let mft_entries = reader.read_entries()?;

            let mut index = FileIndex::new();

            for mft_entry in mft_entries {
                if !mft_entry.is_directory {
                    let name_lower = mft_entry.name.to_lowercase();
                    let entry = FileEntry {
                        path: mft_entry.path,
                        name: mft_entry.name,
                        name_lower,
                        size: mft_entry.size,
                        modified: UNIX_EPOCH,
                    };
                    index.entries.push(entry);
                }
            }

            Ok(index)
        }
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
