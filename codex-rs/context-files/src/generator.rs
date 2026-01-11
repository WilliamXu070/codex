//! Context file generation from extracted entities and relationships.
//!
//! This module takes the output from entity and relationship extraction
//! and generates structured context files that can be used for retrieval.

use std::collections::{HashMap, HashSet};

use crate::context_file::ContextFile;
use crate::entity::{Entity, EntityType};
use crate::relationship::{Relationship, RelationshipType};

/// Configuration for context file generation.
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Minimum number of entities to create a context file.
    pub min_entities_per_context: usize,

    /// Maximum number of entities per context file.
    pub max_entities_per_context: usize,

    /// Minimum relationship strength for clustering.
    pub min_relationship_strength: f32,

    /// Whether to create type-based context files (e.g., "people", "technologies").
    pub create_type_contexts: bool,

    /// Whether to create relationship-based clusters.
    pub create_relationship_clusters: bool,

    /// Source identifier for generated context files.
    pub source_id: Option<String>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            min_entities_per_context: 1,
            max_entities_per_context: 50,
            min_relationship_strength: 0.3,
            create_type_contexts: true,
            create_relationship_clusters: true,
            source_id: None,
        }
    }
}

/// A cluster of related entities that will become a context file.
#[derive(Debug, Clone)]
pub struct EntityCluster {
    /// Unique identifier for the cluster.
    pub id: String,

    /// Name of the cluster (becomes concept name).
    pub name: String,

    /// Entity IDs in this cluster.
    pub entity_ids: Vec<String>,

    /// Primary type of entities in this cluster.
    pub primary_type: Option<EntityType>,

    /// Clustering method that created this cluster.
    pub cluster_method: ClusterMethod,

    /// Confidence score for this cluster.
    pub confidence: f32,
}

/// Method used to create a cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterMethod {
    /// Clustered by entity type.
    TypeBased,
    /// Clustered by relationship connectivity.
    RelationshipBased,
    /// Clustered by document source.
    SourceBased,
    /// Single entity cluster.
    SingleEntity,
}

/// Generated context with metadata.
#[derive(Debug, Clone)]
pub struct GeneratedContext {
    /// The context file.
    pub context_file: ContextFile,

    /// Entities included in this context.
    pub entities: Vec<Entity>,

    /// Relationships within this context.
    pub internal_relationships: Vec<Relationship>,

    /// Relationships to other contexts.
    pub external_relationships: Vec<Relationship>,

    /// How this context was generated.
    pub cluster_method: ClusterMethod,
}

/// Context file generator.
pub struct ContextGenerator {
    config: GeneratorConfig,
}

impl ContextGenerator {
    /// Create a new generator with default configuration.
    pub fn new() -> Self {
        Self {
            config: GeneratorConfig::default(),
        }
    }

    /// Create a new generator with custom configuration.
    pub fn with_config(config: GeneratorConfig) -> Self {
        Self { config }
    }

    /// Generate context files from entities and relationships.
    pub fn generate(
        &self,
        entities: &[Entity],
        relationships: &[Relationship],
    ) -> Vec<GeneratedContext> {
        let mut contexts = Vec::new();

        // Build entity lookup map
        let entity_map: HashMap<&str, &Entity> =
            entities.iter().map(|e| (e.id.as_str(), e)).collect();

        // Create type-based clusters
        if self.config.create_type_contexts {
            let type_clusters = self.cluster_by_type(entities);
            for cluster in type_clusters {
                if let Some(ctx) = self.cluster_to_context(&cluster, &entity_map, relationships) {
                    contexts.push(ctx);
                }
            }
        }

        // Create relationship-based clusters
        if self.config.create_relationship_clusters {
            let rel_clusters = self.cluster_by_relationships(entities, relationships);
            for cluster in rel_clusters {
                // Avoid duplicating entities already in type clusters
                if self.config.create_type_contexts {
                    // Only add relationship clusters that span multiple types
                    let types: HashSet<_> = cluster
                        .entity_ids
                        .iter()
                        .filter_map(|id| entity_map.get(id.as_str()))
                        .map(|e| &e.entity_type)
                        .collect();
                    if types.len() <= 1 {
                        continue;
                    }
                }
                if let Some(ctx) = self.cluster_to_context(&cluster, &entity_map, relationships) {
                    contexts.push(ctx);
                }
            }
        }

        // Create single-entity contexts for high-confidence entities not in clusters
        let clustered_ids: HashSet<String> = contexts
            .iter()
            .flat_map(|c| c.entities.iter().map(|e| e.id.clone()))
            .collect();

        let mut single_entity_contexts = Vec::new();
        for entity in entities {
            if !clustered_ids.contains(&entity.id) && entity.confidence >= 0.7 {
                let cluster = EntityCluster {
                    id: format!("single-{}", entity.id),
                    name: entity.normalized_name.clone(),
                    entity_ids: vec![entity.id.clone()],
                    primary_type: Some(entity.entity_type.clone()),
                    cluster_method: ClusterMethod::SingleEntity,
                    confidence: entity.confidence,
                };
                if let Some(ctx) = self.cluster_to_context(&cluster, &entity_map, relationships) {
                    single_entity_contexts.push(ctx);
                }
            }
        }
        contexts.extend(single_entity_contexts);

        contexts
    }

