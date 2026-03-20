//! GUI module for TurboSearch - Professional file search application

#![allow(dead_code)]

pub mod state;
pub mod ui_components;

use crate::index::{FileEntry, FileIndex};
use crate::search::{SearchQuery, Searcher, SizeFilter};
use crate::gui::state::{AppTheme, SearchMode, FileCategory, PaginationState, ITEMS_PER_PAGE_OPTIONS, FavoriteSearch, Favorites};
use eframe::egui::{self, FontDefinitions, FontData};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Instant;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Generate a safe filename from a path using a hash
fn path_to_safe_filename(path: &std::path::Path) -> String {
    let path_str = path.to_string_lossy();
    let mut hasher = DefaultHasher::new();
    path_str.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Get the app config directory path
fn get_app_config_dir() -> Option<std::path::PathBuf> {
    dirs::data_local_dir().map(|d| d.join("turbo-search"))
}

/// Save the last search path to config file
fn save_last_search_path(path: &std::path::Path) {
    if let Some(config_dir) = get_app_config_dir() {
        if let Err(e) = std::fs::create_dir_all(&config_dir) {
            eprintln!("Failed to create config directory: {}", e);
            return;
        }
        let config_file = config_dir.join("last_path.txt");
        if let Err(e) = std::fs::write(&config_file, path.to_string_lossy().as_bytes()) {
            eprintln!("Failed to save last path: {}", e);
        }
    }
}

/// Truncate path for display
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        let start = path.len() - max_len + 3;
        format!("...{}", &path[start..])
    }
}

/// Format file size in human-readable format
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Known media players to detect on Windows
const KNOWN_PLAYERS: &[(&str, &str)] = &[
    ("VLC", "vlc.exe"),
    ("PotPlayer", "PotPlayerMini64.exe"),
    ("PotPlayer (32-bit)", "PotPlayer.exe"),
    ("MPC-HC (64-bit)", "MPC-HC64.exe"),
    ("MPC-HC (32-bit)", "MPC-HC.exe"),
    ("MPV", "mpv.exe"),
    ("SMPlayer", "smplayer.exe"),
    ("KMPlayer", "KMPlayer.exe"),
    ("Windows Media Player", "wmplayer.exe"),
    ("GOM Player", "GOM.exe"),
];

/// Detect installed media players on the system
#[cfg(windows)]
fn detect_media_players() -> Vec<(String, String)> {
    use std::collections::HashSet;
    use std::env;

    let mut found_players: Vec<(String, String)> = Vec::new();
    let mut checked_paths: HashSet<String> = HashSet::new();

    // Search paths: PATH + common installation directories
    let mut search_dirs: Vec<std::path::PathBuf> = Vec::new();

    // Add PATH directories
    let path_var = env::var("PATH").unwrap_or_default();
    search_dirs.extend(env::split_paths(&path_var));

    // Add common installation directories
    if let Ok(program_files) = env::var("ProgramFiles") {
        search_dirs.push(program_files.into());
    }
    if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
        search_dirs.push(program_files_x86.into());
    }
    // Add common video player subdirectories
    let common_player_dirs = [
        "VideoLAN",
        "Potplayer",
        "MPC-HC",
        "K-Lite Codec Pack",
        "GOM",
        "KMPlayer",
        "smplayer",
        "MPV",
    ];
    for dir in &common_player_dirs {
        if let Ok(program_files) = env::var("ProgramFiles") {
            search_dirs.push(std::path::PathBuf::from(&program_files).join(dir));
        }
        if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
            search_dirs.push(std::path::PathBuf::from(&program_files_x86).join(dir));
        }
    }

    // Search for each player
    for (name, exe_name) in KNOWN_PLAYERS {
        for search_dir in &search_dirs {
            if !search_dir.exists() {
                continue;
            }
            // Check directly in search_dir
            let exe_path = search_dir.join(exe_name);
            if exe_path.exists() {
                let path_str = exe_path.to_string_lossy().to_string();
                if !checked_paths.contains(&path_str) {
                    checked_paths.insert(path_str.clone());
                    found_players.push((name.to_string(), path_str));
                }
            }
            // Also check subdirectories (for players with versioned folders)
            if let Ok(entries) = std::fs::read_dir(search_dir) {
                for entry in entries.flatten() {
                    let sub_path = entry.path();
                    if sub_path.is_dir() {
                        let exe_path = sub_path.join(exe_name);
                        if exe_path.exists() {
                            let path_str = exe_path.to_string_lossy().to_string();
                            if !checked_paths.contains(&path_str) {
                                checked_paths.insert(path_str.clone());
                                found_players.push((name.to_string(), path_str));
                            }
                        }
                    }
                }
            }
        }
    }

    // Add system default as first option
    if !found_players.is_empty() {
        found_players.insert(0, ("System Default".to_string(), "default".to_string()));
    }

    found_players
}

/// Detect installed media players (non-Windows fallback)
#[cfg(not(windows))]
fn detect_media_players() -> Vec<(String, String)> {
    vec![
        ("System Default".to_string(), "xdg-open".to_string()),
    ]
}

pub struct RipgrepApp {
    search_query: String,
    search_query_lower: String,
    displayed_results: Vec<FileEntry>,
    displayed_results_text: Vec<String>,
    total_results: usize,
    error_message: Option<String>,
    index: Arc<FileIndex>,
    search_path: PathBuf,
    search_path_text: String,
    use_regex: bool,
    use_glob: bool,
    case_sensitive: bool,
    size_filter: String,
    size_filter_custom: bool,
    search_mode: SearchMode,
    use_ripgrep: bool,
    ripgrep_available: bool,
    last_query: String,
    is_indexing: bool,
    is_searching: bool,
    search_start_time: Option<Instant>,
    last_search_duration: Option<u64>,
    progress_message: String,
    index_channel: Option<mpsc::Receiver<Option<FileIndex>>>,
    search_channel: Option<mpsc::Receiver<Vec<FileEntry>>>,
    streaming_matches: Vec<std::path::PathBuf>,
    font_size: f32,
    max_index_files: usize,
    max_filename_results: usize,
    max_content_results: usize,
    theme: AppTheme,
    show_welcome: bool,
    show_settings: bool,
    selected_index: Option<usize>,
    preview_content: String,
    highlighted_content: Option<String>,
    preview_loading: bool,
    preview_path: Option<std::path::PathBuf>,
    preview_channel: Option<mpsc::Receiver<String>>,
    available_players: Vec<(String, String)>,
    selected_player: Option<String>,
    // Pagination state
    pagination: PaginationState,
    // Favorites
    favorites: Favorites,
    show_favorites_dropdown: bool,
}

