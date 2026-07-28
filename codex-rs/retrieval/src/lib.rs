//! # Retrieval Engine
//!
//! This crate provides a unified retrieval engine that combines:
//!
//! - **Context Files**: Persistent memory storage
//! - **Embeddings**: Semantic similarity search
//! - **Directory Watcher**: File system monitoring
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Unified Retrieval Engine                     │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                  │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
//! │  │   Context    │  │  Embeddings  │  │  Directory   │          │
//! │  │    Files     │  │    Engine    │  │   Watcher    │          │
//! │  └──────────────┘  └──────────────┘  └──────────────┘          │
//! │         │                │                  │                   │
//! │         └────────────────┼──────────────────┘                   │
//! │                          ▼                                      │
//! │                  ┌──────────────┐                               │
//! │                  │   Unified    │                               │
//! │                  │   Retrieval  │                               │
//! │                  │   Engine     │                               │
//! │                  └──────────────┘                               │
//! │                          │                                      │
//! │                          ▼                                      │
//! │                  ┌──────────────┐                               │
//! │                  │    Query     │                               │
//! │                  │  Processing  │                               │
//! │                  └──────────────┘                               │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use codex_retrieval::UnifiedRetrieval;
//!
//! let engine = UnifiedRetrieval::builder()
//!     .with_context_dir("~/.codex/contexts")
//!     .with_watch_dir("~/Documents")
//!     .build()
//!     .await?;
//!
//! let results = engine.query("What projects am I working on?").await?;
//! ```

pub mod config;
pub mod engine;
pub mod error;

pub use config::RetrievalConfig;
pub use engine::UnifiedRetrieval;
pub use error::Result;
pub use error::RetrievalError;

// Re-export from dependencies for convenience
pub use codex_context_files::ContextFile;
pub use codex_context_files::ContextStore;
pub use codex_context_files::Query;
pub use codex_context_files::QueryResult;
pub use codex_directory_watcher::DirectoryConfig;
pub use codex_directory_watcher::DirectoryWatcher;
pub use codex_directory_watcher::FileEvent;
pub use codex_embeddings::EmbeddingProvider;
pub use codex_embeddings::SimilarityIndex;
