//! Embedding cache for efficient retrieval.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::error::{EmbeddingError, Result};
use crate::Embedding;

/// Cache entry for an embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The text that was embedded.
    pub text_hash: String,

    /// The embedding vector.
    pub embedding: Embedding,

    /// Model used to generate the embedding.
    pub model: String,

    /// When the entry was created.
    pub created_at: u64,
}

/// Cache for embeddings to avoid redundant API calls.
pub struct EmbeddingCache {
    /// In-memory cache.
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,

    /// Path for persistent cache storage.
    cache_path: Option<PathBuf>,

    /// Maximum cache size.
    max_entries: usize,
}

impl EmbeddingCache {
    /// Create a new in-memory cache.
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_path: None,
            max_entries,
        }
    }

    /// Create a cache with persistent storage.
    pub async fn with_persistence(path: impl AsRef<Path>, max_entries: usize) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        let mut cache = Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_path: Some(path.clone()),
            max_entries,
        };

        // Load existing cache
        if path.exists() {
            cache.load().await?;
        }

        Ok(cache)
    }

    /// Compute a hash for cache lookup.
    fn hash_key(text: &str, model: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        model.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get an embedding from the cache.
    pub async fn get(&self, text: &str, model: &str) -> Option<Embedding> {
        let key = Self::hash_key(text, model);
        let cache = self.cache.read().await;
        cache.get(&key).map(|e| e.embedding.clone())
    }

    /// Put an embedding in the cache.
    pub async fn put(&self, text: &str, model: &str, embedding: Embedding) -> Result<()> {
        let key = Self::hash_key(text, model);
        let entry = CacheEntry {
            text_hash: key.clone(),
            embedding,
            model: model.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let mut cache = self.cache.write().await;

        // Evict if at capacity
        if cache.len() >= self.max_entries {
            // Remove oldest entry
            if let Some(oldest_key) = cache
                .iter()
                .min_by_key(|(_, v)| v.created_at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
            }
        }

        cache.insert(key, entry);
        debug!("Cached embedding for text (model: {model})");

        // Persist if enabled
        if self.cache_path.is_some() {
            drop(cache); // Release lock before I/O
            self.save().await?;
        }

        Ok(())
    }

    /// Check if an embedding is cached.
    pub async fn contains(&self, text: &str, model: &str) -> bool {
        let key = Self::hash_key(text, model);
        self.cache.read().await.contains_key(&key)
    }

    /// Remove an embedding from the cache.
    pub async fn remove(&self, text: &str, model: &str) {
        let key = Self::hash_key(text, model);
        self.cache.write().await.remove(&key);
    }

    /// Clear the entire cache.
    pub async fn clear(&self) {
        self.cache.write().await.clear();
        info!("Cleared embedding cache");
    }

    /// Get cache statistics.
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        CacheStats {
            entries: cache.len(),
            max_entries: self.max_entries,
            models: cache
                .values()
                .map(|e| e.model.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect(),
        }
    }

    /// Save cache to disk.
    async fn save(&self) -> Result<()> {
        if let Some(ref path) = self.cache_path {
            let cache = self.cache.read().await;
            let entries: Vec<&CacheEntry> = cache.values().collect();
            let content = serde_json::to_string(&entries)?;

            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await?;
            }

            fs::write(path, content).await?;
            debug!("Saved {} cache entries to disk", entries.len());
        }
        Ok(())
    }

    /// Load cache from disk.
    async fn load(&self) -> Result<()> {
        if let Some(ref path) = self.cache_path {
            if path.exists() {
                let content = fs::read_to_string(path).await?;
                let entries: Vec<CacheEntry> = serde_json::from_str(&content)?;

                let mut cache = self.cache.write().await;
                for entry in entries {
                    cache.insert(entry.text_hash.clone(), entry);
                }

                info!("Loaded {} cache entries from disk", cache.len());
            }
        }
        Ok(())
    }
}

/// Statistics about the embedding cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Number of entries in cache.
    pub entries: usize,

    /// Maximum cache size.
    pub max_entries: usize,

    /// Models with cached embeddings.
    pub models: Vec<String>,
}

/// A wrapper that provides cached embedding generation.
pub struct CachedProvider<P> {
    provider: P,
    cache: EmbeddingCache,
}

impl<P> CachedProvider<P>
where
    P: crate::provider::EmbeddingProvider,
{
    /// Create a new cached provider.
    pub fn new(provider: P, cache: EmbeddingCache) -> Self {
        Self { provider, cache }
    }

    /// Generate an embedding, using cache if available.
    pub async fn embed(
        &self,
        request: crate::provider::EmbeddingRequest,
    ) -> Result<crate::provider::EmbeddingResponse> {
        let model = request
            .model
            .as_ref()
            .unwrap_or(&self.provider.default_model().to_string())
            .clone();

        // Check cache
        if let Some(embedding) = self.cache.get(&request.text, &model).await {
            debug!("Cache hit for embedding");
            return Ok(crate::provider::EmbeddingResponse {
                embedding: embedding.clone(),
                model: model.clone(),
                dimension: embedding.len(),
                tokens_used: None,
            });
        }

        // Generate and cache
        let response = self.provider.embed(request.clone()).await?;
        self.cache
            .put(&request.text, &model, response.embedding.clone())
            .await?;

        Ok(response)
    }

    /// Get the underlying cache.
    pub fn cache(&self) -> &EmbeddingCache {
        &self.cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_put_get() {
        let cache = EmbeddingCache::new(100);
        let embedding = vec![1.0, 2.0, 3.0];

        cache
            .put("hello", "model-1", embedding.clone())
            .await
            .unwrap();

        let retrieved = cache.get("hello", "model-1").await;
        assert_eq!(retrieved, Some(embedding));
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = EmbeddingCache::new(100);
        let result = cache.get("not cached", "model-1").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = EmbeddingCache::new(2);

        cache.put("a", "model", vec![1.0]).await.unwrap();
        cache.put("b", "model", vec![2.0]).await.unwrap();
        cache.put("c", "model", vec![3.0]).await.unwrap();

        // One entry should have been evicted
        let stats = cache.stats().await;
        assert_eq!(stats.entries, 2);
    }
}