impl Default for RipgrepApp {
    fn default() -> Self {
        Self::new()
    }
}

impl RipgrepApp {
    pub fn new() -> Self {
        let mut app = Self {
            search_query: String::new(),
            search_query_lower: String::new(),
            displayed_results: Vec::new(),
            displayed_results_text: Vec::new(),
            total_results: 0,
            error_message: None,
            index: Arc::new(FileIndex::new()),
            search_path: PathBuf::new(),
            search_path_text: String::new(),
            use_regex: false,
            use_glob: false,
            case_sensitive: false,
            size_filter: String::new(),
            size_filter_custom: false,
            search_mode: SearchMode::Filename,
            use_ripgrep: true,
            ripgrep_available: Self::check_ripgrep_available(),
            last_query: String::new(),
            is_indexing: false,
            is_searching: false,
            search_start_time: None,
            last_search_duration: None,
            progress_message: String::new(),
            index_channel: None,
            search_channel: None,
            streaming_matches: Vec::new(),
            font_size: 14.0,
            max_index_files: 100000,
            max_filename_results: 500,
            max_content_results: 5000,
            theme: AppTheme::Light,
            show_welcome: true,
            show_settings: false,
            selected_index: None,
            preview_content: String::new(),
            highlighted_content: None,
            preview_loading: false,
            preview_path: None,
            preview_channel: None,
            available_players: Self::detect_media_players(),
            selected_player: Some("System Default".to_string()),
            pagination: PaginationState::new(100),
            favorites: Favorites::new(),
            show_favorites_dropdown: false,
        };

        // Load settings from config file
        app.load_settings();
        // Load favorites
        app.favorites = app.load_favorites();

        // Load favorites
        app.favorites = app.load_favorites();

        // Auto-select first detected player if available
        if app.selected_player.is_none() && !app.available_players.is_empty() {
            app.selected_player = Some(app.available_players[0].0.clone());
        }

        app.index_channel = app.load_saved_state();
        app
    }

    /// Re-detect media players
    fn detect_media_players() -> Vec<(String, String)> {
        crate::gui::detect_media_players()
    }

    fn check_ripgrep_available() -> bool {
        std::process::Command::new("rg")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get config directory path
    fn get_config_dir() -> Option<std::path::PathBuf> {
        dirs::data_local_dir().map(|d| d.join("turbo-search"))
    }

    /// Save settings to JSON file
    fn save_settings(&self) {
        if let Some(config_dir) = Self::get_config_dir() {
            if let Err(e) = std::fs::create_dir_all(&config_dir) {
                eprintln!("Failed to create config directory: {}", e);
                return;
            }

            let settings_file = config_dir.join("settings.json");
            let settings = serde_json::json!({
                "theme": self.theme.display_name(),
                "font_size": self.font_size,
                "max_index_files": self.max_index_files,
                "max_filename_results": self.max_filename_results,
                "max_content_results": self.max_content_results,
                "show_welcome": self.show_welcome,
            });

            if let Ok(json) = serde_json::to_string_pretty(&settings) {
                if let Err(e) = std::fs::write(&settings_file, json) {
                    eprintln!("Failed to save settings: {}", e);
                }
            }
        }
    }

    /// Load favorites from file
    fn load_favorites(&self) -> Favorites {
        if let Some(config_dir) = Self::get_config_dir() {
            let favorites_file = config_dir.join("favorites.json");
            if let Ok(json_str) = std::fs::read_to_string(&favorites_file) {
                if let Ok(favorites) = serde_json::from_str(&json_str) {
                    return favorites;
                }
            }
        }
        Favorites::new()
    }

    /// Save favorites to file
    fn save_favorites(&self) {
        if let Some(config_dir) = Self::get_config_dir() {
            if let Err(e) = std::fs::create_dir_all(&config_dir) {
                eprintln!("Failed to create config directory: {}", e);
                return;
            }
            let favorites_file = config_dir.join("favorites.json");
            if let Ok(json_str) = serde_json::to_string_pretty(&self.favorites) {
                if let Err(e) = std::fs::write(&favorites_file, json_str) {
                    eprintln!("Failed to save favorites: {}", e);
                }
            }
        }
    }

    /// Add current search to favorites
    fn add_to_favorites(&mut self, name: String) {
        let favorite = FavoriteSearch::new(
            name,
            self.search_query.clone(),
            self.search_path.to_string_lossy().to_string(),
            self.search_mode,
            self.use_regex,
            self.use_glob,
            self.case_sensitive,
            self.size_filter.clone(),
        );
        self.favorites.add(favorite);
        self.save_favorites();
    }

    /// Apply a favorite search
    fn apply_favorite(&mut self, favorite: &FavoriteSearch) {
        self.search_query = favorite.search_pattern.clone();
        self.search_path = PathBuf::from(&favorite.search_path);
        self.search_path_text = favorite.search_path.clone();
        self.search_mode = favorite.search_mode;
        self.use_regex = favorite.use_regex;
        self.use_glob = favorite.use_glob;
        self.case_sensitive = favorite.case_sensitive;
        self.size_filter = favorite.size_filter.clone();

        // Trigger search if we have valid inputs
        if !self.search_query.is_empty() && !self.search_path.as_os_str().is_empty() && !self.is_indexing {
            self.last_query.clear();
            self.perform_search();
        }
    }

    /// Remove a favorite
    fn remove_favorite(&mut self, id: &str) {
        self.favorites.remove(id);
        self.save_favorites();
    }

    /// Load settings from JSON file
    fn load_settings(&mut self) {
        if let Some(config_dir) = Self::get_config_dir() {
            let settings_file = config_dir.join("settings.json");

            if let Ok(json_str) = std::fs::read_to_string(&settings_file) {
                if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    // Load theme
                    if let Some(theme_str) = settings.get("theme").and_then(|v| v.as_str()) {
                        self.theme = match theme_str {
                            "Dark" => AppTheme::Dark,
                            "Blue" => AppTheme::Blue,
                            "Green" => AppTheme::Green,
                            "Purple" => AppTheme::Purple,
                            _ => AppTheme::Light,
                        };
                    }

                    // Load font size
                    if let Some(font_size) = settings.get("font_size").and_then(|v| v.as_f64()) {
                        self.font_size = font_size as f32;
                    }

                    // Load max files
                    if let Some(max_files) = settings.get("max_index_files").and_then(|v| v.as_u64()) {
                        self.max_index_files = max_files as usize;
                    }

                    // Load max results
                    if let Some(max_results) = settings.get("max_filename_results").and_then(|v| v.as_u64()) {
                        self.max_filename_results = max_results as usize;
                    }

                    if let Some(max_results) = settings.get("max_content_results").and_then(|v| v.as_u64()) {
                        self.max_content_results = max_results as usize;
                    }

                    // Load show welcome
                    if let Some(show_welcome) = settings.get("show_welcome").and_then(|v| v.as_bool()) {
                        self.show_welcome = show_welcome;
                    }
                }
            }
        }
    }

