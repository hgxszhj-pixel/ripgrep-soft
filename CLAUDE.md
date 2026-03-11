# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TurboSearch (formerly ripgrep-soft) is a high-performance file and content search tool for Windows, combining Everything's instant filename search with ripgrep's powerful content matching. Supports both CLI and GUI modes.

## Build Commands

```bash
# Development build
cargo build

# Release build (produces target/release/turbo-search.exe)
cargo build --release

# Run in development
cargo run

# Run GUI directly
cargo run -- --gui

# Run tests
cargo test

# Run a single test
cargo test test_function_name

# Check for warnings
cargo check
```

## Architecture

### Core Modules

- **main.rs** - Entry point. Handles CLI parsing via clap, GUI launch, and command dispatch.
- **cli.rs** - CLI argument definitions using clap. Defines `Cli` struct with `search`, `index`, and `history` subcommands.
- **search.rs** - Search functionality:
  - `Searcher` / `SearchQuery` - filename search with fuzzy matching (SkimMatcherV2), glob patterns, and regex support
  - `ContentSearcher` / `ContentSearchQuery` - content search inside files using regex
- **index.rs** - File indexing:
  - `FileEntry` - individual file metadata (path, name, size, modified timestamp)
  - `FileIndex` - collection with multiple indexing methods:
    - `walk_directory_limited()` - Single-pass traversal with early termination
    - `walk_directory_parallel()` - Parallel processing for large directories
    - `walk_directory_parallel_high_performance()` - Parallel top-level subdirectory processing
    - `walk_directory_jwalk()` - **jwalk-based** high-performance parallel traversal (~4x faster with sorting)
  - Uses both **walkdir** and **jwalk** (jwalk is faster for sorted results)
  - `mft_indexer` submodule - Windows NTFS MFT reading
  - `os_str_to_string()` - handles UTF-8, UTF-16, and GBK encoding for Windows filename compatibility
- **gui.rs** - eframe/egui-based GUI. Uses background threading for indexing with mpsc channels.
- **history.rs** - Search history persistence.
- **file_watcher.rs** - File system monitoring using `notify` crate for incremental indexing.
- **mft_reader.rs** - Windows MFT reader using FindFirstFile API for ultra-fast enumeration
- **logging.rs** - Tracing-based logging with file appender.
- **error.rs** - Error types using thiserror.

### GUI Workflow

1. App starts with lazy loading - no automatic indexing on startup
2. User selects a search path via folder picker
3. Background indexing starts via `start_background_indexing()` using `walk_directory_jwalk()` (jwalk-based) for maximum performance
4. Results stream back via mpsc channel and update UI
5. Index is saved to `%LOCALAPPDATA%/turbo-search/index_*.gz` for persistence

### Indexing Performance Optimization

The project uses multiple indexing strategies:

| Method | Use Case | Performance |
|--------|----------|-------------|
| `walk_directory_limited()` | Default with max file limit | Good |
| `walk_directory_parallel()` | Large directories | Better |
| `walk_directory_parallel_high_performance()` | Very large dirs (e.g., C:\) | Best |
| `walk_directory_jwalk()` | Sorted results needed | ~4x faster than walkdir |

**Key optimizations:**
- Single-pass traversal (no double directory walk)
- `min_depth(1)` skips root directory
- `same_file_system(true)` avoids cross-mount traversal
- Adaptive parallelism (>1000 files uses rayon, else sequential)
- **jwalk** used for sorted output (built-in parallel + sorting)

### File Watcher (Incremental Indexing)

- **file_watcher.rs** - File system monitoring using `notify` crate
  - `FileWatcher` - Watch for file changes (create, modify, remove)
  - `FileChange` - Enum with Created/Modified/Removed events
  - Uses Windows ReadDirectoryChangesW API

### Key Design Patterns

- **Builder pattern**: `SearchQuery`, `ContentSearchQuery` use builder methods (`.with_case_sensitive()`, `.with_regex()`)
- **Conditional compilation**: Windows-specific features guarded with `#[cfg(windows)]`
- **Serde serialization**: `FileIndex` and `FileEntry` are serializable with gzip compression for persistence
- **Background threading**: GUI uses `std::thread::spawn` with mpsc channels for non-blocking indexing

### CLI Usage

```bash
# Search files by name
turbo-search.exe search --path C:\Users --pattern "document"

# Glob pattern search
turbo-search.exe search --path D:\ --pattern "*.pdf"

# Content search
turbo-search.exe search --path C:\Projects --content "TODO"

# Regex search
turbo-search.exe search --path . --pattern "\.rs$" --regex

# Build index
turbo-search.exe index --path C:\Users\YourName\Documents
```
