//! Error types for the context file system.

use thiserror::Error;

/// Result type alias for context operations.
pub type Result<T> = std::result::Result<T, ContextError>;

/// Errors that can occur in the context file system.
#[derive(Error, Debug)]
pub enum ContextError {
    /// Context file not found.
    #[error("context file not found: {0}")]
    NotFound(String),

    /// Concept already exists.
    #[error("concept already exists: {0}")]
    ConceptExists(String),

    /// Storage operation failed.
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Embedding generation error.
    #[error("embedding error: {0}")]
    Embedding(String),

    /// Query processing error.
    #[error("query error: {0}")]
    Query(String),

    /// Sync conflict detected.
    #[error("sync conflict: {0}")]
    SyncConflict(String),

    /// Invalid context file format.
    #[error("invalid format: {0}")]
    InvalidFormat(String),
}

/// Storage-specific errors.
#[derive(Error, Debug)]
pub enum StorageError {
    /// Failed to create storage directory.
    #[error("failed to create directory: {0}")]
    CreateDirectory(String),

    /// Failed to read context file.
    #[error("failed to read file: {0}")]
    ReadFile(String),

    /// Failed to write context file.
    #[error("failed to write file: {0}")]
    WriteFile(String),

    /// Failed to delete context file.
    #[error("failed to delete file: {0}")]
    DeleteFile(String),

    /// Database error (if using embedded DB).
    #[error("database error: {0}")]
    Database(String),
}
