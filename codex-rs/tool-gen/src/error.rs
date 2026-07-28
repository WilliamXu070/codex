//! Error types for the tool generation system.

use thiserror::Error;

/// Result type alias for tool operations.
pub type Result<T> = std::result::Result<T, ToolError>;

/// Errors that can occur in the tool generation system.
#[derive(Error, Debug)]
pub enum ToolError {
    /// Tool not found.
    #[error("tool not found: {0}")]
    NotFound(String),

    /// Tool already exists.
    #[error("tool already exists: {0}")]
    AlreadyExists(String),

    /// Invalid tool definition.
    #[error("invalid tool definition: {0}")]
    InvalidDefinition(String),

    /// Tool execution failed.
    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    /// Tool generation failed.
    #[error("generation failed: {0}")]
    GenerationFailed(String),

    /// Invalid tool input.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Tool dependency not met.
    #[error("dependency not met: {0}")]
    DependencyNotMet(String),

    /// Storage operation failed.
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Network error (for community features).
    #[error("network error: {0}")]
    Network(String),

    /// Version conflict.
    #[error("version conflict: expected {expected}, got {actual}")]
    VersionConflict { expected: String, actual: String },

    /// Security validation failed.
    #[error("security validation failed: {0}")]
    SecurityValidation(String),
}

/// Storage-specific errors.
#[derive(Error, Debug)]
pub enum StorageError {
    /// Failed to create storage directory.
    #[error("failed to create directory: {0}")]
    CreateDirectory(String),

    /// Failed to read tool file.
    #[error("failed to read file: {0}")]
    ReadFile(String),

    /// Failed to write tool file.
    #[error("failed to write file: {0}")]
    WriteFile(String),

    /// Failed to delete tool file.
    #[error("failed to delete file: {0}")]
    DeleteFile(String),
}
