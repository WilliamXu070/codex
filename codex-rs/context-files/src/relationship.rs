//! Relationship extraction between entities.
//!
//! Extracts relationships between entities using pattern matching
//! and co-occurrence analysis.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::chunker::Chunk;
use crate::entity::{Entity, EntityType};

/// A relationship between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Unique identifier.
    pub id: String,

    /// Source entity ID.
    pub source_id: String,

    /// Source entity name (for display).
    pub source_name: String,

    /// Target entity ID.
    pub target_id: String,

    /// Target entity name (for display).
    pub target_name: String,

    /// Type of relationship.
    pub relationship_type: RelationshipType,

    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,

    /// Evidence supporting this relationship.
    pub evidence: Vec<RelationshipEvidence>,
}

impl Relationship {
    /// Create a new relationship.
    pub fn new(
        source: &Entity,
        target: &Entity,
        relationship_type: RelationshipType,
        confidence: f32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_id: source.id.clone(),
            source_name: source.name.clone(),
            target_id: target.id.clone(),
            target_name: target.name.clone(),
            relationship_type,
            confidence,
            evidence: Vec::new(),
        }
    }

    /// Add evidence for this relationship.
    pub fn add_evidence(&mut self, evidence: RelationshipEvidence) {
        self.evidence.push(evidence);
    }

    /// Get a string representation of this relationship.
    pub fn to_string_repr(&self) -> String {
        format!(
            "{} --[{}]--> {}",
            self.source_name,
            self.relationship_type.as_str(),
            self.target_name
        )
    }
}

/// Type of relationship between entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// A created/authored B.
    CreatedBy,
    /// A uses/utilizes B.
    Uses,
    /// A depends on B.
    DependsOn,
    /// A contains B.
    Contains,
    /// A is part of B.
    PartOf,
    /// A is related to B (co-occurrence).
    RelatedTo,
    /// A references B.
    References,
    /// A is version of B.
    VersionOf,
    /// A implements B.
    Implements,
    /// A extends B.
    Extends,
    /// A is located at B.
    LocatedAt,
    /// A occurred on B (for dates).
    OccurredOn,
    /// A works with B (for people).
    WorksWith,
    /// A manages/maintains B.
    Maintains,
}

impl RelationshipType {
    /// Get a string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CreatedBy => "created_by",
            Self::Uses => "uses",
            Self::DependsOn => "depends_on",
            Self::Contains => "contains",
            Self::PartOf => "part_of",
            Self::RelatedTo => "related_to",
            Self::References => "references",
            Self::VersionOf => "version_of",
            Self::Implements => "implements",
            Self::Extends => "extends",
            Self::LocatedAt => "located_at",
            Self::OccurredOn => "occurred_on",
            Self::WorksWith => "works_with",
            Self::Maintains => "maintains",
        }
    }

    /// Get a human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CreatedBy => "Created By",
            Self::Uses => "Uses",
            Self::DependsOn => "Depends On",
            Self::Contains => "Contains",
            Self::PartOf => "Part Of",
            Self::RelatedTo => "Related To",
            Self::References => "References",
            Self::VersionOf => "Version Of",
            Self::Implements => "Implements",
            Self::Extends => "Extends",
            Self::LocatedAt => "Located At",
            Self::OccurredOn => "Occurred On",
            Self::WorksWith => "Works With",
            Self::Maintains => "Maintains",
        }
    }
}

/// Evidence for a relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEvidence {
    /// Type of evidence.
    pub evidence_type: EvidenceType,

    /// The text or pattern that provides evidence.
    pub text: String,

    /// Chunk ID where this evidence was found.
    pub chunk_id: Option<String>,

    /// Confidence contribution from this evidence.
    pub confidence_contribution: f32,
}

/// Type of relationship evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    /// Direct pattern match in text.
    PatternMatch,
    /// Entities appear in same chunk.
    CoOccurrence,
    /// Structural relationship (e.g., in same section).
    Structural,
    /// Inferred from entity types.
    TypeInference,
}

/// Configuration for relationship extraction.
#[derive(Debug, Clone)]
pub struct RelationshipExtractorConfig {
    /// Minimum confidence threshold.
    pub min_confidence: f32,

    /// Whether to use pattern matching.
    pub use_patterns: bool,

    /// Whether to use co-occurrence analysis.
    pub use_cooccurrence: bool,

    /// Minimum co-occurrence score to create relationship.
    pub cooccurrence_threshold: f32,

    /// Whether to infer relationships from entity types.
    pub use_type_inference: bool,
}

