//! Common utility functions for TurboSearch

use std::path::Path;
use std::path::PathBuf;

/// Format file size in human-readable format
pub fn format_size(bytes: u64) -> String {
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
        format!("{bytes} B")
    }
}

/// Truncate path for display
pub fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        let start = path.len() - max_len + 3;
        format!("...{}", &path[start..])
    }
}

/// Check if file is a video
pub fn is_video_file(path: &Path) -> bool {
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(extension.as_str(), "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg" | "3gp")
}

/// Check if file is audio
pub fn is_audio_file(path: &Path) -> bool {
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(extension.as_str(), "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a")
}

/// Known media players to detect on Windows
pub const KNOWN_PLAYERS: &[(&str, &str)] = &[
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
pub fn detect_media_players() -> Vec<(String, String)> {
    use std::collections::HashSet;
    use std::env;

    let mut found_players: Vec<(String, String)> = Vec::new();
    let mut checked_paths: HashSet<String> = HashSet::new();

    let mut search_dirs: Vec<PathBuf> = Vec::new();

    let path_var = env::var("PATH").unwrap_or_default();
    search_dirs.extend(env::split_paths(&path_var));

    if let Ok(program_files) = env::var("ProgramFiles") {
        search_dirs.push(program_files.into());
    }
    if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
        search_dirs.push(program_files_x86.into());
    }

    let common_player_dirs = [
        "VideoLAN", "Potplayer", "MPC-HC", "K-Lite Codec Pack",
        "GOM", "KMPlayer", "smplayer", "MPV",
    ];
    for dir in &common_player_dirs {
        if let Ok(program_files) = env::var("ProgramFiles") {
            search_dirs.push(PathBuf::from(&program_files).join(dir));
        }
        if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
            search_dirs.push(PathBuf::from(&program_files_x86).join(dir));
        }
    }

    for (name, exe_name) in KNOWN_PLAYERS {
        for search_dir in &search_dirs {
            if !search_dir.exists() {
                continue;
            }
            let exe_path = search_dir.join(exe_name);
            if exe_path.exists() {
                let path_str = exe_path.to_string_lossy().to_string();
                if !checked_paths.contains(&path_str) {
                    checked_paths.insert(path_str.clone());
                    found_players.push((name.to_string(), path_str));
                }
            }
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

    if !found_players.is_empty() {
        found_players.insert(0, ("System Default".to_string(), "default".to_string()));
    }

    found_players
}

#[cfg(not(windows))]
pub fn detect_media_players() -> Vec<(String, String)> {
    vec![
        ("System Default".to_string(), "xdg-open".to_string()),
    ]
}

/// Open file with system default player
pub fn open_with_player(path: &Path, player_path: &str) {
    #[cfg(windows)]
    {
        use std::process::Command;
        let path_str = path.to_string_lossy().to_string();

        if player_path == "default" || player_path.is_empty() {
            let _ = Command::new("cmd")
                .args(["/c", "start", "", &path_str])
                .spawn();
        } else {
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

/// Native Windows folder picker - uses PowerShell to open folder picker dialog
/// This avoids the rfd crate which can trigger Microsoft Store prompts
#[cfg(windows)]
pub fn pick_folder_native() -> Option<PathBuf> {
    use std::process::Command;

    // Use PowerShell to show folder browser dialog - no Microsoft Store trigger
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.Description = 'Select a folder to search'
$dialog.ShowNewFolderButton = $false
if ($dialog.ShowDialog() -eq 'OK') {
    Write-Output $dialog.SelectedPath
}
"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok()?;

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !path.is_empty() {
        Some(PathBuf::from(path))
    } else {
        None
    }
}

/// Fallback folder picker for non-Windows
#[cfg(not(windows))]
pub fn pick_folder_native() -> Option<PathBuf> {
    // On non-Windows, we could use rfd or native dialogs
    // For now, return None to indicate no folder was selected
    None
}

/// Open file with default system application
pub fn open_with_default_player(path: &Path) {
    #[cfg(windows)]
    {
        use std::process::Command;
        let path_str = path.to_string_lossy();
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

/// Generate a safe filename from a path using a hash
pub fn path_to_safe_filename(path: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let path_str = path.to_string_lossy();
    let mut hasher = DefaultHasher::new();
    path_str.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Get the app config directory path
pub fn get_app_config_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("turbo-search"))
}

/// Save the last search path to config file
pub fn save_last_search_path(path: &Path) {
    if let Some(config_dir) = get_app_config_dir() {
        if let Err(e) = std::fs::create_dir_all(&config_dir) {
            eprintln!("Failed to create config directory: {e}");
            return;
        }
        let config_file = config_dir.join("last_path.txt");
        if let Err(e) = std::fs::write(&config_file, path.to_string_lossy().as_bytes()) {
            eprintln!("Failed to save last path: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_truncate_path() {
        assert_eq!(truncate_path("short", 10), "short");
        assert_eq!(truncate_path("very_long_path", 10), "...ng_path");
        assert_eq!(truncate_path("exact", 5), "exact");
    }

    #[test]
    fn test_is_video_file() {
        assert!(is_video_file(Path::new("test.mp4")));
        assert!(is_video_file(Path::new("test.avi")));
        assert!(is_video_file(Path::new("test.mkv")));
        assert!(!is_video_file(Path::new("test.txt")));
        assert!(!is_video_file(Path::new("test.mp3")));
    }

    #[test]
    fn test_is_audio_file() {
        assert!(is_audio_file(Path::new("test.mp3")));
        assert!(is_audio_file(Path::new("test.wav")));
        assert!(is_audio_file(Path::new("test.flac")));
        assert!(!is_audio_file(Path::new("test.txt")));
        assert!(!is_audio_file(Path::new("test.mp4")));
    }

    #[test]
    fn test_path_to_safe_filename() {
        let result = path_to_safe_filename(Path::new("test/path"));
        assert!(!result.is_empty());
        assert_eq!(result.len(), 16); // hex hash
    }

    #[test]
    fn test_get_app_config_dir() {
        let dir = get_app_config_dir();
        assert!(dir.is_some());
        let path = dir.unwrap();
        assert!(path.to_string_lossy().contains("turbo-search"));
    }
}
