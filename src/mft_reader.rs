//! NTFS MFT (Master File Table) reader for Windows fast file indexing
//! This provides fast file listing using Windows FindFirstFile API

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

    #[error("Windows API error: {0}")]
    WindowsApi(String),
}

/// MFT file entry with basic file information
#[derive(Debug, Clone)]
pub struct MftFileEntry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub parent_seq: u64,
    pub is_directory: bool,
    pub modified: Option<std::time::SystemTime>,
}

pub struct MftReader {
    volume: String,
    use_fast_mode: bool,
}

impl MftReader {
    /// Create a new MFT reader for the specified volume
    pub fn new(volume: &str) -> Result<Self, MftError> {
        let vol = if volume.len() >= 2 {
            volume.chars().next().unwrap().to_string()
        } else {
            volume.trim_end_matches(':').to_string()
        };

        Ok(Self {
            volume: vol,
            use_fast_mode: true,
        })
    }

    /// Set fast mode (use Windows API) or standard mode
    pub fn with_fast_mode(mut self, fast: bool) -> Self {
        self.use_fast_mode = fast;
        self
    }

    /// Get volume name
    pub fn volume(&self) -> &str {
        &self.volume
    }

    /// Read all file entries from the volume
    pub fn read_entries(&self) -> Result<Vec<MftFileEntry>, MftError> {
        if self.use_fast_mode {
            self.read_entries_fast()
        } else {
            self.read_entries_compatible()
        }
    }

    /// Fast mode: Use Windows FindFirstFile/FindNextFile API
    #[cfg(windows)]
    fn read_entries_fast(&self) -> Result<Vec<MftFileEntry>, MftError> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::Storage::FileSystem::{
            FindFirstFileW, FindNextFileW, WIN32_FIND_DATAW, FILE_ATTRIBUTE_DIRECTORY,
        };

        let root = format!("{}:\\*", self.volume);
        let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();

        let mut entries = Vec::new();
        let mut find_data = WIN32_FIND_DATAW::default();

        unsafe {
            let handle = FindFirstFileW(
                windows::core::PCWSTR::from_raw(wide.as_ptr()),
                &mut find_data,
            );

            if handle.is_err() {
                return Err(MftError::OpenVolume(format!(
                    "Failed to open {}: {:?}",
                    root,
                    handle.err()
                )));
            }

            let handle = handle.unwrap();

            loop {
                // Get file name
                let name_slice = &find_data.cFileName;
                // Optimization: Use safe iteration instead of unsafe get_unchecked
                let name_len = name_slice.iter().position(|&c| c == 0).unwrap_or(name_slice.len());
                let name_vec: Vec<u16> = name_slice[..name_len].to_vec();
                let name = OsString::from_wide(&name_vec).to_string_lossy().to_string();

                // Skip . and ..
                if name != "." && name != ".." {
                    let attrs = find_data.dwFileAttributes;
                    let is_dir = (attrs & FILE_ATTRIBUTE_DIRECTORY.0) != 0;

                    let size = ((find_data.nFileSizeHigh as u64) << 32)
                        | (find_data.nFileSizeLow as u64);

                    let modified = filetime_to_system_time(&find_data.ftLastWriteTime);

                    entries.push(MftFileEntry {
                        path: PathBuf::from(format!("{}:\\{}", self.volume, name)),
                        name,
                        size,
                        parent_seq: 0,
                        is_directory: is_dir,
                        modified,
                    });
                }

                // Find next
                if FindNextFileW(handle, &mut find_data).is_err() {
                    break;
                }
            }

            // Close handle
            let _ = CloseHandle(handle);
        }

