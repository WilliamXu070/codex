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
pub use error::{Result, RetrievalError};

// Re-export from dependencies for convenience
pub use codex_context_files::{ContextFile, ContextStore, Query, QueryResult};
pub use codex_directory_watcher::{DirectoryConfig, DirectoryWatcher, FileEvent};
pub use codex_embeddings::{EmbeddingProvider, SimilarityIndex};
