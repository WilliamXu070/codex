//! Unified retrieval engine implementation.

use std::path::Path;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info};

use codex_context_files::{
    ConceptExtractor, ConceptIndex, ContextStore, Query, QueryResult, RetrievalEngine,
};
use codex_directory_watcher::{DirectoryConfig, DirectoryWatcher, FileEvent};
use codex_embeddings::{EmbeddingCache, OpenAIProvider, SimilarityIndex};

use crate::config::{EmbeddingProviderType, RetrievalConfig};
use crate::error::{Result, RetrievalError};

/// Unified retrieval engine that combines all retrieval components.
///
/// This is the main entry point for the Codex memory system. It coordinates:
/// - Context file storage and retrieval
/// - Semantic embedding generation and similarity search
/// - Directory watching and file indexing
/// - Query processing and result ranking
pub struct UnifiedRetrieval {
    /// Configuration.
    config: RetrievalConfig,

    /// Context file storage.
    context_store: Arc<RwLock<ContextStore>>,

    /// Concept index for fast lookup.
    concept_index: Arc<RwLock<ConceptIndex>>,

    /// Embedding similarity index.
    similarity_index: Arc<RwLock<SimilarityIndex>>,

    /// Directory watcher.
    watcher: Arc<RwLock<DirectoryWatcher>>,

    /// Concept extractor.
    extractor: ConceptExtractor,

    /// Context retrieval engine.
    retrieval: RetrievalEngine,

    /// Whether the engine is initialized.
    initialized: bool,
}

impl UnifiedRetrieval {
    /// Create a new unified retrieval engine builder.
    pub fn builder() -> UnifiedRetrievalBuilder {
        UnifiedRetrievalBuilder::new()
    }

    /// Initialize the engine with the given configuration.
    pub async fn new(config: RetrievalConfig) -> Result<Self> {
        info!("Initializing unified retrieval engine");

        // Initialize context store
        let context_store = ContextStore::new(&config.context_dir).await?;

        // Initialize concept index
        let concept_index = ConceptIndex::new();

        // Initialize similarity index
        let dimension = match config.embedding.provider {
            EmbeddingProviderType::OpenAI => 1536, // text-embedding-3-small
            EmbeddingProviderType::Local => 384,   // MiniLM
            EmbeddingProviderType::None => 0,
        };
        let similarity_index = if dimension > 0 {
            SimilarityIndex::new(dimension)
        } else {
            SimilarityIndex::new(1) // Placeholder
        };

        // Initialize directory watcher
        let mut watcher = DirectoryWatcher::new();
        for watch_dir in &config.watch_dirs {
            if watch_dir.exists() {
                let dir_config = DirectoryConfig::new(watch_dir);
                watcher.add(dir_config).await?;
            }
        }

        let engine = Self {
            config,
            context_store: Arc::new(RwLock::new(context_store)),
            concept_index: Arc::new(RwLock::new(concept_index)),
            similarity_index: Arc::new(RwLock::new(similarity_index)),
            watcher: Arc::new(RwLock::new(watcher)),
            extractor: ConceptExtractor::with_defaults(),
            retrieval: RetrievalEngine::with_defaults(),
            initialized: true,
        };

        info!("Unified retrieval engine initialized");
        Ok(engine)
    }

    /// Start watching directories.
    pub async fn start(&self) -> Result<()> {
        if self.config.sync.realtime_watch {
            self.watcher.write().await.start().await?;
            info!("Directory watching started");
        }
        Ok(())
    }

    /// Stop watching directories.
    pub async fn stop(&self) {
        self.watcher.write().await.stop().await;
        info!("Directory watching stopped");
    }

    /// Process a natural language query.
    pub async fn query(&self, query_text: &str) -> Result<QueryResult> {
        if !self.initialized {
            return Err(RetrievalError::NotInitialized);
        }

        debug!("Processing query: {query_text}");

        let store = self.context_store.read().await;
        let index = self.concept_index.read().await;

        let result = self.retrieval.retrieve(query_text, &store, &index)?;

        Ok(result)
    }

    /// Add or update a context file.
    pub async fn upsert_context(
        &self,
        concept: &str,
        summary: &str,
    ) -> Result<()> {
        let cf = codex_context_files::ContextFile::new(concept, summary);
        self.context_store.write().await.upsert(cf).await?;

        // Update concept index
        self.concept_index
            .write()
            .await
            .add_concept(codex_context_files::Concept::new(concept));

        debug!("Upserted context: {concept}");
        Ok(())
    }

