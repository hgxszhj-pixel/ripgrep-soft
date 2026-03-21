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
        format!("{} B", bytes)
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