    /// Cluster entities by their type.
    fn cluster_by_type(&self, entities: &[Entity]) -> Vec<EntityCluster> {
        let mut type_groups: HashMap<EntityType, Vec<&Entity>> = HashMap::new();

        for entity in entities {
            type_groups
                .entry(entity.entity_type.clone())
                .or_default()
                .push(entity);
        }

        let mut clusters = Vec::new();

        for (entity_type, group) in type_groups {
            if group.len() < self.config.min_entities_per_context {
                continue;
            }

            let cluster_name = type_to_concept_name(&entity_type);
            let avg_confidence =
                group.iter().map(|e| e.confidence).sum::<f32>() / group.len() as f32;

            // Split large groups
            for (i, chunk) in group
                .chunks(self.config.max_entities_per_context)
                .enumerate()
            {
                let suffix = if i > 0 {
                    format!("-{}", i + 1)
                } else {
                    String::new()
                };

                clusters.push(EntityCluster {
                    id: format!("type-{:?}{}", entity_type, suffix),
                    name: format!("{}{}", cluster_name, suffix),
                    entity_ids: chunk.iter().map(|e| e.id.clone()).collect(),
                    primary_type: Some(entity_type.clone()),
                    cluster_method: ClusterMethod::TypeBased,
                    confidence: avg_confidence,
                });
            }
        }

        clusters
    }

