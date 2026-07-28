//! Similarity index for fast embedding lookups.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::Embedding;
use crate::error::{EmbeddingError, Result};
use crate::similarity::{SimilarityResult, cosine_similarity, find_top_k, normalize};

/// An entry in the similarity index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Unique identifier.
    pub id: String,

    /// The embedding vector (normalized).
    pub embedding: Embedding,

    /// Associated metadata.
    pub metadata: Option<serde_json::Value>,
}

/// A similarity index for fast vector lookups.
///
/// The index stores embeddings and supports efficient similarity search
/// using cosine similarity.
pub struct SimilarityIndex {
    /// Stored entries.
    entries: HashMap<String, IndexEntry>,

    /// Expected dimension of embeddings.
    dimension: usize,

    /// Whether embeddings should be normalized.
    normalize_embeddings: bool,
}

impl SimilarityIndex {
    /// Create a new similarity index.
    pub fn new(dimension: usize) -> Self {
        Self {
            entries: HashMap::new(),
            dimension,
            normalize_embeddings: true,
        }
    }

    /// Disable embedding normalization.
    pub fn without_normalization(mut self) -> Self {
        self.normalize_embeddings = false;
        self
    }

    /// Add an embedding to the index.
    pub fn add(
        &mut self,
        id: impl Into<String>,
        mut embedding: Embedding,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        let id = id.into();

        if embedding.len() != self.dimension {
            return Err(EmbeddingError::DimensionMismatch {
                expected: self.dimension,
                actual: embedding.len(),
            });
        }

        if self.normalize_embeddings {
            normalize(&mut embedding);
        }

        let entry = IndexEntry {
            id: id.clone(),
            embedding,
            metadata,
        };

        self.entries.insert(id.clone(), entry);
        debug!("Added embedding to index: {id}");

        Ok(())
    }

    /// Remove an embedding from the index.
    pub fn remove(&mut self, id: &str) -> Option<IndexEntry> {
        self.entries.remove(id)
    }

    /// Get an embedding by ID.
    pub fn get(&self, id: &str) -> Option<&IndexEntry> {
        self.entries.get(id)
    }

    /// Check if an ID exists in the index.
    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    /// Get the number of entries in the index.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Search for similar embeddings.
    pub fn search(
        &self,
        query: &Embedding,
        k: usize,
        min_score: f32,
    ) -> Result<Vec<SimilarityResult>> {
        if query.len() != self.dimension {
            return Err(EmbeddingError::DimensionMismatch {
                expected: self.dimension,
                actual: query.len(),
            });
        }

        let mut query = query.clone();
        if self.normalize_embeddings {
            normalize(&mut query);
        }

        let candidates: Vec<(String, Embedding)> = self
            .entries
            .values()
            .map(|e| (e.id.clone(), e.embedding.clone()))
            .collect();

        let mut results = find_top_k(&query, &candidates, k, min_score)?;

        // Add metadata to results
        for result in &mut results {
            if let Some(entry) = self.entries.get(&result.id) {
                result.metadata = entry.metadata.clone();
            }
        }

        Ok(results)
    }

    /// Search for the single most similar embedding.
    pub fn search_one(
        &self,
        query: &Embedding,
        min_score: f32,
    ) -> Result<Option<SimilarityResult>> {
        let results = self.search(query, 1, min_score)?;
        Ok(results.into_iter().next())
    }

    /// Compute similarity between two IDs in the index.
    pub fn similarity(&self, id1: &str, id2: &str) -> Result<f32> {
        let entry1 = self
            .entries
            .get(id1)
            .ok_or_else(|| EmbeddingError::Cache(format!("Entry not found: {id1}")))?;
        let entry2 = self
            .entries
            .get(id2)
            .ok_or_else(|| EmbeddingError::Cache(format!("Entry not found: {id2}")))?;

        cosine_similarity(&entry1.embedding, &entry2.embedding)
    }

    /// Get all IDs in the index.
    pub fn ids(&self) -> Vec<&str> {
        self.entries.keys().map(String::as_str).collect()
    }

    /// Clear the index.
    pub fn clear(&mut self) {
        self.entries.clear();
        info!("Cleared similarity index");
    }

    /// Serialize the index to JSON.
    pub fn to_json(&self) -> Result<String> {
        let entries: Vec<&IndexEntry> = self.entries.values().collect();
        Ok(serde_json::to_string(&entries)?)
    }

    /// Load index from JSON.
    pub fn from_json(json: &str, dimension: usize) -> Result<Self> {
        let entries: Vec<IndexEntry> = serde_json::from_str(json)?;

        let mut index = Self::new(dimension);
        for entry in entries {
            if entry.embedding.len() != dimension {
                return Err(EmbeddingError::DimensionMismatch {
                    expected: dimension,
                    actual: entry.embedding.len(),
                });
            }
            index.entries.insert(entry.id.clone(), entry);
        }

        info!("Loaded {} entries into similarity index", index.len());
        Ok(index)
    }

    /// Merge another index into this one.
    pub fn merge(&mut self, other: SimilarityIndex) -> Result<()> {
        if other.dimension != self.dimension {
            return Err(EmbeddingError::DimensionMismatch {
                expected: self.dimension,
                actual: other.dimension,
            });
        }

        let count = other.entries.len();
        for (id, entry) in other.entries {
            self.entries.insert(id, entry);
        }

        info!("Merged {count} entries into similarity index");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_index_add_and_get() {
        let mut index = SimilarityIndex::new(3);
        index.add("item1", vec![1.0, 0.0, 0.0], None).unwrap();

        assert!(index.contains("item1"));
        assert!(!index.contains("item2"));
    }

    #[test]
    fn test_index_search() {
        let mut index = SimilarityIndex::new(3);
        index.add("a", vec![1.0, 0.0, 0.0], None).unwrap();
        index.add("b", vec![0.0, 1.0, 0.0], None).unwrap();
        index.add("c", vec![0.7, 0.7, 0.0], None).unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = index.search(&query, 2, 0.0).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a");
    }

    #[test]
    fn test_dimension_mismatch() {
        let mut index = SimilarityIndex::new(3);
        let result = index.add("bad", vec![1.0, 0.0], None);
        assert!(result.is_err());
    }
}