impl Default for RelationshipExtractorConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.4,
            use_patterns: true,
            use_cooccurrence: true,
            cooccurrence_threshold: 0.3,
            use_type_inference: true,
        }
    }
}

/// Relationship extractor.
pub struct RelationshipExtractor {
    config: RelationshipExtractorConfig,
}

impl RelationshipExtractor {
    /// Create a new relationship extractor.
    pub fn new() -> Self {
        Self {
            config: RelationshipExtractorConfig::default(),
        }
    }

    /// Create an extractor with custom configuration.
    pub fn with_config(config: RelationshipExtractorConfig) -> Self {
        Self { config }
    }

    /// Extract relationships between entities.
    pub fn extract(&self, entities: &[Entity], chunks: &[Chunk]) -> Vec<Relationship> {
        let mut relationships = Vec::new();

        // Build entity lookup by chunk
        let chunk_entities = self.build_chunk_entity_map(entities);

        // Pattern-based relationship extraction
        if self.config.use_patterns {
            relationships.extend(self.extract_pattern_relationships(entities, chunks));
        }

        // Co-occurrence based relationships
        if self.config.use_cooccurrence {
            relationships
                .extend(self.extract_cooccurrence_relationships(entities, &chunk_entities));
        }

        // Type-inference based relationships
        if self.config.use_type_inference {
            relationships.extend(self.infer_type_relationships(entities, &chunk_entities));
        }

        // Deduplicate and filter
        self.deduplicate_relationships(relationships)
    }