    /// Cluster entities by relationship connectivity using union-find.
    fn cluster_by_relationships(
        &self,
        entities: &[Entity],
        relationships: &[Relationship],
    ) -> Vec<EntityCluster> {
        // Build adjacency list with strong relationships only
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

        for rel in relationships {
            if rel.confidence >= self.config.min_relationship_strength {
                adjacency
                    .entry(rel.source_id.as_str())
                    .or_default()
                    .push(rel.target_id.as_str());
                adjacency
                    .entry(rel.target_id.as_str())
                    .or_default()
                    .push(rel.source_id.as_str());
            }
        }

        // Union-Find for connected components
        let entity_ids: Vec<&str> = entities.iter().map(|e| e.id.as_str()).collect();
        let id_to_idx: HashMap<&str, usize> = entity_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, i))
            .collect();

        let mut parent: Vec<usize> = (0..entities.len()).collect();

        fn find(parent: &mut [usize], i: usize) -> usize {
            if parent[i] != i {
                parent[i] = find(parent, parent[i]);
            }
            parent[i]
        }

        fn union(parent: &mut [usize], i: usize, j: usize) {
            let pi = find(parent, i);
            let pj = find(parent, j);
            if pi != pj {
                parent[pi] = pj;
            }
        }

        // Union connected entities
        for (source, targets) in &adjacency {
            if let Some(&src_idx) = id_to_idx.get(source) {
                for target in targets {
                    if let Some(&tgt_idx) = id_to_idx.get(target) {
                        union(&mut parent, src_idx, tgt_idx);
                    }
                }
            }
        }

        // Group by component
        let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..entities.len() {
            let root = find(&mut parent, i);
            components.entry(root).or_default().push(i);
        }

        // Create clusters from components with multiple entities
        let mut clusters = Vec::new();
        for (_, indices) in components {
            if indices.len() < 2 {
                continue; // Skip single-entity components
            }

            let cluster_entities: Vec<&Entity> = indices.iter().map(|&i| &entities[i]).collect();

            // Find the most central entity for naming
            let central_entity = cluster_entities
                .iter()
                .max_by(|a, b| {
                    let a_rels = adjacency.get(a.id.as_str()).map(|v| v.len()).unwrap_or(0);
                    let b_rels = adjacency.get(b.id.as_str()).map(|v| v.len()).unwrap_or(0);
                    a_rels.cmp(&b_rels)
                })
                .unwrap();

            let avg_confidence =
                cluster_entities.iter().map(|e| e.confidence).sum::<f32>() / indices.len() as f32;

            clusters.push(EntityCluster {
                id: format!("rel-{}", central_entity.id),
                name: format!("{}-context", central_entity.normalized_name),
                entity_ids: cluster_entities.iter().map(|e| e.id.clone()).collect(),
                primary_type: Some(central_entity.entity_type.clone()),
                cluster_method: ClusterMethod::RelationshipBased,
                confidence: avg_confidence,
            });
        }

        clusters
    }

    /// Convert a cluster to a generated context.
    fn cluster_to_context(
        &self,
        cluster: &EntityCluster,
        entity_map: &HashMap<&str, &Entity>,
        relationships: &[Relationship],
    ) -> Option<GeneratedContext> {
        // Gather entities
        let entities: Vec<Entity> = cluster
            .entity_ids
            .iter()
            .filter_map(|id| entity_map.get(id.as_str()).map(|e| (*e).clone()))
            .collect();

        if entities.is_empty() {
            return None;
        }

        let entity_id_set: HashSet<_> = cluster.entity_ids.iter().map(|s| s.as_str()).collect();

        // Separate internal and external relationships
        let mut internal_relationships = Vec::new();
        let mut external_relationships = Vec::new();

        for rel in relationships {
            let source_in = entity_id_set.contains(rel.source_id.as_str());
            let target_in = entity_id_set.contains(rel.target_id.as_str());

            if source_in && target_in {
                internal_relationships.push(rel.clone());
            } else if source_in || target_in {
                external_relationships.push(rel.clone());
            }
        }

        // Generate summary
        let summary = self.generate_summary(&entities, &internal_relationships, cluster);

        // Create context file
        let mut context_file = ContextFile::new(&cluster.name, &summary);

        // Add metadata as structured data
        if let Some(source_id) = &self.config.source_id {
            context_file.set_structured("source", serde_json::json!(source_id));
        }
        context_file.set_structured(
            "cluster_method",
            serde_json::json!(format!("{:?}", cluster.cluster_method)),
        );
        context_file.set_structured("entity_count", serde_json::json!(entities.len()));
        context_file.set_structured("confidence", serde_json::json!(cluster.confidence));

        // Add related concepts
        for rel in &external_relationships {
            let related_name = if entity_id_set.contains(rel.source_id.as_str()) {
                &rel.target_name
            } else {
                &rel.source_name
            };
            context_file.add_related_concept(related_name.to_lowercase().replace(' ', "-"));
        }

        Some(GeneratedContext {
            context_file,
            entities,
            internal_relationships,
            external_relationships,
            cluster_method: cluster.cluster_method,
        })
    }

    /// Generate a summary for a context.
    fn generate_summary(
        &self,
        entities: &[Entity],
        relationships: &[Relationship],
        cluster: &EntityCluster,
    ) -> String {
        let mut parts = Vec::new();

        // Opening based on cluster type
        match cluster.cluster_method {
            ClusterMethod::TypeBased => {
                if let Some(ref entity_type) = cluster.primary_type {
                    let type_name = type_to_plural_name(entity_type);
                    parts.push(format!(
                        "This context contains {} {}.",
                        entities.len(),
                        type_name
                    ));
                }
            }
            ClusterMethod::RelationshipBased => {
                parts.push(format!(
                    "This context groups {} related entities centered around '{}'.",
                    entities.len(),
                    cluster.name.replace("-context", "")
                ));
            }
            ClusterMethod::SingleEntity => {
                if let Some(entity) = entities.first() {
                    parts.push(format!(
                        "{} is a {} with {} mentions.",
                        entity.name,
                        type_to_singular_name(&entity.entity_type),
                        entity.mentions.len()
                    ));
                }
            }
            ClusterMethod::SourceBased => {
                parts.push(format!(
                    "This context contains {} entities from the same source.",
                    entities.len()
                ));
            }
        }

        // List key entities (up to 5)
        if entities.len() > 1 {
            let key_entities: Vec<_> = entities.iter().take(5).map(|e| e.name.as_str()).collect();
            parts.push(format!("Key items: {}.", key_entities.join(", ")));
        }

        // Describe relationships
        if !relationships.is_empty() {
            let rel_types: HashSet<_> =
                relationships.iter().map(|r| &r.relationship_type).collect();
            let rel_descriptions: Vec<_> = rel_types
                .iter()
                .map(|t| relationship_type_to_name(t))
                .collect();
            parts.push(format!(
                "Contains {} relationships: {}.",
                relationships.len(),
                rel_descriptions.join(", ")
            ));
        }

        // Add entity details
        if entities.len() <= 3 {
            for entity in entities {
                if !entity.attributes.is_empty() {
                    let attrs: Vec<_> = entity
                        .attributes
                        .iter()
                        .take(3)
                        .map(|(k, v)| format!("{}: {}", k, v))
                        .collect();
                    parts.push(format!("{} - {}.", entity.name, attrs.join(", ")));
                }
            }
        }

        parts.join(" ")
    }
}

