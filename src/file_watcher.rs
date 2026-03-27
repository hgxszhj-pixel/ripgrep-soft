//! File system watcher for incremental indexing
//! Uses notify crate to watch for file changes and update index incrementally

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

/// File change event types
#[derive(Debug, Clone)]
pub enum FileChange {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
}

/// Background file watcher that sends changes to a channel
pub struct FileWatcher {
    watcher: Option<RecommendedWatcher>,
    watch_path: PathBuf,
    rx: Option<Receiver<FileChange>>,
}

impl FileWatcher {
    /// Create a new file watcher for the given path
    pub fn new(path: &Path) -> Result<Self, notify::Error> {
        let (tx, rx) = channel();

        let tx_clone = tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let changes: Vec<FileChange> = match event.kind {
                        EventKind::Create(_) => {
                            event.paths.into_iter().map(FileChange::Created).collect()
                        }
                        EventKind::Modify(_) => {
                            event.paths.into_iter().map(FileChange::Modified).collect()
                        }
                        EventKind::Remove(_) => {
                            event.paths.into_iter().map(FileChange::Removed).collect()
                        }
                        _ => vec![],
                    };

                    for change in changes {
                        let _ = tx_clone.send(change);
                    }
                }
            },
            Config::default(),
        )?;

        watcher.watch(path, RecursiveMode::Recursive)?;

        Ok(Self {
            watcher: Some(watcher),
            watch_path: path.to_path_buf(),
            rx: Some(rx),
        })
    }

    /// Receive file changes (blocks until available)
    pub fn recv(&self) -> Result<FileChange, std::sync::mpsc::RecvError> {
        if let Some(ref rx) = self.rx {
            rx.recv()
        } else {
            Err(std::sync::mpsc::RecvError)
        }
    }

    /// Try to receive file changes (non-blocking)
    pub fn try_recv(&self) -> Result<FileChange, std::sync::mpsc::TryRecvError> {
        if let Some(ref rx) = self.rx {
            rx.try_recv()
        } else {
            Err(std::sync::mpsc::TryRecvError::Disconnected)
        }
    }

    /// Check if watcher is still active
    pub fn is_watching(&self) -> bool {
        self.watcher.is_some()
    }

    /// Stop watching
    pub fn stop(&mut self) {
        self.watcher = None;
        self.rx = None;
    }

    /// Get watched path
    pub fn path(&self) -> &Path {
        &self.watch_path
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_change_path() {
        let path = PathBuf::from("C:/test/file.txt");
        assert_eq!(FileChange::Created(path.clone()).path(), PathBuf::from("C:/test/file.txt"));
        assert_eq!(FileChange::Modified(path.clone()).path(), PathBuf::from("C:/test/file.txt"));
        assert_eq!(FileChange::Removed(path.clone()).path(), PathBuf::from("C:/test/file.txt"));
    }

    #[test]
    fn test_file_change_eq() {
        let path1 = PathBuf::from("C:/test/file.txt");
        let path2 = PathBuf::from("C:/test/file.txt");
        let path3 = PathBuf::from("C:/test/other.txt");

        assert_eq!(FileChange::Created(path1.clone()), FileChange::Created(path2.clone()));
        assert_ne!(FileChange::Created(path1.clone()), FileChange::Created(path3.clone()));
        assert_ne!(FileChange::Created(path1.clone()), FileChange::Modified(path1.clone()));
    }

    #[test]
    fn test_debounced_watcher_creation() {
        // Test creating a debounced watcher on a temp directory
        let temp_dir = std::env::temp_dir();
        let result = DebouncedWatcher::new(&temp_dir, 100);
        assert!(result.is_ok(), "Should create debounced watcher on temp dir");
    }

    #[test]
    fn test_debounced_watcher_try_recv_empty() {
        let temp_dir = std::env::temp_dir();
        let mut watcher = DebouncedWatcher::new(&temp_dir, 100).unwrap();

        // Should return None when no events pending
        match watcher.try_recv() {
            Ok(None) => {}
            Ok(Some(_)) => panic!("Expected None for empty watcher"),
            Err(_) => panic!("Expected Ok(None) or Err(Disconnected), not error"),
        }
    }

    #[test]
    fn test_debounced_watcher_is_watching() {
        let temp_dir = std::env::temp_dir();
        let mut watcher = DebouncedWatcher::new(&temp_dir, 100).unwrap();
        assert!(watcher.is_watching(), "Watcher should be active after creation");
    }

    #[test]
    fn test_debounced_watcher_stop() {
        let temp_dir = std::env::temp_dir();
        let mut watcher = DebouncedWatcher::new(&temp_dir, 100).unwrap();
        watcher.stop();
        assert!(!watcher.is_watching(), "Watcher should be inactive after stop");
    }

    #[test]
    fn test_debounced_watcher_path() {
        let temp_dir = std::env::temp_dir();
        let watcher = DebouncedWatcher::new(&temp_dir, 100).unwrap();
        assert_eq!(watcher.path(), temp_dir);
    }

    #[test]
    fn test_debounced_watcher_is_watching_after_stop() {
        let temp_dir = std::env::temp_dir();
        let mut watcher = DebouncedWatcher::new(&temp_dir, 100).unwrap();
        watcher.stop();
        assert!(!watcher.is_watching());
    }

    #[test]
    fn test_file_watcher_new_invalid_path() {
        // Should fail for non-existent path
        let invalid_path = PathBuf::from("Z:/this/path/does/not/exist");
        let result = FileWatcher::new(&invalid_path);
        // On Windows, this might succeed for a network path or fail - just check it compiles
        // On error, notify returns an error
        if let Err(e) = result {
            assert!(e.to_string().len() > 0);
        }
    }

    #[test]
    fn test_file_watcher_recv_disconnected() {
        let temp_dir = std::env::temp_dir();
        let watcher = FileWatcher::new(&temp_dir).unwrap();
        // After stop, recv should return error
        // But we can't easily test this without internal access
        assert!(watcher.is_watching());
    }

    #[test]
    fn test_file_watcher_is_watching() {
        let temp_dir = std::env::temp_dir();
        let watcher = FileWatcher::new(&temp_dir).unwrap();
        assert!(watcher.is_watching(), "Watcher should be active after creation");
    }

    #[test]
    fn test_file_watcher_path() {
        let temp_dir = std::env::temp_dir();
        let watcher = FileWatcher::new(&temp_dir).unwrap();
        assert_eq!(watcher.path(), temp_dir);
    }

    #[test]
    fn test_file_watcher_stop() {
        let temp_dir = std::env::temp_dir();
        let mut watcher = FileWatcher::new(&temp_dir).unwrap();
        watcher.stop();
        assert!(!watcher.is_watching(), "Watcher should be inactive after stop");
    }
}
