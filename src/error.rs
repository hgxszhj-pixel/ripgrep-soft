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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AppError::Parse("invalid format".to_string());
        assert!(err.to_string().contains("Parse error"));

        let err = AppError::Index("failed to index".to_string());
        assert!(err.to_string().contains("Index error"));

        let err = AppError::Search("no results".to_string());
        assert!(err.to_string().contains("Search error"));

        let err = AppError::Config("missing file".to_string());
        assert!(err.to_string().contains("Configuration error"));
    }

    #[test]
    fn test_error_constructors() {
        let err = AppError::parse("test");
        assert!(matches!(err, AppError::Parse(_)));

        let err = AppError::index("test");
        assert!(matches!(err, AppError::Index(_)));

        let err = AppError::search("test");
        assert!(matches!(err, AppError::Search(_)));

        let err = AppError::config("test");
        assert!(matches!(err, AppError::Config(_)));

        let err = AppError::ui("test");
        assert!(matches!(err, AppError::Ui(_)));

        let err = AppError::serialization("test");
        assert!(matches!(err, AppError::Serialization(_)));

        let err = AppError::path("test");
        assert!(matches!(err, AppError::Path(_)));

        let err = AppError::regex("test");
        assert!(matches!(err, AppError::Regex(_)));

        let err = AppError::glob("test");
        assert!(matches!(err, AppError::Glob(_)));
    }

    #[test]
    fn test_error_serialization() {
        let err = AppError::parse("test error");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("Parse error"));
    }

    #[test]
    fn test_result_type() {
        fn sample_fn() -> Result<i32> {
            Ok(42)
        }

        let result: Result<i32> = sample_fn();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