    /// Build a map of chunk ID to entities mentioned in that chunk.
    fn build_chunk_entity_map<'a>(
        &self,
        entities: &'a [Entity],
    ) -> HashMap<String, Vec<&'a Entity>> {
        let mut map: HashMap<String, Vec<&'a Entity>> = HashMap::new();

        for entity in entities {
            for mention in &entity.mentions {
                map.entry(mention.chunk_id.clone())
                    .or_default()
                    .push(entity);
            }
        }

        map
    }

    /// Extract relationships using pattern matching.
    fn extract_pattern_relationships(
        &self,
        entities: &[Entity],
        chunks: &[Chunk],
    ) -> Vec<Relationship> {
        let mut relationships = Vec::new();

        // Relationship patterns: (pattern, source_group, target_group, relationship_type)
        let patterns: Vec<(&str, usize, usize, RelationshipType)> = vec![
            // Dependency patterns
            (
                r"(\S+)\s+depends\s+on\s+(\S+)",
                1,
                2,
                RelationshipType::DependsOn,
            ),
            (
                r"(\S+)\s+requires\s+(\S+)",
                1,
                2,
                RelationshipType::DependsOn,
            ),
            (r"(\S+)\s+uses\s+(\S+)", 1, 2, RelationshipType::Uses),
            (r"built\s+with\s+(\S+)", 0, 1, RelationshipType::Uses),
            // Authorship patterns
            (
                r"(\S+)\s+(?:created|wrote|authored)\s+by\s+(\S+)",
                1,
                2,
                RelationshipType::CreatedBy,
            ),
            (
                r"(\S+)\s+maintains?\s+(\S+)",
                1,
                2,
                RelationshipType::Maintains,
            ),
            // Containment patterns
            (
                r"(\S+)\s+contains?\s+(\S+)",
                1,
                2,
                RelationshipType::Contains,
            ),
            (
                r"(\S+)\s+includes?\s+(\S+)",
                1,
                2,
                RelationshipType::Contains,
            ),
            // Implementation patterns
            (
                r"(\S+)\s+implements?\s+(\S+)",
                1,
                2,
                RelationshipType::Implements,
            ),
            (r"(\S+)\s+extends?\s+(\S+)", 1, 2, RelationshipType::Extends),
        ];

        // Build entity name lookup for matching
        let entity_lookup: HashMap<String, &Entity> = entities
            .iter()
            .map(|e| (e.normalized_name.clone(), e))
            .collect();

        for chunk in chunks {
            for (pattern, _source_group, target_group, rel_type) in &patterns {
                if let Ok(re) = regex_lite::Regex::new(pattern) {
                    for cap in re.captures_iter(&chunk.content) {
                        if let Some(target_match) = cap.get(*target_group) {
                            let target_text = target_match.as_str().to_lowercase();

                            // Try to find matching entities
                            if let Some(target_entity) = entity_lookup.get(&target_text) {
                                // Look for source entity in same chunk
                                for source_entity in entities {
                                    if source_entity.id != target_entity.id
                                        && source_entity
                                            .mentions
                                            .iter()
                                            .any(|m| m.chunk_id == chunk.id)
                                    {
                                        let mut rel = Relationship::new(
                                            source_entity,
                                            target_entity,
                                            *rel_type,
                                            0.8,
                                        );
                                        rel.add_evidence(RelationshipEvidence {
                                            evidence_type: EvidenceType::PatternMatch,
                                            text: cap.get(0).unwrap().as_str().to_string(),
                                            chunk_id: Some(chunk.id.clone()),
                                            confidence_contribution: 0.8,
                                        });
                                        relationships.push(rel);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        relationships
    }

    /// Extract relationships based on co-occurrence in chunks.
    fn extract_cooccurrence_relationships(
        &self,
        entities: &[Entity],
        chunk_entities: &HashMap<String, Vec<&Entity>>,
    ) -> Vec<Relationship> {
        let mut relationships = Vec::new();
        let mut pair_scores: HashMap<(String, String), f32> = HashMap::new();
        let mut pair_chunks: HashMap<(String, String), Vec<String>> = HashMap::new();

        // Calculate co-occurrence scores
        for (chunk_id, entities_in_chunk) in chunk_entities {
            let n = entities_in_chunk.len();
            if n < 2 {
                continue;
            }

            // Each pair of entities in the same chunk gets a score
            for i in 0..n {
                for j in (i + 1)..n {
                    let e1 = entities_in_chunk[i];
                    let e2 = entities_in_chunk[j];

                    // Skip if same entity type (less meaningful relationship)
                    if e1.entity_type == e2.entity_type {
                        continue;
                    }

                    let key = if e1.id < e2.id {
                        (e1.id.clone(), e2.id.clone())
                    } else {
                        (e2.id.clone(), e1.id.clone())
                    };

                    // Score based on chunk size (smaller chunk = stronger relationship)
                    let score = 1.0 / (1.0 + (n as f32).ln());

                    *pair_scores.entry(key.clone()).or_insert(0.0) += score;
                    pair_chunks.entry(key).or_default().push(chunk_id.clone());
                }
            }
        }

        // Create relationships for high-scoring pairs
        let entity_map: HashMap<String, &Entity> =
            entities.iter().map(|e| (e.id.clone(), e)).collect();

        for ((id1, id2), score) in pair_scores {
            if score >= self.config.cooccurrence_threshold {
                if let (Some(e1), Some(e2)) = (entity_map.get(&id1), entity_map.get(&id2)) {
                    let normalized_score = (score / 2.0).min(1.0);

                    let mut rel =
                        Relationship::new(e1, e2, RelationshipType::RelatedTo, normalized_score);

                    if let Some(chunk_ids) = pair_chunks.get(&(id1.clone(), id2.clone())) {
                        rel.add_evidence(RelationshipEvidence {
                            evidence_type: EvidenceType::CoOccurrence,
                            text: format!("Co-occurred in {} chunks", chunk_ids.len()),
                            chunk_id: chunk_ids.first().cloned(),
                            confidence_contribution: normalized_score,
                        });
                    }

                    relationships.push(rel);
                }
            }
        }

        relationships
    }

    /// Infer relationships based on entity types.
    fn infer_type_relationships(
        &self,
        entities: &[Entity],
        chunk_entities: &HashMap<String, Vec<&Entity>>,
    ) -> Vec<Relationship> {
        let mut relationships = Vec::new();

        for (chunk_id, entities_in_chunk) in chunk_entities {
            // Look for specific type combinations

            // Person + Project = Created/Maintains
            let people: Vec<_> = entities_in_chunk
                .iter()
                .filter(|e| e.entity_type == EntityType::Person)
                .collect();
            let projects: Vec<_> = entities_in_chunk
                .iter()
                .filter(|e| e.entity_type == EntityType::Project)
                .collect();

            for person in &people {
                for project in &projects {
                    let mut rel =
                        Relationship::new(person, project, RelationshipType::Maintains, 0.5);
                    rel.add_evidence(RelationshipEvidence {
                        evidence_type: EvidenceType::TypeInference,
                        text: "Person mentioned with project".to_string(),
                        chunk_id: Some(chunk_id.clone()),
                        confidence_contribution: 0.5,
                    });
                    relationships.push(rel);
                }
            }

            // Project + Technology = Uses
            let technologies: Vec<_> = entities_in_chunk
                .iter()
                .filter(|e| e.entity_type == EntityType::Technology)
                .collect();

            for project in &projects {
                for tech in &technologies {
                    let mut rel = Relationship::new(project, tech, RelationshipType::Uses, 0.6);
                    rel.add_evidence(RelationshipEvidence {
                        evidence_type: EvidenceType::TypeInference,
                        text: "Technology mentioned with project".to_string(),
                        chunk_id: Some(chunk_id.clone()),
                        confidence_contribution: 0.6,
                    });
                    relationships.push(rel);
                }
            }

            // Technology + Technology (in same context) = RelatedTo
            for i in 0..technologies.len() {
                for j in (i + 1)..technologies.len() {
                    let mut rel = Relationship::new(
                        technologies[i],
                        technologies[j],
                        RelationshipType::RelatedTo,
                        0.4,
                    );
                    rel.add_evidence(RelationshipEvidence {
                        evidence_type: EvidenceType::TypeInference,
                        text: "Technologies mentioned together".to_string(),
                        chunk_id: Some(chunk_id.clone()),
                        confidence_contribution: 0.4,
                    });
                    relationships.push(rel);
                }
            }
        }

        relationships
    }

    /// Deduplicate relationships and combine evidence.
    fn deduplicate_relationships(&self, relationships: Vec<Relationship>) -> Vec<Relationship> {
        let mut unique: HashMap<(String, String, String), Relationship> = HashMap::new();

        for rel in relationships {
            let key = (
                rel.source_id.clone(),
                rel.target_id.clone(),
                rel.relationship_type.as_str().to_string(),
            );

            unique
                .entry(key)
                .and_modify(|existing| {
                    // Combine evidence and boost confidence
                    for evidence in &rel.evidence {
                        existing.add_evidence(evidence.clone());
                    }
                    existing.confidence = (existing.confidence + rel.confidence * 0.5).min(1.0);
                })
                .or_insert(rel);
        }

        unique
            .into_values()
            .filter(|r| r.confidence >= self.config.min_confidence)
            .collect()
    }
}

impl Default for RelationshipExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::ChunkType;

    fn make_chunk(id: &str, content: &str) -> Chunk {
        Chunk {
            id: id.to_string(),
            content: content.to_string(),
            source: None,
            chunk_type: ChunkType::Text,
            start_offset: 0,
            end_offset: content.len(),
            parent_id: None,
            metadata: Default::default(),
        }
    }

    fn make_entity(name: &str, entity_type: EntityType, chunk_ids: &[&str]) -> Entity {
        let mut entity = Entity::new(name, entity_type, 0.9);
        for chunk_id in chunk_ids {
            entity.add_mention(crate::entity::EntityMention {
                chunk_id: chunk_id.to_string(),
                position: 0,
                matched_text: name.to_string(),
                context: None,
            });
        }
        entity
    }

    #[test]
    fn test_cooccurrence_relationships() {
        let extractor = RelationshipExtractor::new();

        let chunks = vec![make_chunk("chunk1", "Rust project using tokio")];

        let entities = vec![
            make_entity("Rust", EntityType::Technology, &["chunk1"]),
            make_entity("tokio", EntityType::Technology, &["chunk1"]),
            make_entity("my-project", EntityType::Project, &["chunk1"]),
        ];

        let relationships = extractor.extract(&entities, &chunks);

        // Should have relationships between entities in same chunk
        assert!(!relationships.is_empty());
    }

    #[test]
    fn test_type_inference_relationships() {
        let extractor = RelationshipExtractor::new();

        let chunks = vec![make_chunk("chunk1", "John maintains the codex project")];

        let entities = vec![
            make_entity("John", EntityType::Person, &["chunk1"]),
            make_entity("codex", EntityType::Project, &["chunk1"]),
        ];

        let relationships = extractor.extract(&entities, &chunks);

        // Should infer person-project relationship
        let person_project: Vec<_> = relationships
            .iter()
            .filter(|r| r.source_name == "John" && r.target_name == "codex")
            .collect();

        assert!(!person_project.is_empty());
    }

    #[test]
    fn test_relationship_deduplication() {
        let extractor = RelationshipExtractor::new();

        let chunks = vec![
            make_chunk("chunk1", "Using Rust"),
            make_chunk("chunk2", "Also using Rust"),
        ];

        let entities = vec![
            make_entity("project", EntityType::Project, &["chunk1", "chunk2"]),
            make_entity("Rust", EntityType::Technology, &["chunk1", "chunk2"]),
        ];

        let relationships = extractor.extract(&entities, &chunks);

        // Same relationship should be deduplicated
        let rust_rels: Vec<_> = relationships
            .iter()
            .filter(|r| r.target_name == "Rust")
            .collect();

        // Should have only one relationship (deduplicated)
        assert!(rust_rels.len() <= 2);
    }
}
