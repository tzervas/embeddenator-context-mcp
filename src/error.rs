//! Error types for the context MCP server

use thiserror::Error;

/// Result type alias for context operations
pub type Result<T> = std::result::Result<T, ContextError>;

/// Result type alias (alternative name)
pub type ContextResult<T> = std::result::Result<T, ContextError>;

/// Errors that can occur in context operations
#[derive(Error, Debug)]
pub enum ContextError {
    /// Context not found
    #[error("Context not found: {0}")]
    NotFound(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid query
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// Context expired
    #[error("Context has expired: {0}")]
    Expired(String),


    /// Security screening failed
    #[error("Security screening failed: {0}")]
    ScreeningFailed(String),

    /// Context blocked by security screening
    #[error("Context blocked: {0}")]
    Blocked(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ContextError {
    /// Check if this is a not found error
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound(_))
    }

    /// Check if this is a security-related error
    pub fn is_security_error(&self) -> bool {
        matches!(self, Self::ScreeningFailed(_) | Self::Blocked(_))
    }
}

impl From<sled::Error> for ContextError {
    fn from(err: sled::Error) -> Self {
        Self::Storage(err.to_string())
    }
}