    fn load_saved_state(&mut self) -> Option<mpsc::Receiver<Option<FileIndex>>> {
        let config_dir = dirs::data_local_dir()?;
        let config_file = config_dir.join("turbo-search").join("last_path.txt");

        let saved_path = match std::fs::read_to_string(&config_file) {
            Ok(path_str) => std::path::PathBuf::from(path_str.trim()),
            Err(_) => {
                // Try old location (ripgrep-soft)
                let old_config_file = config_dir.join("ripgrep-soft").join("last_path.txt");
                match std::fs::read_to_string(&old_config_file) {
                    Ok(path_str) => std::path::PathBuf::from(path_str.trim()),
                    Err(_) => return None,
                }
            }
        };

        if !saved_path.exists() {
            self.search_path = saved_path;
            self.progress_message = "Click Browse to select a folder".to_string();
            return None;
        }

        self.search_path = saved_path.clone();
        self.search_path_text = saved_path.display().to_string();

        // Try to find index file in turbo-search directory first, then fallback to ripgrep-soft
        let data_dir = dirs::data_local_dir()?;
        let path_hash = path_to_safe_filename(&saved_path);

        // Check turbo-search first
        let app_dir = data_dir.join("turbo-search");
        let mut index_file = app_dir.join(format!("index_{}.gz", path_hash));

        // If not found, try old ripgrep-soft directory
        if !index_file.exists() {
            let old_app_dir = data_dir.join("ripgrep-soft");
            let old_index_file = old_app_dir.join(format!("index_{}.gz", path_hash));
            if old_index_file.exists() {
                // Copy to new location for future use
                if let Err(e) = std::fs::copy(&old_index_file, &index_file) {
                    eprintln!("Failed to migrate index: {}", e);
                    index_file = old_index_file;
                }
            }
        }

        if index_file.exists() {
            self.is_indexing = true;
            self.progress_message = "Loading saved index...".to_string();

            let index_file_path = index_file;

            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                let result = FileIndex::load(&index_file_path).ok();
                let _ = tx.send(result);
            });

