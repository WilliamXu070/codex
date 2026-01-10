//! Error types for the embeddings system.

use thiserror::Error;

/// Result type alias for embedding operations.
pub type Result<T> = std::result::Result<T, EmbeddingError>;

/// Errors that can occur in the embeddings system.
#[derive(Error, Debug)]
pub enum EmbeddingError {
    /// Provider not configured.
    #[error("embedding provider not configured")]
    ProviderNotConfigured,

    /// API request failed.
    #[error("API request failed: {0}")]
    ApiRequest(String),

    /// Invalid response from provider.
    #[error("invalid response: {0}")]
    InvalidResponse(String),

    /// Rate limit exceeded.
    #[error("rate limit exceeded, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    /// Dimension mismatch.
    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    /// Cache error.
    #[error("cache error: {0}")]
    Cache(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP error.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    /// Text too long for embedding.
    #[error("text too long: {length} characters, max {max_length}")]
    TextTooLong { length: usize, max_length: usize },
}
