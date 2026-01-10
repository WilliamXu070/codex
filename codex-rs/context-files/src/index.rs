//! Concept index for fast lookup and search.
//!
//! The `ConceptIndex` maintains an inverted index of concepts for
//! efficient retrieval based on keywords, tags, and relationships.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::concept::{Concept, ConceptRelation, RelationType};
use crate::error::Result;

/// An inverted index for concept lookup.
///
/// The index supports:
/// - Keyword-to-concept mapping
/// - Tag-based filtering
/// - Relationship traversal
/// - Hierarchical navigation
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConceptIndex {
    /// Mapping from keywords to concept names.
    keyword_index: HashMap<String, HashSet<String>>,

    /// Mapping from tags to concept names.
    tag_index: HashMap<String, HashSet<String>>,

    /// All known concepts.
    concepts: HashMap<String, Concept>,

    /// Relationships between concepts.
    relations: Vec<ConceptRelation>,

    /// Parent-child relationships (concept -> children).
    hierarchy: HashMap<String, HashSet<String>>,
}

impl ConceptIndex {
    /// Create a new empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a concept to the index.
    pub fn add_concept(&mut self, concept: Concept) {
        let name = concept.name.clone();

        // Index keywords from the concept name
        for keyword in Self::extract_keywords(&concept.name) {
            self.keyword_index
                .entry(keyword)
                .or_default()
                .insert(name.clone());
        }

        // Index display name keywords
        for keyword in Self::extract_keywords(&concept.display_name) {
            self.keyword_index
                .entry(keyword)
                .or_default()
                .insert(name.clone());
        }

        // Update hierarchy if parent is set
        if let Some(ref parent) = concept.parent {
            self.hierarchy
                .entry(parent.clone())
                .or_default()
                .insert(name.clone());
        }

        self.concepts.insert(name, concept);
    }

    /// Add a tag to a concept.
    pub fn add_tag(&mut self, concept: &str, tag: impl Into<String>) {
        let tag = tag.into();
        self.tag_index
            .entry(tag.to_lowercase())
            .or_default()
            .insert(concept.to_string());
    }

    /// Add a relationship between concepts.
    pub fn add_relation(&mut self, relation: ConceptRelation) {
        // Also add to hierarchy if it's a Contains relationship
        if relation.relation_type == RelationType::Contains {
            self.hierarchy
                .entry(relation.from.clone())
                .or_default()
                .insert(relation.to.clone());
        }

        self.relations.push(relation);
    }

    /// Find concepts by keyword.
    pub fn find_by_keyword(&self, keyword: &str) -> Vec<&Concept> {
        let keyword_lower = keyword.to_lowercase();
        self.keyword_index
            .get(&keyword_lower)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.concepts.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find concepts by tag.
    pub fn find_by_tag(&self, tag: &str) -> Vec<&Concept> {
        let tag_lower = tag.to_lowercase();
        self.tag_index
            .get(&tag_lower)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.concepts.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find concepts matching multiple keywords (AND logic).
    pub fn find_by_keywords(&self, keywords: &[&str]) -> Vec<&Concept> {
        if keywords.is_empty() {
            return Vec::new();
        }

        let mut result: Option<HashSet<&str>> = None;

        for keyword in keywords {
            let keyword_lower = keyword.to_lowercase();
            if let Some(names) = self.keyword_index.get(&keyword_lower) {
                let name_refs: HashSet<&str> = names.iter().map(String::as_str).collect();
                result = Some(match result {
                    Some(existing) => existing.intersection(&name_refs).copied().collect(),
                    None => name_refs,
                });
            } else {
                return Vec::new(); // Keyword not found, no results
            }
        }

        result
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.concepts.get(*name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all children of a concept.
    pub fn get_children(&self, concept: &str) -> Vec<&Concept> {
        self.hierarchy
            .get(concept)
            .map(|children| {
                children
                    .iter()
                    .filter_map(|name| self.concepts.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all related concepts.
    pub fn get_related(&self, concept: &str) -> Vec<(&Concept, &ConceptRelation)> {
        self.relations
            .iter()
            .filter(|r| r.from == concept || r.to == concept)
            .filter_map(|r| {
                let other_name = if r.from == concept { &r.to } else { &r.from };
                self.concepts.get(other_name).map(|c| (c, r))
            })
            .collect()
    }

    /// Get a concept by name.
    pub fn get(&self, name: &str) -> Option<&Concept> {
        self.concepts.get(name)
    }

    /// Check if a concept exists.
    pub fn contains(&self, name: &str) -> bool {
        self.concepts.contains_key(name)
    }

    /// List all concept names.
    pub fn list(&self) -> Vec<&str> {
        self.concepts.keys().map(String::as_str).collect()
    }

    /// Remove a concept from the index.
    pub fn remove(&mut self, name: &str) -> Option<Concept> {
        if let Some(concept) = self.concepts.remove(name) {
            // Remove from keyword index
            for keywords in self.keyword_index.values_mut() {
                keywords.remove(name);
            }

            // Remove from tag index
            for tags in self.tag_index.values_mut() {
                tags.remove(name);
            }

            // Remove from hierarchy
            self.hierarchy.remove(name);
            for children in self.hierarchy.values_mut() {
                children.remove(name);
            }

            // Remove relations involving this concept
            self.relations
                .retain(|r| r.from != name && r.to != name);

            debug!("Removed concept from index: {name}");
            Some(concept)
        } else {
            None
        }
    }

    /// Extract keywords from a string for indexing.
    fn extract_keywords(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| c == '-' || c == '_' || c.is_whitespace())
            .filter(|s| s.len() >= 2)
            .map(String::from)
            .collect()
    }

    /// Get statistics about the index.
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            concept_count: self.concepts.len(),
            keyword_count: self.keyword_index.len(),
            tag_count: self.tag_index.len(),
            relation_count: self.relations.len(),
        }
    }
}

/// Statistics about the concept index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub concept_count: usize,
    pub keyword_count: usize,
    pub tag_count: usize,
    pub relation_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_keyword_search() {
        let mut index = ConceptIndex::new();
        index.add_concept(Concept::new("work-experience"));
        index.add_concept(Concept::new("work-projects"));

        let results = index.find_by_keyword("work");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_multi_keyword_search() {
        let mut index = ConceptIndex::new();
        index.add_concept(Concept::new("work-experience"));
        index.add_concept(Concept::new("work-projects"));
        index.add_concept(Concept::new("personal-projects"));

        let results = index.find_by_keywords(&["work", "projects"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "work-projects");
    }

    #[test]
    fn test_hierarchy() {
        let mut index = ConceptIndex::new();
        index.add_concept(Concept::new("hobbies"));
        index.add_concept(Concept::new("coding").with_parent("hobbies"));
        index.add_concept(Concept::new("gaming").with_parent("hobbies"));

        let children = index.get_children("hobbies");
        assert_eq!(children.len(), 2);
    }
}
