//! Error types for the unified retrieval engine.

use thiserror::Error;

/// Result type alias for retrieval operations.
pub type Result<T> = std::result::Result<T, RetrievalError>;

/// Errors that can occur in the retrieval engine.
#[derive(Error, Debug)]
pub enum RetrievalError {
    /// Context file error.
    #[error("context error: {0}")]
    Context(#[from] codex_context_files::ContextError),

    /// Embedding error.
    #[error("embedding error: {0}")]
    Embedding(#[from] codex_embeddings::EmbeddingError),

    /// Directory watcher error.
    #[error("watcher error: {0}")]
    Watcher(#[from] codex_directory_watcher::WatcherError),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),

    /// Query processing error.
    #[error("query error: {0}")]
    Query(String),

    /// Engine not initialized.
    #[error("engine not initialized")]
    NotInitialized,

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