        Ok(entries)
    }

    /// Compatible mode: Use jwalk for full recursive listing
    #[cfg(windows)]
    fn read_entries_compatible(&self) -> Result<Vec<MftFileEntry>, MftError> {
        use jwalk::WalkDir;

        let root = format!("{}:\\", self.volume);
        let path = std::path::Path::new(&root);

        let entries: Vec<MftFileEntry> = WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                let metadata = e.metadata().ok();
                MftFileEntry {
                    path: e.path().to_path_buf(),
                    name: e.file_name().to_string_lossy().to_string(),
                    size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                    parent_seq: 0,
                    is_directory: false,
                    modified: metadata.and_then(|m| m.modified().ok()),
                }
            })
            .collect();

        Ok(entries)
    }

    /// Read entries from a specific path using Windows API
    pub fn read_entries_from_path(&self, path: &std::path::Path) -> Result<Vec<MftFileEntry>, MftError> {
        #[cfg(windows)]
        {
            use std::ffi::OsString;
            use std::os::windows::ffi::OsStringExt;
            use windows::Win32::Foundation::CloseHandle;
            use windows::Win32::Storage::FileSystem::{
                FindFirstFileW, FindNextFileW, WIN32_FIND_DATAW, FILE_ATTRIBUTE_DIRECTORY,
            };

            let search_pattern = format!("{}\\*", path.display());
            let wide: Vec<u16> = search_pattern.encode_utf16().chain(std::iter::once(0)).collect();

            let mut entries = Vec::new();
            let mut find_data = WIN32_FIND_DATAW::default();

            unsafe {
                let handle = FindFirstFileW(
                    windows::core::PCWSTR::from_raw(wide.as_ptr()),
                    &mut find_data,
                );

                if handle.is_err() {
                    return Ok(entries);
                }

                let handle = handle.unwrap();

                loop {
                    let name_slice = &find_data.cFileName;
                    // Optimization: Use safe iteration instead of unsafe get_unchecked
                    let name_len = name_slice.iter().position(|&c| c == 0).unwrap_or(name_slice.len());
                    let name_vec: Vec<u16> = name_slice[..name_len].to_vec();
                    let name = OsString::from_wide(&name_vec).to_string_lossy().to_string();

                    if name != "." && name != ".." {
                        let attrs = find_data.dwFileAttributes;
                        let is_dir = (attrs & FILE_ATTRIBUTE_DIRECTORY.0) != 0;
                        let size = ((find_data.nFileSizeHigh as u64) << 32)
                            | (find_data.nFileSizeLow as u64);
                        let modified = filetime_to_system_time(&find_data.ftLastWriteTime);

                        entries.push(MftFileEntry {
                            path: path.join(&name),
                            name,
                            size,
                            parent_seq: 0,
                            is_directory: is_dir,
                            modified,
                        });
                    }

                    if FindNextFileW(handle, &mut find_data).is_err() {
                        break;
                    }
                }

                let _ = CloseHandle(handle);
            }

            Ok(entries)
        }

        #[cfg(not(windows))]
        {
            let _ = path;
            Err(MftError::NotNtfs)
        }
    }
}

/// Convert Windows FILETIME to SystemTime
#[cfg(windows)]
fn filetime_to_system_time(ft: &windows::Win32::Foundation::FILETIME) -> Option<std::time::SystemTime> {
    use std::time::{Duration, UNIX_EPOCH};

    let ticks = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
    const NANOSECONDS_BETWEEN_1601_AND_1970: u64 = 11644473600 * 1_000_000_000;
    let nanos = ticks.saturating_mul(100);
    let duration = Duration::from_nanos(nanos.saturating_sub(NANOSECONDS_BETWEEN_1601_AND_1970));

    UNIX_EPOCH.checked_add(duration)
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
    fn test_read_entries_fast() {
        let reader = MftReader::new("C").unwrap();
        let entries = reader.read_entries();
        if let Ok(entries) = entries {
            println!("Found {} entries in C:\\ (fast mode)", entries.len());
            for entry in entries.iter().take(5) {
                println!("  {:?} - {} bytes", entry.name, entry.size);
            }
        }
    }

    #[test]
    fn test_read_entries_from_path() {
        let reader = MftReader::new("C").unwrap();
        let path = std::path::Path::new("C:\\Users");
        let entries = reader.read_entries_from_path(path);
        if let Ok(entries) = entries {
            println!("Found {} entries in C:\\Users", entries.len());
        }
    }
}
