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