            return Some(rx);
        }

        self.progress_message = "Click Browse to index folder".to_string();
        None
    }

    fn start_background_indexing(&mut self) -> mpsc::Receiver<Option<FileIndex>> {
        let (tx, rx) = mpsc::channel();
        let search_path = self.search_path.clone();
        let max_files = self.max_index_files;

        thread::spawn(move || {
            let mut index = FileIndex::with_root(&search_path);

            #[cfg(windows)]
            let count = index.walk_directory_jwalk(&search_path, max_files).unwrap_or(0);

            #[cfg(not(windows))]
            let count = index.walk_directory_limited(&search_path, max_files).unwrap_or(0);

            tracing::info!("Indexed {} files from {:?}", count, search_path);
            let _ = tx.send(Some(index));
        });

        self.is_indexing = true;
        self.progress_message = "Indexing...".to_string();
        rx
    }

    fn check_indexing_complete(&mut self) {
        if self.is_indexing {
            if let Some(rx) = self.index_channel.take() {
                match rx.try_recv() {
                    Ok(Some(index)) => {
                        let index_len = index.len();
                        self.index = Arc::new(index);
                        self.is_indexing = false;

                        if self.search_path != std::path::PathBuf::from(".") && index_len > 0 {
                            self.progress_message = format!(
                                "Loaded {} files - click Search",
                                index_len
                            );

                            let index_for_save = self.index.clone();
                            let search_path_for_save = self.search_path.clone();
                            thread::spawn(move || {
                                if let Some(data_dir) = dirs::data_local_dir() {
                                    let app_dir = data_dir.join("turbo-search");
                                    if let Err(e) = std::fs::create_dir_all(&app_dir) {
                                        eprintln!("Failed to create app directory: {}", e);
                                    } else {
                                        let path_hash = path_to_safe_filename(&search_path_for_save);
                                        let index_file = app_dir.join(format!("index_{}.gz", path_hash));
                                        if let Err(e) = index_for_save.save(&index_file) {
                                            eprintln!("Failed to save index: {}", e);
                                        }
                                    }
                                }
                            });
                        }
                    }
                    Ok(None) => {
                        self.is_indexing = false;
                        self.progress_message = "Indexing failed".to_string();
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        self.index_channel = Some(rx);
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.is_indexing = false;
                    }
                }
            }
        }
    }

    /// Check for completed search results
    /// Update displayed results text based on current page
    fn update_displayed_results_text(&mut self) {
        self.displayed_results_text.clear();

        let offset = self.pagination.offset();
        let limit = self.pagination.limit();

        // Only compute display strings for current page
        for entry in self.displayed_results.iter().skip(offset).take(limit) {
            let file_name = entry
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            let path_str = entry.path.to_string_lossy().to_string();
            self.displayed_results_text.push(format!("{} | {}", file_name, path_str));
        }
    }

    /// Handle pagination changes
    fn handle_page_change(&mut self) {
        // Clear selection when changing page
        self.selected_index = None;
        self.preview_content.clear();

        // Update displayed results for new page
        self.update_displayed_results_text();
    }

    fn check_search_complete(&mut self) {
        if self.is_searching {
            if let Some(rx) = self.search_channel.take() {
                match rx.try_recv() {
                    Ok(results) => {
                        // Search completed
                        self.displayed_results = results;
                        self.total_results = self.displayed_results.len();
                        self.is_searching = false;

                        // Update pagination state
                        self.pagination.update_total(self.total_results);
                        self.pagination.current_page = 1;

                        // Calculate duration
                        if let Some(start) = self.search_start_time.take() {
                            self.last_search_duration = Some(start.elapsed().as_millis() as u64);
                        }

                        // Update progress message
                        self.progress_message = format!(
                            "Found {} results ({} ms)",
                            self.total_results,
                            self.last_search_duration.unwrap_or(0)
                        );

                        // Pre-compute display strings for current page only
                        self.update_displayed_results_text();
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // Still searching, put channel back
                        self.search_channel = Some(rx);
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        // Thread finished unexpectedly
                        self.is_searching = false;
                        self.progress_message = "Search failed".to_string();
                    }
                }
            }
        }
    }

    fn perform_search(&mut self) {
        if self.search_query.is_empty() || self.search_path.as_os_str().is_empty() {
            return;
        }

        if self.last_query == self.search_query && !self.displayed_results.is_empty() {
            return;
        }

        self.last_query = self.search_query.clone();
        self.search_query_lower = self.search_query.to_lowercase();
        self.is_searching = true;
        self.search_start_time = Some(Instant::now());
        self.displayed_results.clear();
        self.displayed_results_text.clear();
        self.selected_index = None;
        self.preview_content.clear();
        self.highlighted_content = None;

        let query = self.search_query.clone();
        let index = self.index.clone();
        let use_regex = self.use_regex;
        let use_glob = self.use_glob;
        let case_sensitive = self.case_sensitive;
        let size_filter = self.size_filter.clone();
        let max_results = if self.search_mode == SearchMode::Filename {
            self.max_filename_results
        } else {
            self.max_content_results
        };
        let search_mode = self.search_mode;

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            if search_mode == SearchMode::Filename {
                // Build search query
                let mut search_query = SearchQuery::new(query.clone());

                if use_regex {
                    search_query.regex = true;
                } else if use_glob {
                    search_query.glob = true;
                }

                // Set limit for search
                search_query.limit = max_results;

                if case_sensitive {
                    search_query.case_sensitive = true;
                }

                // Size filter
                if !size_filter.is_empty() {
                    if let Some(filter) = SizeFilter::from_string(&size_filter) {
                        search_query.size_filter = filter;
                    }
                }

                // Search the index
                let search_results = Searcher::search(&search_query, &index);

                // Limit results
                let results: Vec<FileEntry> = search_results
                    .into_iter()
                    .map(|e| (*e).clone())
                    .take(max_results)
                    .collect();

                let _ = tx.send(results);
            } else {
                // Content search - simplified
                let search_results: Vec<FileEntry> = index
                    .entries()
                    .iter()
                    .filter_map(|e| {
                        if let Ok(content) = std::fs::read_to_string(&e.path) {
                            if content.to_lowercase().contains(&query.to_lowercase()) {
                                return Some((*e).clone());
                            }
                        }
                        None
                    })
                    .take(max_results)
                    .collect();

                let _ = tx.send(search_results);
            }
        });

        self.search_channel = Some(rx);
    }

    fn reset_search(&mut self) {
        self.search_query.clear();
        self.search_query_lower.clear();
        self.displayed_results.clear();
        self.displayed_results_text.clear();
        self.total_results = 0;
        self.selected_index = None;
        self.preview_content.clear();
        self.highlighted_content = None;
        self.last_query.clear();
        self.search_channel = None;
        // Reset pagination
        self.pagination.current_page = 1;
        self.pagination.total_items = 0;
    }

    /// Render pagination controls
    fn render_pagination(&mut self, ui: &mut egui::Ui) {
        let total_pages = self.pagination.total_pages();
        let current_page = self.pagination.current_page;

        ui.horizontal(|ui| {
            // First page button
            if ui.button("<< First").clicked() {
                self.pagination.first_page();
                self.handle_page_change();
            }

            // Previous page button
            if ui.button("< Prev").clicked() {
                self.pagination.prev_page();
                self.handle_page_change();
            }

            // Page info
            ui.label(egui::RichText::new(format!("Page {}/{} ", current_page, total_pages)).strong());

            // Next page button
            if ui.button("Next >").clicked() {
                self.pagination.next_page();
                self.handle_page_change();
            }

            // Last page button
            if ui.button("Last >>").clicked() {
                self.pagination.last_page();
                self.handle_page_change();
            }

            ui.separator();

            // Items per page selector
            ui.label("Per page:");
            let mut current_per_page = self.pagination.items_per_page;
            egui::ComboBox::from_id_salt("items_per_page")
                .selected_text(format!("{}", current_per_page))
                .show_ui(ui, |ui| {
                    for &per_page in ITEMS_PER_PAGE_OPTIONS {
                        if ui.selectable_value(&mut current_per_page, per_page, format!("{}", per_page)).clicked() {
                            self.pagination.items_per_page = per_page;
                            self.pagination.current_page = 1; // Reset to first page
                            self.handle_page_change();
                        }
                    }
                });
        });

        // Quick page jump
        ui.horizontal(|ui| {
            ui.label("Go to page:");
            let mut page_input = format!("{}", current_page);
            if ui.add(egui::TextEdit::singleline(&mut page_input).desired_width(60.0)).lost_focus() {
                if let Ok(page) = page_input.parse::<usize>() {
                    self.pagination.go_to_page(page);
                    self.handle_page_change();
                }
            }
            ui.label(format!("(1-{})", total_pages));
        });
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        let visuals = match self.theme {
            AppTheme::Light => {
                let mut v = egui::Visuals::light();
                v.override_text_color = Some(egui::Color32::from_black_alpha(200));
                v
            }
            AppTheme::Dark => {
                let mut v = egui::Visuals::dark();
                v.override_text_color = Some(egui::Color32::from_white_alpha(230));
                v
            }
            AppTheme::Blue => {
                let mut v = egui::Visuals::light();
                v.panel_fill = egui::Color32::from_rgb(235, 245, 255);
                v.window_fill = egui::Color32::from_rgb(240, 248, 255);
                v.override_text_color = Some(egui::Color32::from_rgb(0, 50, 100));
                v.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(200, 220, 240);
                v.widgets.hovered.bg_fill = egui::Color32::from_rgb(150, 190, 230);
                v.widgets.active.bg_fill = egui::Color32::from_rgb(100, 150, 220);
                v.selection.bg_fill = egui::Color32::from_rgb(80, 130, 200);
                v
            }
            AppTheme::Green => {
                let mut v = egui::Visuals::light();
                v.panel_fill = egui::Color32::from_rgb(235, 250, 235);
                v.window_fill = egui::Color32::from_rgb(240, 252, 240);
                v.override_text_color = Some(egui::Color32::from_rgb(0, 80, 30));
                v.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(200, 235, 200);
                v.widgets.hovered.bg_fill = egui::Color32::from_rgb(150, 220, 160);
                v.widgets.active.bg_fill = egui::Color32::from_rgb(100, 180, 120);
                v.selection.bg_fill = egui::Color32::from_rgb(80, 160, 100);
                v
            }
            AppTheme::Purple => {
                let mut v = egui::Visuals::dark();
                v.panel_fill = egui::Color32::from_rgb(40, 42, 54);
                v.window_fill = egui::Color32::from_rgb(48, 50, 66);
                v.override_text_color = Some(egui::Color32::from_rgb(248, 248, 242));
                v.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(68, 71, 90);
                v.widgets.hovered.bg_fill = egui::Color32::from_rgb(98, 114, 164);
                v.widgets.active.bg_fill = egui::Color32::from_rgb(139, 92, 246);
                v.selection.bg_fill = egui::Color32::from_rgb(98, 114, 164);
                v
            }
        };
        ctx.set_visuals(visuals);
    }

    fn apply_font_size(&self, ctx: &egui::Context) {
        ctx.style_mut(|style| {
            style.text_styles.iter_mut().for_each(|(_id, style)| {
                style.size = self.font_size;
            });
        });
    }

    fn read_file_content_sync(path: &std::path::Path) -> String {
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Binary file extensions to skip
        let binary_exts = ["exe", "dll", "zip", "rar", "7z", "tar", "gz", "jpg", "jpeg", "png", "gif", "bmp", "ico", "webp", "mp3", "mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx"];

        if binary_exts.contains(&extension.as_str()) {
            return format!("[Preview not available for {} files]", extension);
        }

        // Try to read as UTF-8
        match std::fs::read_to_string(path) {
            Ok(content) => {
                // Limit preview size
                if content.len() > 100000 {
                    format!("{}\n\n[... truncated ...]", &content[..100000])
                } else {
                    content
                }
            }
            Err(_) => {
                // Try with lossy conversion
                match std::fs::read(path) {
                    Ok(bytes) => {
                        // Try to detect encoding and convert
                        let (content, _, _) = encoding_rs::GBK.decode(&bytes);
                        if content.len() > 100000 {
                            format!("{}\n\n[... truncated ...]", &content[..100000])
                        } else {
                            content.to_string()
                        }
                    }
                    Err(e) => format!("[Cannot read file: {}]", e)
                }
            }
        }
    }

    /// Check if file is a video
    fn is_video_file(path: &std::path::Path) -> bool {
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(extension.as_str(), "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg" | "3gp")
    }

    /// Check if file is audio
    fn is_audio_file(path: &std::path::Path) -> bool {
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(extension.as_str(), "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a")
    }

    /// Open file with system default player
    fn open_with_player(path: &std::path::Path, player_path: &str) {
        #[cfg(windows)]
        {
            use std::process::Command;
            let path_str = path.to_string_lossy().to_string();

            if player_path == "default" || player_path.is_empty() {
                // Use system default
                let _ = Command::new("cmd")
                    .args(["/c", "start", "", &path_str])
                    .spawn();
            } else {
                // Use specified player
                let _ = Command::new(player_path)
                    .arg(&path_str)
                    .spawn();
            }
        }
        #[cfg(not(windows))]
        {
            use std::process::Command;
            if player_path == "default" || player_path.is_empty() {
                let _ = Command::new("xdg-open").arg(path).spawn();
            } else {
                let _ = Command::new(player_path).arg(path).spawn();
            }
        }
    }

    fn open_with_default_player(path: &std::path::Path) {
        #[cfg(windows)]
        {
            use std::process::Command;
            let path_str = path.to_string_lossy();
            // Use cmd /c start to open with default application
            let _ = Command::new("cmd")
                .args(["/c", "start", "", &path_str])
                .spawn();
        }
        #[cfg(not(windows))]
        {
            use std::process::Command;
            let _ = Command::new("xdg-open")
                .arg(path)
                .spawn();
        }
    }
}