    /// Get a context file by concept name.
    pub async fn get_context(&self, concept: &str) -> Option<codex_context_files::ContextFile> {
        self.context_store.read().await.get(concept).cloned()
    }

    /// List all concepts.
    pub async fn list_concepts(&self) -> Vec<String> {
        self.context_store
            .read()
            .await
            .list_concepts()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Extract concepts from text and optionally store them.
    pub async fn extract_and_store(&self, text: &str, store: bool) -> Result<Vec<String>> {
        let extracted = self.extractor.extract(text)?;

        let concept_names: Vec<String> = extracted
            .iter()
            .map(|e| e.concept.name.clone())
            .collect();

        if store {
            for ext in extracted {
                if !ext.is_known {
                    // New concept discovered
                    self.upsert_context(
                        &ext.concept.name,
                        &format!("Auto-discovered concept: {}", ext.concept.display_name),
                    )
                    .await?;
                }
            }
        }

        Ok(concept_names)
    }

    /// Process a file event from the directory watcher.
    pub async fn process_file_event(&self, event: FileEvent) -> Result<()> {
        debug!("Processing file event: {:?} for {:?}", event.kind, event.path);

        // Extract concepts from the file path
        let path_str = event.path.to_string_lossy();
        let concepts = self.extract_and_store(&path_str, false).await?;

        if !concepts.is_empty() {
            debug!("Found concepts in path: {:?}", concepts);
        }

        Ok(())
    }

    /// Add a directory to watch.
    pub async fn add_watch_dir(&self, path: impl AsRef<Path>) -> Result<()> {
        let config = DirectoryConfig::new(path.as_ref());
        self.watcher.write().await.add(config).await?;
        Ok(())
    }

    /// Get engine statistics.
    pub async fn stats(&self) -> EngineStats {
        let context_count = self.context_store.read().await.list_concepts().len();
        let concept_count = self.concept_index.read().await.list().len();
        let embedding_count = self.similarity_index.read().await.len();
        let watcher_stats = self.watcher.read().await.stats().await;

        EngineStats {
            context_files: context_count,
            concepts_indexed: concept_count,
            embeddings_stored: embedding_count,
            watched_directories: watcher_stats.total_directories,
            realtime_watches: watcher_stats.realtime_watches,
        }
    }
}

/// Builder for unified retrieval engine.
pub struct UnifiedRetrievalBuilder {
    config: RetrievalConfig,
}

impl UnifiedRetrievalBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            config: RetrievalConfig::default(),
        }
    }

    /// Set the context directory.
    pub fn with_context_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.config.context_dir = dir.into();
        self
    }

    /// Add a directory to watch.
    pub fn with_watch_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.config.watch_dirs.push(dir.into());
        self
    }

    /// Set the embedding provider.
    pub fn with_embedding_provider(mut self, provider: EmbeddingProviderType) -> Self {
        self.config.embedding.provider = provider;
        self
    }

    /// Enable or disable realtime watching.
    pub fn with_realtime_watch(mut self, enabled: bool) -> Self {
        self.config.sync.realtime_watch = enabled;
        self
    }

    /// Build the engine.
    pub async fn build(self) -> Result<UnifiedRetrieval> {
        UnifiedRetrieval::new(self.config).await
    }
}

impl Default for UnifiedRetrievalBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the retrieval engine.
#[derive(Debug, Clone)]
pub struct EngineStats {
    /// Number of context files.
    pub context_files: usize,

    /// Number of concepts indexed.
    pub concepts_indexed: usize,

    /// Number of embeddings stored.
    pub embeddings_stored: usize,

    /// Number of watched directories.
    pub watched_directories: usize,

    /// Number of realtime watches.
    pub realtime_watches: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_engine_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = RetrievalConfig::new(temp_dir.path());

        let engine = UnifiedRetrieval::new(config).await.unwrap();
        assert!(engine.initialized);
    }

    #[tokio::test]
    async fn test_builder_pattern() {
        let temp_dir = TempDir::new().unwrap();

        let engine = UnifiedRetrieval::builder()
            .with_context_dir(temp_dir.path())
            .with_embedding_provider(EmbeddingProviderType::None)
            .with_realtime_watch(false)
            .build()
            .await
            .unwrap();

        assert!(engine.initialized);
    }
}
