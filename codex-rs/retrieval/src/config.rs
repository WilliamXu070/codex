//! Configuration for the unified retrieval engine.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration for the unified retrieval engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// Path to store context files.
    pub context_dir: PathBuf,

    /// Directories to watch for changes.
    pub watch_dirs: Vec<PathBuf>,

    /// Embedding provider configuration.
    pub embedding: EmbeddingConfig,

    /// Query processing configuration.
    pub query: QueryConfig,

    /// Sync configuration.
    pub sync: SyncConfig,
}

impl RetrievalConfig {
    /// Create a new configuration with default values.
    pub fn new(context_dir: impl Into<PathBuf>) -> Self {
        Self {
            context_dir: context_dir.into(),
            watch_dirs: Vec::new(),
            embedding: EmbeddingConfig::default(),
            query: QueryConfig::default(),
            sync: SyncConfig::default(),
        }
    }

    /// Add a directory to watch.
    pub fn with_watch_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.watch_dirs.push(dir.into());
        self
    }

    /// Set the embedding configuration.
    pub fn with_embedding(mut self, config: EmbeddingConfig) -> Self {
        self.embedding = config;
        self
    }

    /// Set the query configuration.
    pub fn with_query(mut self, config: QueryConfig) -> Self {
        self.query = config;
        self
    }
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self::new(dirs::data_dir().unwrap_or_default().join("codex/contexts"))
    }
}

/// Configuration for the embedding provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Which provider to use.
    pub provider: EmbeddingProviderType,

    /// Model to use for embeddings.
    pub model: Option<String>,

    /// Whether to cache embeddings.
    pub cache_enabled: bool,

    /// Maximum cache size.
    pub cache_max_entries: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: EmbeddingProviderType::OpenAI,
            model: None,
            cache_enabled: true,
            cache_max_entries: 10000,
        }
    }
}

/// Type of embedding provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingProviderType {
    /// OpenAI embeddings API.
    OpenAI,
    /// Local embedding model.
    Local,
    /// No embeddings (keyword-only search).
    None,
}

/// Configuration for query processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConfig {
    /// Maximum number of results to return.
    pub max_results: usize,

    /// Minimum relevance score (0.0 to 1.0).
    pub min_relevance: f32,

    /// Weight for keyword matching.
    pub keyword_weight: f32,

    /// Weight for semantic similarity.
    pub semantic_weight: f32,

    /// Weight for recency.
    pub recency_weight: f32,

    /// Whether to expand queries to related concepts.
    pub expand_related: bool,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_relevance: 0.3,
            keyword_weight: 0.3,
            semantic_weight: 0.5,
            recency_weight: 0.2,
            expand_related: true,
        }
    }
}

/// Configuration for synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Whether to watch directories in real-time.
    pub realtime_watch: bool,

    /// Interval for scheduled scans (in seconds).
    pub scan_interval_secs: u64,

    /// How to resolve conflicts.
    pub conflict_resolution: ConflictResolution,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            realtime_watch: true,
            scan_interval_secs: 3600, // 1 hour
            conflict_resolution: ConflictResolution::Merge,
        }
    }
}

/// How to resolve sync conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    /// Keep the local version.
    KeepLocal,
    /// Keep the incoming version.
    KeepIncoming,
    /// Attempt to merge changes.
    Merge,
    /// Ask the user.
    AskUser,
}
