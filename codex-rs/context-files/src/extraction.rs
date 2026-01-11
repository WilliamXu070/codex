//! Concept extraction from text and conversations.
//!
//! The `ConceptExtractor` identifies key concepts from user input,
//! conversations, and file contents.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::concept::Concept;
use crate::error::Result;

/// Configuration for concept extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Minimum confidence threshold for extracted concepts (0.0 to 1.0).
    pub min_confidence: f32,

    /// Maximum number of concepts to extract from a single text.
    pub max_concepts: usize,

    /// Whether to use semantic analysis (requires embeddings).
    pub use_semantic: bool,

    /// Known concept names to match against.
    pub known_concepts: HashSet<String>,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            max_concepts: 10,
            use_semantic: true,
            known_concepts: HashSet::new(),
        }
    }
}

/// An extracted concept with confidence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedConcept {
    /// The concept that was extracted.
    pub concept: Concept,

    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,

    /// The text span that triggered this extraction.
    pub source_span: Option<String>,

    /// Whether this is a known concept or newly discovered.
    pub is_known: bool,
}

/// Extracts concepts from text and conversations.
///
/// The extractor uses a combination of:
/// - Pattern matching against known concepts
/// - Keyword extraction
/// - Semantic analysis (when embeddings are available)
pub struct ConceptExtractor {
    config: ExtractionConfig,
}

impl ConceptExtractor {
    /// Create a new concept extractor with the given configuration.
    pub fn new(config: ExtractionConfig) -> Self {
        Self { config }
    }

    /// Create a concept extractor with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ExtractionConfig::default())
    }

    /// Add a known concept for matching.
    pub fn add_known_concept(&mut self, concept: impl Into<String>) {
        self.config.known_concepts.insert(concept.into());
    }

    /// Extract concepts from text.
    ///
    /// This method identifies relevant concepts mentioned in the text,
    /// matching against known concepts and discovering new ones.
    pub fn extract(&self, text: &str) -> Result<Vec<ExtractedConcept>> {
        let mut extracted = Vec::new();
        let text_lower = text.to_lowercase();

        // Phase 1: Match known concepts
        for known in &self.config.known_concepts {
            if text_lower.contains(&known.to_lowercase()) {
                extracted.push(ExtractedConcept {
                    concept: Concept::new(known),
                    confidence: 0.9,
                    source_span: Some(known.clone()),
                    is_known: true,
                });
            }
        }

        // Phase 2: Extract potential new concepts using heuristics
        let potential = self.extract_potential_concepts(text);
        for (name, confidence) in potential {
            if !self.config.known_concepts.contains(&name)
                && confidence >= self.config.min_confidence
            {
                extracted.push(ExtractedConcept {
                    concept: Concept::new(&name),
                    confidence,
                    source_span: Some(name.clone()),
                    is_known: false,
                });
            }
        }

        // Sort by confidence and limit
        extracted.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        extracted.truncate(self.config.max_concepts);

        debug!("Extracted {} concepts from text", extracted.len());
        Ok(extracted)
    }

    /// Extract potential concepts using heuristic patterns.
    ///
    /// This is a placeholder for more sophisticated NLP/ML-based extraction.
    fn extract_potential_concepts(&self, text: &str) -> Vec<(String, f32)> {
        let mut concepts = Vec::new();

        // Common concept indicators
        let indicators = [
            ("my ", 0.7),
            ("about ", 0.6),
            ("regarding ", 0.6),
            ("for ", 0.5),
            ("related to ", 0.8),
        ];

        // Extract noun phrases after indicators
        for (indicator, base_confidence) in indicators {
            if let Some(idx) = text.to_lowercase().find(indicator) {
                let start = idx + indicator.len();
                let rest = &text[start..];
                if let Some(end) =
                    rest.find(|c: char| c == '.' || c == ',' || c == '?' || c == '\n')
                {
                    let phrase = rest[..end].trim();
                    if phrase.len() >= 2 && phrase.len() <= 50 {
                        let normalized = Self::normalize_concept_name(phrase);
                        concepts.push((normalized, base_confidence));
                    }
                }
            }
        }

        // Extract capitalized multi-word phrases (potential named entities)
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut i = 0;
        while i < words.len() {
            if words[i].chars().next().map_or(false, |c| c.is_uppercase()) {
                let mut phrase = vec![words[i]];
                let mut j = i + 1;
                while j < words.len() && words[j].chars().next().map_or(false, |c| c.is_uppercase())
                {
                    phrase.push(words[j]);
                    j += 1;
                }
                if phrase.len() >= 2 {
                    let name = Self::normalize_concept_name(&phrase.join(" "));
                    concepts.push((name, 0.6));
                    i = j;
                    continue;
                }
            }
            i += 1;
        }

        concepts
    }

    /// Normalize a concept name (lowercase, replace spaces with hyphens).
    fn normalize_concept_name(name: &str) -> String {
        name.to_lowercase()
            .trim()
            .replace(' ', "-")
            .replace('_', "-")
    }

    /// Extract concepts from a conversation turn.
    ///
    /// This method is optimized for extracting concepts from user messages
    /// and assistant responses in a conversation context.
    pub fn extract_from_conversation(
        &self,
        user_message: &str,
        assistant_response: Option<&str>,
    ) -> Result<Vec<ExtractedConcept>> {
        let mut all_concepts = self.extract(user_message)?;

        if let Some(response) = assistant_response {
            let response_concepts = self.extract(response)?;
            // Merge, preferring higher confidence
            for rc in response_concepts {
                if !all_concepts
                    .iter()
                    .any(|c| c.concept.name == rc.concept.name)
                {
                    all_concepts.push(rc);
                }
            }
        }

        Ok(all_concepts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_known_concept_extraction() {
        let mut extractor = ConceptExtractor::with_defaults();
        extractor.add_known_concept("friends");
        extractor.add_known_concept("projects");

        let concepts = extractor
            .extract("Tell me about my friends and ongoing projects")
            .unwrap();

        assert!(concepts.iter().any(|c| c.concept.name == "friends"));
        assert!(concepts.iter().any(|c| c.concept.name == "projects"));
    }

    #[test]
    fn test_normalize_concept_name() {
        assert_eq!(
            ConceptExtractor::normalize_concept_name("Work Experience"),
            "work-experience"
        );
        assert_eq!(
            ConceptExtractor::normalize_concept_name("My_Projects"),
            "my-projects"
        );
    }
}
