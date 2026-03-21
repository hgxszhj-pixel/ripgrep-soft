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

/// A saved favorite search configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FavoriteSearch {
    pub id: String,
    pub name: String,
    pub search_pattern: String,
    pub search_path: String,
    pub search_mode: SearchMode,
    pub use_regex: bool,
    pub use_glob: bool,
    pub case_sensitive: bool,
    pub size_filter: String,
    pub created_at: u64,
}

impl FavoriteSearch {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        search_pattern: String,
        search_path: String,
        search_mode: SearchMode,
        use_regex: bool,
        use_glob: bool,
        case_sensitive: bool,
        size_filter: String,
    ) -> Self {
        let id = format!("fav_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis());
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id,
            name,
            search_pattern,
            search_path,
            search_mode,
            use_regex,
            use_glob,
            case_sensitive,
            size_filter,
            created_at,
        }
    }
}

/// Collection of favorite searches
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Favorites {
    pub favorites: Vec<FavoriteSearch>,
}

impl Favorites {
    pub fn new() -> Self {
        Self { favorites: Vec::new() }
    }

    pub fn add(&mut self, favorite: FavoriteSearch) {
        self.favorites.push(favorite);
    }

    pub fn remove(&mut self, id: &str) {
        self.favorites.retain(|f| f.id != id);
    }

    pub fn get(&self, id: &str) -> Option<&FavoriteSearch> {
        self.favorites.iter().find(|f| f.id == id)
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

/// Pagination state for search results
#[derive(Clone, Debug, Default)]
pub struct PaginationState {
    pub current_page: usize,      // Current page number (1-based)
    pub items_per_page: usize,    // Number of items per page
    pub total_items: usize,       // Total number of items
}

impl PaginationState {
    /// Create a new pagination state
    pub fn new(items_per_page: usize) -> Self {
        Self {
            current_page: 1,
            items_per_page,
            total_items: 0,
        }
    }

    /// Calculate total number of pages
    pub fn total_pages(&self) -> usize {
        if self.total_items == 0 {
            return 1;
        }
        self.total_items.div_ceil(self.items_per_page)
    }

    /// Get offset for current page (0-based)
    pub fn offset(&self) -> usize {
        (self.current_page.saturating_sub(1)) * self.items_per_page
    }

    /// Get limit for current page
    pub fn limit(&self) -> usize {
        self.items_per_page
    }

    /// Update pagination with new total
    pub fn update_total(&mut self, total: usize) {
        self.total_items = total;
        // Adjust current page if it's now out of bounds
        if self.current_page > self.total_pages() {
            self.current_page = self.total_pages().max(1);
        }
    }

    /// Check if pagination is needed
    pub fn needs_pagination(&self) -> bool {
        self.total_items > self.items_per_page
    }

    /// Go to next page
    pub fn next_page(&mut self) {
        if self.current_page < self.total_pages() {
            self.current_page += 1;
        }
    }

    /// Go to previous page
    pub fn prev_page(&mut self) {
        if self.current_page > 1 {
            self.current_page -= 1;
        }
    }

    /// Go to first page
    pub fn first_page(&mut self) {
        self.current_page = 1;
    }

    /// Go to last page
    pub fn last_page(&mut self) {
        self.current_page = self.total_pages().max(1);
    }

    /// Go to specific page (clamped to valid range)
    pub fn go_to_page(&mut self, page: usize) {
        let target = page.clamp(1, self.total_pages().max(1));
        self.current_page = target;
    }

    /// Get visible page numbers for pagination UI
    /// Shows surrounding pages around current page
    pub fn get_visible_pages(&self, visible_count: usize) -> Vec<usize> {
        let total = self.total_pages().max(1);
        let current = self.current_page;

        if total <= visible_count {
            return (1..=total).collect();
        }

        let half = visible_count / 2;
        let mut start = current.saturating_sub(half);
        let end = (start + visible_count).min(total);

        // Adjust if we're near the end
        if end - start < visible_count {
            start = end.saturating_sub(visible_count);
        }

        // Always include first and last page
        let mut pages = Vec::new();

        // Add first page and ellipsis if needed
        if start > 1 {
            pages.push(1);
            if start > 2 {
                // Ellipsis
            }
        }

        // Add middle pages
        for p in start..=end {
            pages.push(p);
        }

        // Add ellipsis and last page if needed
        if end < total {
            if end < total - 1 {
                // Ellipsis
            }
            pages.push(total);
        }

        pages
    }
}

/// Available items per page options
pub const ITEMS_PER_PAGE_OPTIONS: &[usize] = &[50, 100, 200, 500];

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
