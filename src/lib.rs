//! ripgrep-soft - A high-performance file and content search tool

pub mod cli;
pub mod cli_search;
pub mod config;
pub mod error;
pub mod file_watcher;
pub mod gui;
pub mod history;
pub mod index;
pub mod logging;
pub mod search;
pub mod utils;
pub mod heartbeat;

#[cfg(windows)]
pub mod mft_reader;
