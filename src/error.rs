//! Error types for TurboSearch

use thiserror::Error;

/// Main application error type
#[derive(Debug, Error)]
pub enum AppError {
    /// IO errors (file system, network, etc.)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Parse errors (invalid input, format errors)
    #[error("Parse error: {0}")]
    Parse(String),

    /// Index errors (indexing operations)
    #[error("Index error: {0}")]
    Index(String),

    /// Search errors (search operations)
    #[error("Search error: {0}")]
    Search(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// UI errors
    #[error("UI error: {0}")]
    Ui(String),

    /// Serialization/Deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Path errors (invalid paths, path traversal)
    #[error("Path error: {0}")]
    Path(String),

    /// Regex compilation errors
    #[error("Regex error: {0}")]
    Regex(String),

    /// Glob pattern errors
    #[error("Glob pattern error: {0}")]
    Glob(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, AppError>;

/// Convenience constructors
impl AppError {
    /// Create a new parse error
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::Parse(msg.into())
    }

    /// Create a new index error
    pub fn index(msg: impl Into<String>) -> Self {
        Self::Index(msg.into())
    }

    /// Create a new search error
    pub fn search(msg: impl Into<String>) -> Self {
        Self::Search(msg.into())
    }

    /// Create a new configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a new UI error
    pub fn ui(msg: impl Into<String>) -> Self {
        Self::Ui(msg.into())
    }

    /// Create a new serialization error
    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::Serialization(msg.into())
    }

    /// Create a new path error
    pub fn path(msg: impl Into<String>) -> Self {
        Self::Path(msg.into())
    }

    /// Create a new regex error
    pub fn regex(msg: impl Into<String>) -> Self {
        Self::Regex(msg.into())
    }

    /// Create a new glob error
    pub fn glob(msg: impl Into<String>) -> Self {
        Self::Glob(msg.into())
    }
}
