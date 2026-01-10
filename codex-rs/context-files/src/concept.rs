//! Concept types and relationships.
//!
//! Concepts are high-level topics that organize the context file system.
//! They form a graph with relationships between them.

use serde::{Deserialize, Serialize};

/// A concept represents a high-level topic in the knowledge graph.
///
/// Examples: "friends", "projects", "research", "hobbies", "work-experience"
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Concept {
    /// The concept name (unique identifier).
    pub name: String,

    /// Human-readable display name.
    pub display_name: String,

    /// Category for grouping concepts.
    pub category: Option<String>,

    /// Parent concept (for hierarchical organization).
    pub parent: Option<String>,
}

impl Concept {
    /// Create a new concept.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let display_name = name
            .replace('-', " ")
            .replace('_', " ");
        Self {
            name,
            display_name,
            category: None,
            parent: None,
        }
    }

    /// Set the display name.
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    /// Set the category.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set the parent concept.
    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }
}

/// A relationship between two concepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptRelation {
    /// Source concept name.
    pub from: String,

    /// Target concept name.
    pub to: String,

    /// Type of relationship.
    pub relation_type: RelationType,

    /// Strength of the relationship (0.0 to 1.0).
    pub strength: f32,
}

impl ConceptRelation {
    /// Create a new concept relation.
    pub fn new(from: impl Into<String>, to: impl Into<String>, relation_type: RelationType) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            relation_type,
            strength: 1.0,
        }
    }

    /// Set the relationship strength.
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength.clamp(0.0, 1.0);
        self
    }
}

/// Types of relationships between concepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// Parent-child relationship (hierarchical).
    Contains,

    /// Bidirectional association.
    RelatedTo,

    /// One concept depends on another.
    DependsOn,

    /// One concept references another.
    References,

    /// Temporal relationship (one precedes another).
    Precedes,

    /// Custom relationship type.
    Custom,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_concept_creation() {
        let concept = Concept::new("work-experience")
            .with_display_name("Work Experience")
            .with_category("professional");

        assert_eq!(concept.name, "work-experience");
        assert_eq!(concept.display_name, "Work Experience");
        assert_eq!(concept.category, Some("professional".to_string()));
    }

    #[test]
    fn test_relation_strength_clamping() {
        let relation = ConceptRelation::new("a", "b", RelationType::RelatedTo)
            .with_strength(1.5);
        assert_eq!(relation.strength, 1.0);

        let relation = ConceptRelation::new("a", "b", RelationType::RelatedTo)
            .with_strength(-0.5);
        assert_eq!(relation.strength, 0.0);
    }
}