impl Default for ContextGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert entity type to a concept name.
fn type_to_concept_name(entity_type: &EntityType) -> String {
    match entity_type {
        EntityType::Person => "people".to_string(),
        EntityType::Project => "projects".to_string(),
        EntityType::Technology => "technologies".to_string(),
        EntityType::Date => "timeline".to_string(),
        EntityType::Location => "locations".to_string(),
        EntityType::Organization => "organizations".to_string(),
        EntityType::Version => "versions".to_string(),
        EntityType::Url => "links".to_string(),
        EntityType::Email => "contacts".to_string(),
        EntityType::Concept => "concepts".to_string(),
        EntityType::File => "files".to_string(),
        EntityType::CodeElement => "code-elements".to_string(),
    }
}

/// Convert entity type to plural noun.
fn type_to_plural_name(entity_type: &EntityType) -> &'static str {
    match entity_type {
        EntityType::Person => "people",
        EntityType::Project => "projects",
        EntityType::Technology => "technologies",
        EntityType::Date => "dates",
        EntityType::Location => "locations",
        EntityType::Organization => "organizations",
        EntityType::Version => "versions",
        EntityType::Url => "URLs",
        EntityType::Email => "email addresses",
        EntityType::Concept => "concepts",
        EntityType::File => "files",
        EntityType::CodeElement => "code elements",
    }
}

/// Convert entity type to singular noun.
fn type_to_singular_name(entity_type: &EntityType) -> &'static str {
    match entity_type {
        EntityType::Person => "person",
        EntityType::Project => "project",
        EntityType::Technology => "technology",
        EntityType::Date => "date",
        EntityType::Location => "location",
        EntityType::Organization => "organization",
        EntityType::Version => "version",
        EntityType::Url => "URL",
        EntityType::Email => "email address",
        EntityType::Concept => "concept",
        EntityType::File => "file",
        EntityType::CodeElement => "code element",
    }
}