impl eframe::App for RipgrepApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Only request repaint when background work is happening
        if self.is_indexing || self.is_searching || self.preview_loading {
            ctx.request_repaint();
        }

        // Check for completed background indexing/loading
        self.check_indexing_complete();

        // Check for completed search results
        self.check_search_complete();

        // Apply theme and font
        self.apply_theme(ctx);
        self.apply_font_size(ctx);

        // Show welcome dialog on first run
        if self.show_welcome {
            egui::Window::new("Welcome to TurboSearch")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add(egui::Label::new(egui::RichText::new("\u{1F50D}").size(48.0)).selectable(false));
                        ui.heading(egui::RichText::new("TurboSearch").strong().color(egui::Color32::from_rgb(0, 120, 212)));
                        ui.label(egui::RichText::new("Fast File & Content Search").small().color(egui::Color32::GRAY));
                        ui.add_space(10.0);
                        ui.separator();
                    });

                    ui.label(egui::RichText::new("Keyboard Shortcuts").strong());
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("[Ctrl+O]").background_color(ui.style().visuals.faint_bg_color));
                        ui.label("Open folder");
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("[Enter]").background_color(ui.style().visuals.faint_bg_color));
                        ui.label("Start search");
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("[Escape]").background_color(ui.style().visuals.faint_bg_color));
                        ui.label("Clear search");
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Version").small().color(egui::Color32::GRAY));
                        ui.label("1.0.0");
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.show_welcome, "Show on startup");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new(egui::RichText::new("Get Started \u{2192}").color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(0, 120, 212))).clicked() {
                                self.show_welcome = false;
                            }
                        });
                    });
                });
        }

        // Settings panel
        if self.show_settings {
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.heading("Settings");
                    ui.separator();

                    // Theme selection
                    ui.label(egui::RichText::new("Appearance").strong());
                    ui.horizontal(|ui| {
                        ui.label("Theme:");
                        egui::ComboBox::from_id_salt("settings_theme")
                            .selected_text(self.theme.display_name())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.theme, AppTheme::Light, "Light");
                                ui.selectable_value(&mut self.theme, AppTheme::Dark, "Dark");
                                ui.selectable_value(&mut self.theme, AppTheme::Blue, "Blue");
                                ui.selectable_value(&mut self.theme, AppTheme::Green, "Green");
                                ui.selectable_value(&mut self.theme, AppTheme::Purple, "Purple");
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Font Size:");
                        ui.add(egui::Slider::new(&mut self.font_size, 10.0..=24.0).text(""));
                        ui.label(format!("{}px", self.font_size as i32));
                    });

                    ui.separator();

                    // Search settings
                    ui.label(egui::RichText::new("Search").strong());
                    ui.horizontal(|ui| {
                        ui.label("Max index files:");
                        ui.add(egui::Slider::new(&mut self.max_index_files, 10000..=1000000).text(""));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Max filename results:");
                        ui.add(egui::Slider::new(&mut self.max_filename_results, 50..=5000).text(""));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Max content results:");
                        ui.add(egui::Slider::new(&mut self.max_content_results, 100..=10000).text(""));
                    });

                    ui.separator();

                    // Startup settings
                    ui.label(egui::RichText::new("Startup").strong());
                    ui.checkbox(&mut self.show_welcome, "Show welcome dialog on startup");

                    ui.separator();

                    // Media Player settings
                    ui.label(egui::RichText::new("Media Player").strong());
                    ui.horizontal(|ui| {
                        ui.label("Video Player:");
                        egui::ComboBox::from_id_salt("player_selector")
                            .selected_text(self.selected_player.as_deref().unwrap_or("None"))
                            .show_ui(ui, |ui| {
                                for (name, _path) in &self.available_players {
                                    ui.selectable_value(
                                        &mut self.selected_player,
                                        Some(name.clone()),
                                        name,
                                    );
                                }
                            });
                    });

                    // Show detected players count
                    if !self.available_players.is_empty() {
                        ui.label(egui::RichText::new(format!(
                            "{} player(s) detected",
                            self.available_players.len()
                        )).small().color(egui::Color32::GRAY));
                    } else {
                        ui.label(egui::RichText::new("No players detected")
                            .small()
                            .color(egui::Color32::RED));
                    }

                    // Re-scan button
                    if ui.button("Re-scan Players").clicked() {
                        self.available_players = Self::detect_media_players();
                        if self.selected_player.is_none() && !self.available_players.is_empty() {
                            self.selected_player = Some(self.available_players[0].0.clone());
                        }
                    }

                    ui.separator();

                    // Save and Close buttons
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            self.save_settings();
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                self.show_settings = false;
                            }
                        });
                    });
                });
        }

        // Top toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Animated lightning bolt
                let time = ctx.input(|i| i.time);
                let frame = ((time * 6.0) as usize) % 4;
                let bolt_frames = ["\u{26A1}", "\u{1F4A5}", "\u{26A1}", "\u{1F5E2}"];
                ui.label(egui::RichText::new(bolt_frames[frame]).size(28.0));

                ui.heading(egui::RichText::new("TurboSearch").strong().color(egui::Color32::from_rgb(0, 120, 212)));
                ui.label(egui::RichText::new("File Search").small().color(egui::Color32::GRAY));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Font:");
                    if ui.add(egui::Slider::new(&mut self.font_size, 10.0..=24.0).text("Size")).changed() {
                        self.apply_font_size(ctx);
                    }

                    ui.separator();

                    // Theme toggle
                    ui.label("Theme:");
                    let themes = [
                        (AppTheme::Light, "\u{2600}"),
                        (AppTheme::Dark, "\u{1F319}"),
                        (AppTheme::Blue, "\u{1F499}"),
                        (AppTheme::Green, "\u{1F49A}"),
                    ];
                    for (theme, icon) in themes {
                        let is_active = self.theme == theme;
                        let btn = egui::Button::new(egui::RichText::new(icon).size(16.0))
                            .frame(false)
                            .fill(if is_active {
                                ui.style().visuals.selection.bg_fill
                            } else {
                                egui::Color32::TRANSPARENT
                            });
                        if ui.add(btn).clicked() {
                            self.theme = theme;
                            self.apply_theme(ctx);
                        }
                    }

                    ui.separator();

                    // Settings button
                    if ui.button("\u{2699} Settings").clicked() {
                        self.show_settings = true;
                    }
                });
            });

            ui.separator();

            // Search path row
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("\u{1F4C1}").size(16.0));
                ui.label("Search in:");

                if self.search_path.as_os_str().is_empty() {
                    ui.label(egui::RichText::new("<No folder selected>").small().color(egui::Color32::GRAY));
                } else {
                    ui.label(
                        egui::RichText::new(truncate_path(&self.search_path.display().to_string(), 50))
                            .small()
                            .background_color(ui.style().visuals.faint_bg_color)
                    );
                }

                if self.is_indexing {
                    ui.spinner();
                    ui.label("Indexing...");
                } else if ui.add(egui::Button::new(egui::RichText::new("\u{1F4C2} Browse").color(egui::Color32::WHITE).background_color(egui::Color32::from_rgb(0, 120, 212)))).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.search_path = path.clone();
                        self.search_path_text = path.display().to_string();
                        save_last_search_path(&path);

                        if let Some(data_dir) = dirs::data_local_dir() {
                            let path_hash = path_to_safe_filename(&self.search_path);

                            // Check turbo-search first
                            let app_dir = data_dir.join("turbo-search");
                            let mut index_file = app_dir.join(format!("index_{}.gz", path_hash));

                            // If not found, try old ripgrep-soft directory
                            if !index_file.exists() {
                                let old_app_dir = data_dir.join("ripgrep-soft");
                                let old_index_file = old_app_dir.join(format!("index_{}.gz", path_hash));
                                if old_index_file.exists() {
                                    // Copy to new location for future use
                                    if let Err(e) = std::fs::copy(&old_index_file, &index_file) {
                                        eprintln!("Failed to migrate index: {}", e);
                                        index_file = old_index_file;
                                    }
                                }
                            }

                            if index_file.exists() {
                                self.is_indexing = true;
                                self.progress_message = "Loading saved index...".to_string();

                                let index_file_path = index_file;

                                let (tx, rx) = mpsc::channel();
                                thread::spawn(move || {
                                    let result = FileIndex::load(&index_file_path).ok();
                                    let _ = tx.send(result);
                                });
                                self.index_channel = Some(rx);
                            } else {
                                self.index_channel = Some(self.start_background_indexing());
                            }
                        }
                    }
                }
            });

            ui.separator();

            // Search row
            ui.horizontal(|ui| {
                let search_response = ui.add_sized(
                    [320.0, 32.0],
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search files...")
                        .frame(true)
                );

                ui.label(egui::RichText::new("\u{1F50D}").size(18.0).color(egui::Color32::GRAY));

                if !self.search_query.is_empty()
                    && ui.add(egui::Button::new(egui::RichText::new("\u{2715}").small()).frame(false)).clicked()
                {
                    self.search_query.clear();
                }

                if search_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && !self.search_query.is_empty() && !self.search_path.as_os_str().is_empty() && !self.is_indexing {
                        self.last_query.clear();
                        self.perform_search();
                    }

                if ui.add(egui::Button::new(egui::RichText::new("Search").color(egui::Color32::WHITE).background_color(egui::Color32::from_rgb(0, 120, 212)))).clicked() && !self.search_query.is_empty() {
                    self.last_query.clear();
                    if !self.search_path.as_os_str().is_empty() && !self.is_indexing {
                        self.perform_search();
                    }
                }

                // Favorites button
                ui.separator();
                egui::ComboBox::from_id_salt("favorites")
                    .selected_text("\u{2B50} Favorites")
                    .show_ui(ui, |ui| {
                        // Clone favorites to avoid borrow issues
                        let favorites_clone: Vec<FavoriteSearch> = self.favorites.favorites.clone();

                        if favorites_clone.is_empty() {
                            ui.label(egui::RichText::new("No favorites yet").small().color(egui::Color32::GRAY));
                        } else {
                            for fav in favorites_clone {
                                let icon = if fav.search_mode == SearchMode::Content {
                                    "\u{1F4DD}" // Document for content search
                                } else {
                                    "\u{1F4C1}" // Folder for filename search
                                };
                                let label = format!("{} {}", icon, fav.name);
                                if ui.button(label).clicked() {
                                    self.apply_favorite(&fav);
                                    self.show_favorites_dropdown = false;
                                }
                            }
                            ui.separator();
                        }

                        // Add to favorites option
                        if ui.button("+ Add Current Search").clicked() {
                            // Show a simple dialog by setting a flag
                            // For simplicity, we'll use the search query as name
                            if !self.search_query.is_empty() {
                                let name = format!("Search: {}", self.search_query);
                                self.add_to_favorites(name);
                            }
                        }
                    });

                ui.separator();

                ui.label("Mode:");
                if ui.selectable_label(self.search_mode == SearchMode::Filename, "Filename").clicked() {
                    self.search_mode = SearchMode::Filename;
                }
                if ui.selectable_label(self.search_mode == SearchMode::Content, "Content").clicked() {
                    self.search_mode = SearchMode::Content;
                }

                ui.separator();

                if self.search_mode == SearchMode::Content {
                    if self.ripgrep_available {
                        ui.checkbox(&mut self.use_ripgrep, "ripgrep");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(200, 100, 100), "[no ripgrep]");
                    }
                }
            });

            // Search options row
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Options:").small().color(egui::Color32::GRAY));

                if ui.selectable_label(self.use_regex, "[.*] Regex").clicked() {
                    self.use_regex = !self.use_regex;
                }

                if ui.selectable_label(self.use_glob, "[*] Glob").clicked() {
                    self.use_glob = !self.use_glob;
                }

                if ui.selectable_label(self.case_sensitive, "[Aa] Case").clicked() {
                    self.case_sensitive = !self.case_sensitive;
                }

                ui.separator();

                ui.label(egui::RichText::new("Size:").small().color(egui::Color32::GRAY));

                egui::ComboBox::from_id_salt("size_filter")
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.size_filter, "".to_string(), "None");
                        ui.selectable_value(&mut self.size_filter, "<1k".to_string(), "<1k");
                        ui.selectable_value(&mut self.size_filter, "<1m".to_string(), "<1m");
                        ui.selectable_value(&mut self.size_filter, "<10m".to_string(), "<10m");
                        ui.selectable_value(&mut self.size_filter, "<100m".to_string(), "<100m");
                    });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clear").clicked() {
                        self.reset_search();
                    }
                });
            });
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            // Pagination controls at the top of results
            if self.pagination.needs_pagination() {
                self.render_pagination(ui);
                ui.separator();
            }

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Calculate pagination bounds
                    let offset = if self.pagination.needs_pagination() {
                        self.pagination.offset()
                    } else {
                        0
                    };
                    let limit = if self.pagination.needs_pagination() {
                        self.pagination.limit()
                    } else {
                        self.displayed_results.len()
                    };

                    let max_display = limit.min(self.displayed_results.len().saturating_sub(offset));

                    for idx in 0..max_display {
                        let actual_idx = offset + idx;
                        let is_selected = self.selected_index == Some(actual_idx);

                        if let Some(entry) = self.displayed_results.get(actual_idx) {
                            let file_name = entry
                                .path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            let ext = entry
                                .path
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("");

                            let category = FileCategory::from_extension(ext);
                            let icon = category.icon();

                            // Display with index number
                            let display_with_icon = format!("{}. {}  {}", actual_idx + 1, icon, file_name);

                            let response = ui.selectable_label(is_selected, &display_with_icon);

                            if response.clicked() {
                                self.selected_index = Some(actual_idx);
                                self.preview_content = String::new();
                            }

                            if response.hovered() {
                                ui.label(egui::RichText::new(format!("  {} | {}", format_size(entry.size), entry.path.display()))
                                    .small()
                                    .color(egui::Color32::GRAY));
                            }
                        }
                    }

                    if self.displayed_results.is_empty() && !self.search_query.is_empty() && !self.is_indexing {
                        ui.label("No results");
                    } else if self.search_query.is_empty() {
                        ui.label("Enter search term");
                    }
                });
        });

        // Preview panel on the right
        egui::SidePanel::right("preview_panel")
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.heading("Preview");
                ui.separator();

                if let Some(idx) = self.selected_index {
                    if let Some(entry) = self.displayed_results.get(idx) {
                        // Load preview if needed
                        if self.preview_content.is_empty() || self.preview_path.as_ref() != Some(&entry.path) {
                            self.preview_content = Self::read_file_content_sync(&entry.path);
                            self.preview_path = Some(entry.path.clone());
                        }

                        // Show file info
                        ui.label(egui::RichText::new(entry.path.display().to_string()).small().color(egui::Color32::GRAY));
                        ui.label(format!("Size: {}", format_size(entry.size)));

                        // Show play button for media files
                        let is_video = Self::is_video_file(&entry.path);
                        let is_audio = Self::is_audio_file(&entry.path);

                        if is_video || is_audio {
                            ui.horizontal(|ui| {
                                let media_type = if is_video { "Video" } else { "Audio" };
                                let icon = if is_video { "\u{1F3AC}" } else { "\u{1F3B5}" };

                                if ui.add(egui::Button::new(
                                    egui::RichText::new(format!(" {} Play {}", icon, media_type))
                                        .color(egui::Color32::WHITE))
                                        .fill(egui::Color32::from_rgb(76, 175, 80))
                                        .frame(true))
                                    .clicked()
                                {
                                    // Get selected player path
                                    let player_path = self.selected_player
                                        .as_ref()
                                        .and_then(|name| {
                                            self.available_players
                                                .iter()
                                                .find(|(n, _)| n == name)
                                                .map(|(_, p)| p.clone())
                                        })
                                        .unwrap_or_else(|| "default".to_string());

                                    Self::open_with_player(&entry.path, &player_path);
                                }
                            });
                        }

                        ui.separator();

                        // Show content
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.add(egui::TextEdit::multiline(&mut self.preview_content.clone())
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(30)
                                    .interactive(false));
                            });
                    }
                } else {
                    ui.label("Click a file to preview");
                }
            });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            let time = ctx.input(|i| i.time);

            if self.is_searching {
                // Animated rotating globe for searching
                let frame = ((time * 4.0) as usize) % 12;
                let globe_frames = [
                    "\u{1F30D}", "\u{1F30E}", "\u{1F30F}",
                    "\u{1F310}", "\u{1F311}", "\u{1F312}",
                    "\u{1F313}", "\u{1F314}", "\u{1F315}",
                    "\u{1F316}", "\u{1F317}", "\u{1F318}",
                ];
                let globe = globe_frames[frame];
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(globe).size(16.0));
                    ui.label(
                        egui::RichText::new(format!("Searching... {} results", self.total_results))
                            .color(egui::Color32::from_rgb(255, 183, 0)),
                    );
                    ui.spinner();
                });
            } else if self.is_indexing {
                // Animated arrows for indexing
                let frame = ((time * 4.0) as usize) % 4;
                let index_frames = ["\u{1F504}", "\u{1F501}", "\u{1F500}", "\u{1F501}"];
                let index_icon = index_frames[frame];
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(index_icon).size(16.0));
                    ui.label(
                        egui::RichText::new(format!("Indexing... {}", self.progress_message))
                            .color(egui::Color32::from_rgb(64, 122, 204)),
                    );
                    ui.spinner();
                });
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("\u{2705}").size(14.0));
                    ui.label(
                        egui::RichText::new(format!("Ready - {} files indexed", self.index.len()))
                            .color(egui::Color32::from_rgb(76, 175, 80))
                            .small(),
                    );

                    ui.separator();

                    let mode_text = if self.search_mode == SearchMode::Filename { "Filename" } else { "Content" };
                    let mode_color = if self.search_mode == SearchMode::Filename {
                        egui::Color32::from_rgb(76, 175, 80)
                    } else {
                        egui::Color32::from_rgb(156, 39, 176)
                    };
                    ui.label(
                        egui::RichText::new(format!("[{mode_text}]"))
                            .small()
                            .color(egui::Color32::WHITE)
                            .background_color(mode_color)
                    );

                    ui.label(egui::RichText::new(format!("{} results", self.displayed_results.len())).small());

                    if let Some(duration) = self.last_search_duration {
                        ui.label(egui::RichText::new(format!("{duration} ms")).small().color(egui::Color32::GRAY));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("Max: {}", self.max_index_files)).small().color(egui::Color32::GRAY));
                    });
                });
            }
        });
    }
}

