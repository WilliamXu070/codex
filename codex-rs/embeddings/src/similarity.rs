//! Similarity computation for embeddings.

use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

use crate::Embedding;
use crate::error::{EmbeddingError, Result};

/// Compute the cosine similarity between two embeddings.
///
/// Returns a value between -1.0 and 1.0, where:
/// - 1.0 means identical vectors
/// - 0.0 means orthogonal vectors
/// - -1.0 means opposite vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f32> {
    if a.len() != b.len() {
        return Err(EmbeddingError::DimensionMismatch {
            expected: a.len(),
            actual: b.len(),
        });
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return Ok(0.0);
    }

    Ok(dot_product / (magnitude_a * magnitude_b))
}

/// Compute the euclidean distance between two embeddings.
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> Result<f32> {
    if a.len() != b.len() {
        return Err(EmbeddingError::DimensionMismatch {
            expected: a.len(),
            actual: b.len(),
        });
    }

    let sum: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum();

    Ok(sum.sqrt())
}

/// Compute the dot product between two embeddings.
pub fn dot_product(a: &[f32], b: &[f32]) -> Result<f32> {
    if a.len() != b.len() {
        return Err(EmbeddingError::DimensionMismatch {
            expected: a.len(),
            actual: b.len(),
        });
    }

    Ok(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum())
}

/// A similarity search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityResult {
    /// ID of the matched item.
    pub id: String,

    /// Similarity score.
    pub score: f32,

    /// Additional metadata.
    pub metadata: Option<serde_json::Value>,
}

impl SimilarityResult {
    /// Create a new similarity result.
    pub fn new(id: impl Into<String>, score: f32) -> Self {
        Self {
            id: id.into(),
            score,
            metadata: None,
        }
    }

    /// Add metadata to the result.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Find the top-k most similar embeddings.
pub fn find_top_k(
    query: &Embedding,
    candidates: &[(String, Embedding)],
    k: usize,
    min_score: f32,
) -> Result<Vec<SimilarityResult>> {
    let mut scores: Vec<(OrderedFloat<f32>, String)> = Vec::with_capacity(candidates.len());

    for (id, embedding) in candidates {
        let score = cosine_similarity(query, embedding)?;
        if score >= min_score {
            scores.push((OrderedFloat(score), id.clone()));
        }
    }

    // Sort by score descending
    scores.sort_by(|a, b| b.0.cmp(&a.0));

    // Take top k
    let results: Vec<SimilarityResult> = scores
        .into_iter()
        .take(k)
        .map(|(score, id)| SimilarityResult::new(id, score.0))
        .collect();

    Ok(results)
}

/// Normalize an embedding to unit length.
pub fn normalize(embedding: &mut Embedding) {
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for x in embedding.iter_mut() {
            *x /= magnitude;
        }
    }
}

/// Compute the average of multiple embeddings.
pub fn average(embeddings: &[Embedding]) -> Result<Embedding> {
    if embeddings.is_empty() {
        return Ok(Vec::new());
    }

    let dim = embeddings[0].len();
    for e in embeddings {
        if e.len() != dim {
            return Err(EmbeddingError::DimensionMismatch {
                expected: dim,
                actual: e.len(),
            });
        }
    }

    let n = embeddings.len() as f32;
    let mut result = vec![0.0f32; dim];

    for embedding in embeddings {
        for (i, val) in embedding.iter().enumerate() {
            result[i] += val / n;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!((sim - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!((sim - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_dimension_mismatch() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!(cosine_similarity(&a, &b).is_err());
    }

    #[test]
    fn test_normalize() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);
        assert!((v[0] - 0.6).abs() < 1e-6);
        assert!((v[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_find_top_k() {
        let query = vec![1.0, 0.0, 0.0];
        let candidates = vec![
            ("a".to_string(), vec![1.0, 0.0, 0.0]), // similarity 1.0
            ("b".to_string(), vec![0.0, 1.0, 0.0]), // similarity 0.0
            ("c".to_string(), vec![0.7, 0.7, 0.0]), // similarity ~0.7
        ];

        let results = find_top_k(&query, &candidates, 2, 0.0).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a");
        assert_eq!(results[1].id, "c");
    }
}