/// Convert relationship type to human-readable name.
fn relationship_type_to_name(rel_type: &RelationshipType) -> &'static str {
    match rel_type {
        RelationshipType::CreatedBy => "created by",
        RelationshipType::Uses => "uses",
        RelationshipType::DependsOn => "depends on",
        RelationshipType::Contains => "contains",
        RelationshipType::PartOf => "part of",
        RelationshipType::RelatedTo => "related to",
        RelationshipType::References => "references",
        RelationshipType::VersionOf => "version of",
        RelationshipType::Implements => "implements",
        RelationshipType::Extends => "extends",
        RelationshipType::LocatedAt => "located at",
        RelationshipType::OccurredOn => "occurred on",
        RelationshipType::WorksWith => "works with",
        RelationshipType::Maintains => "maintains",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityMention;

    fn create_test_entity(id: &str, name: &str, entity_type: EntityType) -> Entity {
        Entity {
            id: id.to_string(),
            name: name.to_string(),
            normalized_name: name.to_lowercase().replace(' ', "-"),
            entity_type,
            confidence: 0.8,
            mentions: vec![EntityMention {
                chunk_id: "test-chunk".to_string(),
                position: 0,
                matched_text: name.to_string(),
                context: Some(format!("Test context for {}", name)),
            }],
            attributes: HashMap::new(),
        }
    }

    fn create_test_relationship(
        source: &Entity,
        target: &Entity,
        rel_type: RelationshipType,
    ) -> Relationship {
        use crate::relationship::{EvidenceType, RelationshipEvidence};

        Relationship {
            id: format!("{}-{}", source.id, target.id),
            source_id: source.id.clone(),
            source_name: source.name.clone(),
            target_id: target.id.clone(),
            target_name: target.name.clone(),
            relationship_type: rel_type,
            confidence: 0.8,
            evidence: vec![RelationshipEvidence {
                evidence_type: EvidenceType::PatternMatch,
                text: format!("{} is related to {}", source.name, target.name),
                chunk_id: Some("test-chunk".to_string()),
                confidence_contribution: 0.8,
            }],
        }
    }

    #[test]
    fn test_type_based_clustering() {
        let entities = vec![
            create_test_entity("p1", "Alice", EntityType::Person),
            create_test_entity("p2", "Bob", EntityType::Person),
            create_test_entity("t1", "Rust", EntityType::Technology),
            create_test_entity("t2", "Python", EntityType::Technology),
        ];

        let generator = ContextGenerator::new();
        let contexts = generator.generate(&entities, &[]);

        // Should have at least people and technologies clusters
        assert!(contexts.len() >= 2);

        let people_ctx = contexts.iter().find(|c| c.context_file.concept == "people");
        assert!(people_ctx.is_some());
        assert_eq!(people_ctx.unwrap().entities.len(), 2);

        let tech_ctx = contexts
            .iter()
            .find(|c| c.context_file.concept == "technologies");
        assert!(tech_ctx.is_some());
        assert_eq!(tech_ctx.unwrap().entities.len(), 2);
    }

    #[test]
    fn test_relationship_based_clustering() {
        let entities = vec![
            create_test_entity("p1", "Alice", EntityType::Person),
            create_test_entity("proj1", "MyProject", EntityType::Project),
            create_test_entity("t1", "Rust", EntityType::Technology),
        ];

        let relationships = vec![
            create_test_relationship(&entities[0], &entities[1], RelationshipType::CreatedBy),
            create_test_relationship(&entities[1], &entities[2], RelationshipType::Uses),
        ];

        let mut config = GeneratorConfig::default();
        config.create_type_contexts = false;
        config.create_relationship_clusters = true;

        let generator = ContextGenerator::with_config(config);
        let contexts = generator.generate(&entities, &relationships);

        // Should have a relationship-based cluster
        let rel_ctx = contexts
            .iter()
            .find(|c| c.cluster_method == ClusterMethod::RelationshipBased);
        assert!(rel_ctx.is_some());
    }

    #[test]
    fn test_summary_generation() {
        let entities = vec![
            create_test_entity("t1", "Rust", EntityType::Technology),
            create_test_entity("t2", "Python", EntityType::Technology),
            create_test_entity("t3", "TypeScript", EntityType::Technology),
        ];

        let generator = ContextGenerator::new();
        let contexts = generator.generate(&entities, &[]);

        let tech_ctx = contexts
            .iter()
            .find(|c| c.context_file.concept == "technologies")
            .unwrap();

        assert!(tech_ctx.context_file.summary.contains("3 technologies"));
        assert!(tech_ctx.context_file.summary.contains("Key items:"));
    }

    #[test]
    fn test_external_relationships() {
        let entities = vec![
            create_test_entity("p1", "Alice", EntityType::Person),
            create_test_entity("p2", "Bob", EntityType::Person),
            create_test_entity("proj1", "ProjectA", EntityType::Project),
        ];

        let relationships = vec![create_test_relationship(
            &entities[0],
            &entities[2],
            RelationshipType::Maintains,
        )];

        let generator = ContextGenerator::new();
        let contexts = generator.generate(&entities, &relationships);

        let people_ctx = contexts
            .iter()
            .find(|c| c.context_file.concept == "people")
            .unwrap();

        // ProjectA should be in external relationships
        assert!(!people_ctx.external_relationships.is_empty());
    }
}
