//! ripgrep-soft - A high-performance file and content search tool

pub mod cli;
pub mod error;
pub mod index;
pub mod logging;
pub mod search;

#[cfg(windows)]
pub mod mft_reader;
