//! GUI state types and enums

use serde::{Deserialize, Serialize};

/// Application theme variants
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum AppTheme {
    Light,
    Dark,
    #[default]
    Blue,
    Green,
    Purple,
}

impl AppTheme {
    /// Get theme display name
    pub fn display_name(&self) -> &'static str {
        match self {
            AppTheme::Light => "Light",
            AppTheme::Dark => "Dark",
            AppTheme::Blue => "Blue",
            AppTheme::Green => "Green",
            AppTheme::Purple => "Purple",
        }
    }
}

/// Search mode variants
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum SearchMode {
    #[default]
    Filename,
    Content,
}

impl SearchMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            SearchMode::Filename => "Filename",
            SearchMode::Content => "Content",
        }
    }
}

/// Application settings that persist across sessions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: AppTheme,
    pub font_size: f32,
    pub max_index_files: usize,
    pub max_filename_results: usize,
    pub max_content_results: usize,
    pub show_welcome: bool,
    pub last_search_path: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: AppTheme::Blue,
            font_size: 14.0,
            max_index_files: 100_000,
            max_filename_results: 500,
            max_content_results: 5_000,
            show_welcome: true,
            last_search_path: None,
        }
    }
}

/// Search options for content search
#[derive(Clone, Debug, Default)]
pub struct SearchOptions {
    pub use_regex: bool,
    pub use_glob: bool,
    pub case_sensitive: bool,
    pub size_filter: String,
}

/// UI state for tracking selections and preview
#[derive(Clone, Debug, Default)]
pub struct UiState {
    pub selected_index: Option<usize>,
    pub preview_path: Option<std::path::PathBuf>,
    pub show_welcome: bool,
    pub show_settings: bool,
}

/// Background task state
#[derive(Debug, Default)]
pub struct BackgroundTasks {
    pub is_indexing: bool,
    pub is_searching: bool,
    pub search_start_time: Option<std::time::Instant>,
    pub last_search_duration: Option<u64>,
}

/// File type categorization for icons
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FileCategory {
    Code,
    Document,
    Video,
    Audio,
    Image,
    Archive,
    Executable,
    Config,
    Other,
}

impl FileCategory {
    /// Detect file category from extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            // Code
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" | "c" | "cpp" | "h"
            | "hpp" | "cs" | "rb" | "php" | "swift" | "kt" | "scala" | "vue" | "svelte" => {
                FileCategory::Code
            }
            // Documents
            "pdf" | "doc" | "docx" | "txt" | "md" | "rtf" | "odt" | "xls" | "xlsx" | "ppt" | "pptx" => {
                FileCategory::Document
            }
            // Videos
            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg" | "3gp" => {
                FileCategory::Video
            }
            // Audio
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" => FileCategory::Audio,
            // Images
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" | "tif" => {
                FileCategory::Image
            }
            // Archives
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => FileCategory::Archive,
            // Executables
            "exe" | "msi" | "dll" | "sys" | "bat" | "cmd" | "ps1" | "sh" => FileCategory::Executable,
            // Config
            "json" | "xml" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" => FileCategory::Config,
            _ => FileCategory::Other,
        }
    }

    /// Get emoji icon for file category
    pub fn icon(&self) -> &'static str {
        match self {
            FileCategory::Code => "\u{1F4BB}",      // Computer
            FileCategory::Document => "\u{1F4C4}",  // Page
            FileCategory::Video => "\u{1F3AC}",      // Clapper
            FileCategory::Audio => "\u{1F3B5}",      // Music
            FileCategory::Image => "\u{1F5BC}",     // Picture
            FileCategory::Archive => "\u{1F4E6}",   // Package
            FileCategory::Executable => "\u{2699}",  // Gear
            FileCategory::Config => "\u{1F4C1}",    // Folder
            FileCategory::Other => "\u{1F4C4}",      // Page
        }
    }
}

/// Size filter helper
pub fn parse_size_filter(filter: &str) -> Option<(u64, u64)> {
    let filter = filter.trim();
    if filter.is_empty() {
        return None;
    }

    let parse_size = |s: &str| -> Option<u64> {
        let s = s.trim().to_lowercase();
        let multiplier = if s.ends_with('k') {
            1024
        } else if s.ends_with('m') {
            1024 * 1024
        } else if s.ends_with('g') {
            1024 * 1024 * 1024
        } else {
            1
        };
        let num = s.trim_end_matches(|c: char| !c.is_ascii_digit()).parse::<u64>().ok()?;
        Some(num * multiplier)
    };

    if filter.contains('-') {
        let parts: Vec<&str> = filter.split('-').collect();
        if parts.len() == 2 {
            let min = parse_size(parts[0]);
            let max = parse_size(parts[1]);
            match (min, max) {
                (Some(min), Some(max)) => Some((min, max)),
                _ => None,
            }
        } else {
            None
        }
    } else {
        // Single size - treat as max
        parse_size(filter).map(|s| (0, s))
    }
}
