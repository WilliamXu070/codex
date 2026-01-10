//! Query processing for context retrieval.
//!
//! The query module handles parsing natural language queries,
//! identifying intent, and mapping queries to relevant concepts.

use serde::{Deserialize, Serialize};

use crate::context_file::ContextFile;
use crate::error::Result;

/// A parsed query with identified intent and concepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    /// The original query text.
    pub text: String,

    /// Identified query intent.
    pub intent: QueryIntent,

    /// Concepts identified in the query.
    pub concepts: Vec<String>,

    /// Keywords extracted from the query.
    pub keywords: Vec<String>,

    /// Filters to apply to results.
    pub filters: QueryFilters,
}

impl Query {
    /// Parse a natural language query.
    pub fn parse(text: impl Into<String>) -> Self {
        let text = text.into();
        let text_lower = text.to_lowercase();

        // Identify intent
        let intent = Self::identify_intent(&text_lower);

        // Extract keywords
        let keywords = Self::extract_keywords(&text_lower);

        Self {
            text,
            intent,
            concepts: Vec::new(), // Populated by ConceptExtractor
            keywords,
            filters: QueryFilters::default(),
        }
    }

    /// Add identified concepts to the query.
    pub fn with_concepts(mut self, concepts: Vec<String>) -> Self {
        self.concepts = concepts;
        self
    }

    /// Add filters to the query.
    pub fn with_filters(mut self, filters: QueryFilters) -> Self {
        self.filters = filters;
        self
    }

    /// Identify the query intent.
    fn identify_intent(text: &str) -> QueryIntent {
        // Simple pattern matching for intent detection
        if text.starts_with("what") || text.starts_with("who") || text.starts_with("when") {
            QueryIntent::FactualLookup
        } else if text.contains("help me") || text.contains("create") || text.contains("write") {
            QueryIntent::ContentGeneration
        } else if text.contains("summarize") || text.contains("summary") {
            QueryIntent::Summarization
        } else if text.contains("find") || text.contains("search") || text.contains("look for") {
            QueryIntent::Search
        } else if text.contains("update") || text.contains("change") || text.contains("modify") {
            QueryIntent::Update
        } else if text.contains("remember") || text.contains("note") || text.contains("save") {
            QueryIntent::Store
        } else if text.contains("compare") || text.contains("difference") {
            QueryIntent::Comparison
        } else {
            QueryIntent::General
        }
    }

    /// Extract keywords from text.
    fn extract_keywords(text: &str) -> Vec<String> {
        // Stop words to filter out
        let stop_words: std::collections::HashSet<&str> = [
            "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "must",
            "shall", "can", "need", "dare", "ought", "used", "to", "of", "in", "for", "on", "with",
            "at", "by", "from", "as", "into", "through", "during", "before", "after", "above",
            "below", "between", "under", "again", "further", "then", "once", "here", "there",
            "when", "where", "why", "how", "all", "each", "few", "more", "most", "other", "some",
            "such", "no", "nor", "not", "only", "own", "same", "so", "than", "too", "very", "just",
            "and", "but", "if", "or", "because", "until", "while", "although", "though", "what",
            "which", "who", "whom", "this", "that", "these", "those", "am", "i", "my", "me", "we",
            "our", "you", "your", "he", "she", "it", "they", "them", "his", "her", "its", "their",
            "help", "please", "tell", "about",
        ]
        .into_iter()
        .collect();

        text.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .filter(|word| word.len() >= 2 && !stop_words.contains(word))
            .map(String::from)
            .collect()
    }

    /// Check if this is a simple single-concept query.
    pub fn is_simple(&self) -> bool {
        self.concepts.len() <= 1 && matches!(self.intent, QueryIntent::FactualLookup)
    }

    /// Check if this is a complex multi-concept query.
    pub fn is_complex(&self) -> bool {
        self.concepts.len() > 1
            || matches!(
                self.intent,
                QueryIntent::ContentGeneration | QueryIntent::Summarization
            )
    }
}

/// The identified intent of a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryIntent {
    /// Looking up a specific fact.
    FactualLookup,

    /// Searching for information.
    Search,

    /// Generating content (resume, document, etc.).
    ContentGeneration,

    /// Summarizing information.
    Summarization,

    /// Updating existing information.
    Update,

    /// Storing new information.
    Store,

    /// Comparing concepts or information.
    Comparison,

    /// General query that doesn't fit other categories.
    General,
}

/// Filters to apply to query results.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFilters {
    /// Filter by date range (start).
    pub date_from: Option<chrono::DateTime<chrono::Utc>>,

    /// Filter by date range (end).
    pub date_to: Option<chrono::DateTime<chrono::Utc>>,

    /// Filter by tags.
    pub tags: Vec<String>,

    /// Filter by categories.
    pub categories: Vec<String>,

    /// Maximum number of results.
    pub limit: Option<usize>,

    /// Minimum relevance score.
    pub min_relevance: Option<f32>,
}

/// The result of a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// The original query.
    pub query: Query,

    /// Retrieved context files with relevance scores.
    pub results: Vec<ScoredResult>,

    /// Time taken to process the query (in milliseconds).
    pub processing_time_ms: u64,

    /// Whether the results were truncated.
    pub truncated: bool,
}

impl QueryResult {
    /// Create a new query result.
    pub fn new(query: Query, results: Vec<ScoredResult>, processing_time_ms: u64) -> Self {
        Self {
            query,
            results,
            processing_time_ms,
            truncated: false,
        }
    }

    /// Get the top result if any.
    pub fn top(&self) -> Option<&ScoredResult> {
        self.results.first()
    }

    /// Get results above a certain relevance threshold.
    pub fn above_threshold(&self, threshold: f32) -> Vec<&ScoredResult> {
        self.results
            .iter()
            .filter(|r| r.relevance >= threshold)
            .collect()
    }
}

/// A context file with a relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredResult {
    /// The context file ID.
    pub context_id: String,

    /// The concept name.
    pub concept: String,

    /// Relevance score (0.0 to 1.0).
    pub relevance: f32,

    /// Why this result was included.
    pub match_reason: MatchReason,

    /// Excerpt from the context file.
    pub excerpt: Option<String>,
}

impl ScoredResult {
    /// Create a new scored result.
    pub fn new(context: &ContextFile, relevance: f32, match_reason: MatchReason) -> Self {
        Self {
            context_id: context.id.clone(),
            concept: context.concept.clone(),
            relevance,
            match_reason,
            excerpt: None,
        }
    }

    /// Add an excerpt to the result.
    pub fn with_excerpt(mut self, excerpt: impl Into<String>) -> Self {
        self.excerpt = Some(excerpt.into());
        self
    }
}

/// Reason why a result matched the query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchReason {
    /// Exact concept name match.
    ExactMatch,

    /// Keyword match in content.
    KeywordMatch { keywords: Vec<String> },

    /// Semantic similarity match.
    SemanticMatch { similarity: f32 },

    /// Tag match.
    TagMatch { tags: Vec<String> },

    /// Related concept match.
    RelatedMatch { via_concept: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_query_parse_factual() {
        let query = Query::parse("What is Sarah's birthday?");
        assert_eq!(query.intent, QueryIntent::FactualLookup);
    }

    #[test]
    fn test_query_parse_generation() {
        let query = Query::parse("Help me write a resume");
        assert_eq!(query.intent, QueryIntent::ContentGeneration);
    }

    #[test]
    fn test_keyword_extraction() {
        let query = Query::parse("Tell me about my projects and research");
        assert!(query.keywords.contains(&"projects".to_string()));
        assert!(query.keywords.contains(&"research".to_string()));
    }
}
