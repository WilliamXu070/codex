//! Core context file types and operations.
//!
//! A context file represents a concept that the AI maintains knowledge about.
//! Each context file contains structured metadata, content references, and
//! semantic embeddings for retrieval.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A context file represents a single concept in the AI's memory.
///
/// Context files are self-organizing knowledge units that contain:
/// - Structured metadata about the concept
/// - References to source files, conversations, and notes
/// - Semantic embeddings for similarity search
/// - Cross-references to related concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    /// Unique identifier for this context file.
    pub id: String,

    /// The concept this file represents (e.g., "friends", "projects", "research").
    pub concept: String,

    /// Metadata about the context file.
    pub metadata: ContextMetadata,

    /// High-level summary used for retrieval.
    pub summary: String,

    /// The actual content of the context file.
    pub content: ContextContent,
}

impl ContextFile {
    /// Create a new context file for a concept.
    pub fn new(concept: impl Into<String>, summary: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            concept: concept.into(),
            metadata: ContextMetadata {
                created: now,
                last_updated: now,
                version: 1,
                related_concepts: Vec::new(),
                tags: Vec::new(),
            },
            summary: summary.into(),
            content: ContextContent::default(),
        }
    }

    /// Update the last_updated timestamp and increment version.
    pub fn touch(&mut self) {
        self.metadata.last_updated = Utc::now();
        self.metadata.version += 1;
    }

    /// Add a related concept reference.
    pub fn add_related_concept(&mut self, concept: impl Into<String>) {
        let concept = concept.into();
        if !self.metadata.related_concepts.contains(&concept) {
            self.metadata.related_concepts.push(concept);
            self.touch();
        }
    }

    /// Add a content reference.
    pub fn add_reference(&mut self, reference: ContentReference) {
        self.content.references.push(reference);
        self.touch();
    }

    /// Set structured data for a key.
    pub fn set_structured(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.content.structured.insert(key.into(), value);
        self.touch();
    }

    /// Get structured data for a key.
    pub fn get_structured(&self, key: &str) -> Option<&serde_json::Value> {
        self.content.structured.get(key)
    }

    /// Update the semantic embedding.
    pub fn set_embedding(&mut self, embedding: Vec<f32>) {
        self.content.embedding = Some(embedding);
        self.touch();
    }
}

/// Metadata about a context file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    /// When this context file was created.
    pub created: DateTime<Utc>,

    /// When this context file was last updated.
    pub last_updated: DateTime<Utc>,

    /// Version number (incremented on each update).
    pub version: u64,

    /// Links to other context files.
    pub related_concepts: Vec<String>,

    /// Tags for categorization.
    pub tags: Vec<String>,
}

/// The content of a context file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextContent {
    /// Key-value pairs (birthdays, dates, preferences, etc.).
    pub structured: std::collections::HashMap<String, serde_json::Value>,

    /// Links to source files, conversations, and notes.
    pub references: Vec<ContentReference>,

    /// Semantic vector representation for similarity search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

/// A reference to source content (file, conversation, note, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentReference {
    /// The type of reference.
    pub reference_type: ReferenceType,

    /// Path or identifier for the reference.
    pub path: String,

    /// Optional excerpt from the source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,

    /// How relevant this reference is to the concept (0.0 to 1.0).
    pub relevance: f32,

    /// When this reference was added.
    pub added: DateTime<Utc>,
}

impl ContentReference {
    /// Create a new content reference.
    pub fn new(reference_type: ReferenceType, path: impl Into<String>, relevance: f32) -> Self {
        Self {
            reference_type,
            path: path.into(),
            excerpt: None,
            relevance,
            added: Utc::now(),
        }
    }

    /// Add an excerpt to this reference.
    pub fn with_excerpt(mut self, excerpt: impl Into<String>) -> Self {
        self.excerpt = Some(excerpt.into());
        self
    }
}

/// Type of content reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceType {
    /// Reference to a file on disk.
    File,
    /// Reference to a conversation.
    Conversation,
    /// Reference to a user note.
    Note,
    /// Reference to an external source (URL, etc.).
    External,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_context_file_creation() {
        let cf = ContextFile::new("friends", "Information about friends and family");
        assert_eq!(cf.concept, "friends");
        assert_eq!(cf.metadata.version, 1);
        assert!(cf.content.structured.is_empty());
    }

    #[test]
    fn test_touch_increments_version() {
        let mut cf = ContextFile::new("test", "Test context");
        let v1 = cf.metadata.version;
        cf.touch();
        assert_eq!(cf.metadata.version, v1 + 1);
    }

    #[test]
    fn test_add_structured_data() {
        let mut cf = ContextFile::new("friends", "Friends info");
        cf.set_structured("sarah_birthday", serde_json::json!("March 15"));
        assert_eq!(
            cf.get_structured("sarah_birthday"),
            Some(&serde_json::json!("March 15"))
        );
    }
}