/// Load Chinese fonts (Microsoft YaHei, SimHei, etc.)
fn load_chinese_fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();

    // Try to load Windows Chinese fonts
    #[cfg(windows)]
    {
        let font_paths = [
            "C:\\Windows\\Fonts\\msyh.ttc",   // Microsoft YaHei
            "C:\\Windows\\Fonts\\msyh.ttf",   // Microsoft YaHei (fallback)
            "C:\\Windows\\Fonts\\simhei.ttf",  // SimHei
            "C:\\Windows\\Fonts\\simsun.ttc",  // SimSun
        ];

        for font_path in font_paths {
            if let Ok(font_data) = std::fs::read(font_path) {
                fonts.font_data.insert(
                    font_path.to_string(),
                    Arc::new(FontData::from_owned(font_data)),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, font_path.to_string());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, font_path.to_string());
                break;
            }
        }
    }

    fonts
}

/// Create a simple 16x16 colored icon programmatically
fn create_simple_icon() -> egui::IconData {
    let size = 16;
    let mut rgba = vec![0u8; size * size * 4];

    // Create a blue to purple gradient with search icon shape
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;

            // Circle in center (search glass)
            let cx = x as f32 - 7.0;
            let cy = y as f32 - 7.0;
            let dist = (cx * cx + cy * cy).sqrt();

            if dist < 5.0 {
                // Blue gradient circle
                let t = y as f32 / size as f32;
                rgba[idx] = (33.0 + t * 123.0) as u8;     // R
                rgba[idx + 1] = (150.0 - t * 39.0) as u8; // G
                rgba[idx + 2] = (243.0 - t * 67.0) as u8; // B
                rgba[idx + 3] = 255; // A
            } else if x > 10 && y > 10 {
                // Handle (bottom right)
                rgba[idx] = 33;     // R
                rgba[idx + 1] = 150; // G
                rgba[idx + 2] = 243; // B
                rgba[idx + 3] = 255; // A
            } else {
                // Transparent
                rgba[idx] = 0;
                rgba[idx + 1] = 0;
                rgba[idx + 2] = 0;
                rgba[idx + 3] = 0;
            }
        }
    }

    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

pub fn run_gui() -> Result<(), eframe::Error> {
    // Create a simple colored icon programmatically
    let icon_data = create_simple_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("TurboSearch - File Search")
            .with_icon(icon_data),
        ..Default::default()
    };

    // Load Chinese fonts before running
    let chinese_fonts = load_chinese_fonts();

    eframe::run_native(
        "TurboSearch",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_fonts(chinese_fonts);
            Ok(Box::new(RipgrepApp::new()))
        }),
    )
}
