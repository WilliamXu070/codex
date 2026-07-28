//! Error types for the directory watcher.

use thiserror::Error;

/// Result type alias for watcher operations.
pub type Result<T> = std::result::Result<T, WatcherError>;

/// Errors that can occur in the directory watcher.
#[derive(Error, Debug)]
pub enum WatcherError {
    /// Directory not found.
    #[error("directory not found: {0}")]
    DirectoryNotFound(String),

    /// Permission denied.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Watcher already running.
    #[error("watcher already running for: {0}")]
    AlreadyWatching(String),

    /// Invalid exclude pattern.
    #[error("invalid exclude pattern: {0}")]
    InvalidPattern(String),

    /// Watch limit exceeded.
    #[error("watch limit exceeded: too many directories")]
    WatchLimitExceeded,

    /// Notify error.
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Channel send error.
    #[error("channel error: failed to send event")]
    ChannelSend,

    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),
}
