//! Retrieval engine for context files.
//!
//! The `RetrievalEngine` combines keyword search, semantic similarity,
//! and concept relationships to find the most relevant context files.

use std::collections::HashMap;
use std::time::Instant;

use tracing::{debug, info};

use crate::context_file::ContextFile;
use crate::error::Result;
use crate::extraction::ConceptExtractor;
use crate::index::ConceptIndex;
use crate::query::{MatchReason, Query, QueryResult, ScoredResult};
use crate::storage::ContextStore;

/// Configuration for the retrieval engine.
#[derive(Debug, Clone)]
pub struct RetrievalConfig {
    /// Weight for keyword matching (0.0 to 1.0).
    pub keyword_weight: f32,

    /// Weight for semantic similarity (0.0 to 1.0).
    pub semantic_weight: f32,

    /// Weight for related concepts (0.0 to 1.0).
    pub relation_weight: f32,

    /// Minimum relevance score to include in results.
    pub min_relevance: f32,

    /// Maximum number of results to return.
    pub max_results: usize,

    /// Whether to expand query to related concepts.
    pub expand_related: bool,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            keyword_weight: 0.4,
            semantic_weight: 0.4,
            relation_weight: 0.2,
            min_relevance: 0.3,
            max_results: 10,
            expand_related: true,
        }
    }
}

/// The main retrieval engine for context files.
///
/// The engine supports multiple retrieval strategies:
/// 1. **Keyword matching**: Fast lookup using the concept index
/// 2. **Semantic similarity**: Embedding-based similarity search
/// 3. **Relationship expansion**: Finding related concepts
///
/// Results are combined using a weighted scoring system.
pub struct RetrievalEngine {
    config: RetrievalConfig,
    extractor: ConceptExtractor,
}

impl RetrievalEngine {
    /// Create a new retrieval engine with the given configuration.
    pub fn new(config: RetrievalConfig) -> Self {
        Self {
            config,
            extractor: ConceptExtractor::with_defaults(),
        }
    }

