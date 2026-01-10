//! # Embeddings
//!
//! This crate provides semantic embedding generation and similarity search
//! for the Codex context system.
//!
//! ## Features
//!
//! - **Embedding Generation**: Convert text to dense vectors using AI models
//! - **Similarity Search**: Find semantically similar content
//! - **Multiple Providers**: Support for OpenAI, local models, etc.
//! - **Caching**: Efficient caching of computed embeddings
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Embeddings System                            │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  EmbeddingProvider ──► Embedding ──► EmbeddingStore            │
//! │       │                    │              │                     │
//! │       ▼                    ▼              ▼                     │
//! │  OpenAI/Local        SimilarityIndex  VectorCache              │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod cache;
pub mod error;
pub mod index;
pub mod provider;
pub mod similarity;

pub use cache::EmbeddingCache;
pub use error::{EmbeddingError, Result};
pub use index::SimilarityIndex;
pub use provider::{EmbeddingProvider, EmbeddingRequest, EmbeddingResponse, OpenAIProvider};
pub use similarity::{cosine_similarity, SimilarityResult};

/// A dense vector embedding.
pub type Embedding = Vec<f32>;

/// Dimension of embeddings (varies by model).
pub const DEFAULT_DIMENSION: usize = 1536; // OpenAI text-embedding-3-small
