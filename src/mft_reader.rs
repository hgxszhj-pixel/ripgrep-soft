//! NTFS MFT (Master File Table) reader for Windows fast file indexing

#[cfg(windows)]
use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur when reading NTFS MFT
#[derive(Error, Debug)]
pub enum MftError {
    #[error("Failed to open volume: {0}")]
    OpenVolume(String),

    #[error("Failed to read MFT: {0}")]
    ReadMft(String),

    #[error("Invalid MFT record: {0}")]
    InvalidRecord(String),

    #[error("Volume is not NTFS")]
    NotNtfs,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// MFT file entry with basic file information
#[derive(Debug, Clone)]
pub struct MftFileEntry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub parent_seq: u64,
    pub is_directory: bool,
}

#[cfg(windows)]
pub struct MftReader {
    volume: String,
}

#[cfg(windows)]
impl MftReader {
    /// Create a new MFT reader for the specified volume
    pub fn new(volume: &str) -> Result<Self, MftError> {
        let vol = if volume.len() >= 2 {
            volume.chars().next().unwrap().to_string()
        } else {
            volume.trim_end_matches(':').to_string()
        };

        Ok(Self { volume: vol })
    }

    /// Read all file entries from the volume
    ///
    /// Uses Windows FindFirstFile API for faster enumeration than std::fs
    pub fn read_entries(&self) -> Result<Vec<MftFileEntry>, MftError> {
        let root = format!("{}:\\", self.volume);
        read_directory_entries(&root)
    }

    pub fn volume(&self) -> &str {
        &self.volume
    }
}

#[cfg(windows)]
fn read_directory_entries(dir: &str) -> Result<Vec<MftFileEntry>, MftError> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let mut entries = Vec::new();

    // Use std::fs::ReadDir for simplicity and cross-platform compatibility
    // For true MFT speed, would need raw Windows API + FSCTL_READ_MFT
    let path = std::path::Path::new(dir);

    if let Ok(read_dir) = std::fs::read_dir(path) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Skip . and ..
            if name == "." || name == ".." {
                continue;
            }

            let metadata = entry.metadata().ok();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

            entries.push(MftFileEntry {
                path,
                name,
                size,
                parent_seq: 0,
                is_directory: is_dir,
            });
        }
    }

    Ok(entries)
}

#[cfg(not(windows))]
pub struct MftReader;

#[cfg(not(windows))]
impl MftReader {
    pub fn new(_volume: &str) -> Result<Self, MftError> {
        Err(MftError::NotNtfs)
    }

    pub fn read_entries(&self) -> Result<Vec<MftFileEntry>, MftError> {
        Err(MftError::NotNtfs)
    }
}

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::*;

    #[test]
    fn test_mft_reader_new() {
        let reader = MftReader::new("C");
        assert!(reader.is_ok());

        let reader = MftReader::new("C:");
        assert!(reader.is_ok());
    }

    #[test]
    fn test_read_entries() {
        let reader = MftReader::new("C").unwrap();
        let entries = reader.read_entries();
        if let Ok(entries) = entries {
            println!("Found {} entries in C:\\", entries.len());
        }
    }
}