    /// Create a retrieval engine with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(RetrievalConfig::default())
    }

    /// Retrieve relevant context files for a query.
    pub fn retrieve(
        &self,
        query_text: &str,
        store: &ContextStore,
        index: &ConceptIndex,
    ) -> Result<QueryResult> {
        let start = Instant::now();

        // Parse the query
        let mut query = Query::parse(query_text);

        // Extract concepts from the query
        let extracted = self.extractor.extract(query_text)?;
        query.concepts = extracted.iter().map(|e| e.concept.name.clone()).collect();

        // Score all context files
        let mut scores: HashMap<String, ScoredResult> = HashMap::new();

        // Phase 1: Direct concept matches
        for concept_name in &query.concepts {
            if let Some(cf) = store.get(concept_name) {
                let result = ScoredResult::new(cf, 1.0, MatchReason::ExactMatch);
                scores.insert(cf.id.clone(), result);
            }
        }

        // Phase 2: Keyword matches
        for keyword in &query.keywords {
            let matches = index.find_by_keyword(keyword);
            for concept in matches {
                if let Some(cf) = store.get(&concept.name) {
                    let relevance = self.config.keyword_weight;
                    scores
                        .entry(cf.id.clone())
                        .and_modify(|r| {
                            r.relevance = (r.relevance + relevance).min(1.0);
                        })
                        .or_insert_with(|| {
                            ScoredResult::new(
                                cf,
                                relevance,
                                MatchReason::KeywordMatch {
                                    keywords: vec![keyword.clone()],
                                },
                            )
                        });
                }
            }
        }

        // Phase 3: Related concept expansion
        if self.config.expand_related {
            let direct_concepts: Vec<_> = scores.keys().cloned().collect();
            let mut new_entries = Vec::new();

            for id in direct_concepts {
                if let Some(result) = scores.get(&id) {
                    let via_concept = result.concept.clone();
                    let related = index.get_related(&result.concept);
                    for (related_concept, _relation) in related {
                        if let Some(cf) = store.get(&related_concept.name) {
                            if !scores.contains_key(&cf.id) {
                                let relevance = self.config.relation_weight;
                                new_entries.push((
                                    cf.id.clone(),
                                    ScoredResult::new(
                                        cf,
                                        relevance,
                                        MatchReason::RelatedMatch {
                                            via_concept: via_concept.clone(),
                                        },
                                    ),
                                ));
                            }
                        }
                    }
                }
            }

            // Insert all new entries after the loop
            for (id, result) in new_entries {
                scores.insert(id, result);
            }
        }

        // Phase 4: Semantic similarity (if embeddings available)
        // This is a placeholder - actual implementation would use the embeddings crate
        // self.semantic_search(&query, store, &mut scores)?;

        // Filter and sort results
        let mut results: Vec<ScoredResult> = scores
            .into_values()
            .filter(|r| r.relevance >= self.config.min_relevance)
            .collect();

        results.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let truncated = results.len() > self.config.max_results;
        results.truncate(self.config.max_results);

        let processing_time_ms = start.elapsed().as_millis() as u64;

        debug!(
            "Retrieved {} results for query in {}ms",
            results.len(),
            processing_time_ms
        );

        let mut result = QueryResult::new(query, results, processing_time_ms);
        result.truncated = truncated;

        Ok(result)
    }

    /// Perform semantic similarity search.
    ///
    /// This requires embeddings to be generated for both the query
    /// and the context files.
    #[allow(dead_code)]
    fn semantic_search(
        &self,
        query: &Query,
        store: &ContextStore,
        scores: &mut HashMap<String, ScoredResult>,
    ) -> Result<()> {
        // TODO: Implement using codex-embeddings crate
        // 1. Generate embedding for query text
        // 2. Compare with embeddings in context files
        // 3. Add matches above threshold to scores

        info!(
            "Semantic search not yet implemented for query: {}",
            query.text
        );
        Ok(())
    }

    /// Retrieve context for a simple factual query.
    ///
    /// Optimized path for queries like "What is X's birthday?"
    pub fn retrieve_simple<'a>(
        &self,
        concept: &str,
        store: &'a ContextStore,
    ) -> Result<Option<&'a ContextFile>> {
        Ok(store.get(concept))
    }

    /// Retrieve context for multiple concepts.
    ///
    /// Used for complex queries that need information from multiple sources.
    pub fn retrieve_multi<'a>(
        &self,
        concepts: &[&str],
        store: &'a ContextStore,
    ) -> Vec<&'a ContextFile> {
        concepts.iter().filter_map(|c| store.get(c)).collect()
    }
}

/// A builder for creating queries with filters.
pub struct QueryBuilder {
    text: String,
    concepts: Vec<String>,
    tags: Vec<String>,
    min_relevance: Option<f32>,
    limit: Option<usize>,
}

impl QueryBuilder {
    /// Create a new query builder.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            concepts: Vec::new(),
            tags: Vec::new(),
            min_relevance: None,
            limit: None,
        }
    }

    /// Add a concept filter.
    pub fn with_concept(mut self, concept: impl Into<String>) -> Self {
        self.concepts.push(concept.into());
        self
    }

    /// Add a tag filter.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set minimum relevance threshold.
    pub fn min_relevance(mut self, threshold: f32) -> Self {
        self.min_relevance = Some(threshold);
        self
    }

    /// Set maximum number of results.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Build the query.
    pub fn build(self) -> Query {
        let mut query = Query::parse(self.text);
        query.concepts = self.concepts;
        query.filters.tags = self.tags;
        query.filters.min_relevance = self.min_relevance;
        query.filters.limit = self.limit;
        query
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retrieval_engine_creation() {
        let engine = RetrievalEngine::with_defaults();
        assert_eq!(engine.config.max_results, 10);
    }

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::new("Find my projects")
            .with_concept("projects")
            .with_tag("active")
            .min_relevance(0.5)
            .limit(5)
            .build();

        assert_eq!(query.concepts, vec!["projects"]);
        assert_eq!(query.filters.tags, vec!["active"]);
        assert_eq!(query.filters.min_relevance, Some(0.5));
        assert_eq!(query.filters.limit, Some(5));
    }
}
