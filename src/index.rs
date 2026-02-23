//! File indexing module - provides file system walking and index structure

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Represents a single file entry in the index
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Full path to the file
    pub path: PathBuf,
    /// File name (without path)
    pub name: String,
    /// File size in bytes
    pub size: u64,
    /// Last modification time
    pub modified: SystemTime,
}

impl FileEntry {
    /// Create a new FileEntry from a file path
    pub fn from_path(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        let name = path.file_name()?.to_string_lossy().to_string();

        Some(Self {
            path: path.to_path_buf(),
            name,
            size: metadata.len(),
            modified: metadata.modified().ok()?,
        })
    }
}

/// In-memory file index structure
#[derive(Debug, Default)]
pub struct FileIndex {
    entries: Vec<FileEntry>,
}

impl FileIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
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

    fn walk_directory_recursive(&mut self, path: &Path) {
        let read_dir = match fs::read_dir(path) {
            Ok(rd) => rd,
            Err(_) => return,
        };

        for entry in read_dir.flatten() {
            let entry_path = entry.path();

            if entry_path.is_dir() {
                self.walk_directory_recursive(&entry_path);
            } else if entry_path.is_file() {
                if let Some(file_entry) = FileEntry::from_path(&entry_path) {
                    self.entries.push(file_entry);
                }
            }
        }
    }

    /// Add a file entry to the index
    pub fn add_entry(&mut self, entry: FileEntry) {
        self.entries.push(entry);
    }

    /// Get the number of entries in the index
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries in the index
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
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
}
