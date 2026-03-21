//! Incremental indexing module
//! Provides file watching and incremental index updates

use crate::file_watcher::{FileChange, FileWatcher};
use crate::index::{FileEntry, FileIndex};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Incremental index manager that watches for file changes
pub struct IncrementalIndexer {
    watcher: Option<FileWatcher>,
    is_running: Arc<AtomicBool>,
}

impl IncrementalIndexer {
    /// Create a new incremental indexer
    pub fn new() -> Self {
        Self {
            watcher: None,
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start watching a path for file changes
    pub fn start_watching(&mut self, path: &Path) -> Result<(), notify::Error> {
        // Stop existing watcher if any
        self.stop_watching();

        let watcher = FileWatcher::new(path)?;
        self.watcher = Some(watcher);
        self.is_running.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Stop watching
    pub fn stop_watching(&mut self) {
        if let Some(mut watcher) = self.watcher.take() {
            watcher.stop();
        }
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Check if currently watching
    pub fn is_watching(&self) -> bool {
        self.watcher.as_ref().map(|w| w.is_watching()).unwrap_or(false)
    }

    /// Try to receive a file change (non-blocking)
    pub fn try_recv_change(&self) -> Option<FileChange> {
        self.watcher.as_ref()?.try_recv().ok()
    }

    /// Process file changes and update index
    pub fn process_changes(&self, index: &mut FileIndex) -> usize {
        let mut count = 0;

        while let Some(change) = self.try_recv_change() {
            match change {
                FileChange::Created(path) | FileChange::Modified(path) => {
                    if path.is_file() {
                        // Remove existing entry if any
                        index.remove_entry(&path);
                        // Add new entry
                        if let Some(entry) = FileEntry::from_path(&path) {
                            index.add_entry(entry);
                            count += 1;
                        }
                    }
                }
                FileChange::Removed(path) => {
                    if index.remove_entry(&path) {
                        count += 1;
                    }
                }
            }
        }

        count
    }
}

impl Default for IncrementalIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for IncrementalIndexer {
    fn drop(&mut self) {
        self.stop_watching();
    }
}

/// Start background indexing with file watching
pub fn start_background_index_with_watch(
    path: &Path,
    max_files: usize,
) -> mpsc::Receiver<IncrementalIndexState> {
    let (tx, rx) = mpsc::channel();
    let path = path.to_path_buf();

    thread::spawn(move || {
        let mut index = FileIndex::with_root(&path);

        // Initial indexing
        #[cfg(windows)]
        let count = index.walk_directory_jwalk(&path, max_files).unwrap_or(0);
        #[cfg(not(windows))]
        let count = index.walk_directory_limited(&path, max_files).unwrap_or(0);

        tx.send(IncrementalIndexState::Initial(index.clone(), count)).ok();

        // Try to start file watcher
        let mut indexer = IncrementalIndexer::new();
        if let Err(e) = indexer.start_watching(&path) {
            tracing::warn!("Failed to start file watcher: {}", e);
            return;
        }

        tracing::info!("Started watching {} for changes", path.display());

        // Process changes in a loop
        loop {
            thread::sleep(Duration::from_millis(500));

            let changes = indexer.process_changes(&mut index);
            if changes > 0 {
                tx.send(IncrementalIndexState::Updated(index.clone(), changes)).ok();
                tracing::debug!("Processed {} file changes", changes);
            }
        }
    });

    rx
}

/// State of incremental indexing
#[derive(Debug, Clone)]
pub enum IncrementalIndexState {
    /// Initial indexing complete
    Initial(FileIndex, usize),
    /// Index updated with changes
    Updated(FileIndex, usize),
    /// Error occurred
    Error(String),
}
