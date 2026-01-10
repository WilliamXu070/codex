//! Context extraction pipeline orchestration.
//!
//! This module provides the main entry point for processing documents
//! and generating context files using the full extraction pipeline.

use std::path::{Path, PathBuf};

use tracing::{debug, info, warn};

use crate::chunker::{Chunk, ChunkerConfig, SemanticChunker};
use crate::context_file::ContextFile;
use crate::entity::{Entity, EntityExtractor, EntityExtractorConfig};
use crate::error::{ContextError, Result};
use crate::generator::{ContextGenerator, GeneratedContext, GeneratorConfig};
use crate::relationship::{Relationship, RelationshipExtractor, RelationshipExtractorConfig};
use crate::storage::ContextStore;

/// Configuration for the context extraction pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Chunker configuration.
    pub chunker: ChunkerConfig,

    /// Entity extractor configuration.
    pub entity: EntityExtractorConfig,

    /// Relationship extractor configuration.
    pub relationship: RelationshipExtractorConfig,

    /// Context generator configuration.
    pub generator: GeneratorConfig,

    /// File extensions to process.
    pub file_extensions: Vec<String>,

    /// Directories to skip.
    pub skip_directories: Vec<String>,

    /// Maximum file size to process (in bytes).
    pub max_file_size: usize,

    /// Whether to process hidden files.
    pub process_hidden: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            chunker: ChunkerConfig::default(),
            entity: EntityExtractorConfig::default(),
            relationship: RelationshipExtractorConfig::default(),
            generator: GeneratorConfig::default(),
            file_extensions: vec![
                "md".to_string(),
                "txt".to_string(),
                "rs".to_string(),
                "py".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "tsx".to_string(),
                "jsx".to_string(),
                "json".to_string(),
                "toml".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
            ],
            skip_directories: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "build".to_string(),
                "__pycache__".to_string(),
                ".venv".to_string(),
                "venv".to_string(),
            ],
            max_file_size: 1024 * 1024, // 1MB
            process_hidden: false,
        }
    }
}

/// Result of processing a single document.
#[derive(Debug, Clone)]
pub struct DocumentResult {
    /// Source file path.
    pub source: PathBuf,

    /// Chunks extracted from the document.
    pub chunks: Vec<Chunk>,

    /// Entities extracted.
    pub entities: Vec<Entity>,

    /// Relationships extracted.
    pub relationships: Vec<Relationship>,
}

/// Result of running the full pipeline.
#[derive(Debug)]
pub struct PipelineResult {
    /// All processed documents.
    pub documents: Vec<DocumentResult>,

    /// All entities across all documents.
    pub all_entities: Vec<Entity>,

    /// All relationships across all documents.
    pub all_relationships: Vec<Relationship>,

    /// Generated contexts.
    pub contexts: Vec<GeneratedContext>,

    /// Files that failed to process.
    pub errors: Vec<(PathBuf, String)>,

    /// Pipeline statistics.
    pub stats: PipelineStats,
}

/// Statistics about the pipeline run.
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    /// Number of files processed.
    pub files_processed: usize,

    /// Number of files skipped.
    pub files_skipped: usize,

    /// Number of files with errors.
    pub files_with_errors: usize,

    /// Total chunks created.
    pub total_chunks: usize,

    /// Total entities extracted.
    pub total_entities: usize,

    /// Total relationships extracted.
    pub total_relationships: usize,

    /// Total contexts generated.
    pub total_contexts: usize,

    /// Processing time in milliseconds.
    pub processing_time_ms: u64,
}

/// The main context extraction pipeline.
pub struct ContextPipeline {
    config: PipelineConfig,
    chunker: SemanticChunker,
    entity_extractor: EntityExtractor,
    relationship_extractor: RelationshipExtractor,
    context_generator: ContextGenerator,
}

impl ContextPipeline {
    /// Create a new pipeline with default configuration.
    pub fn new() -> Self {
        Self::with_config(PipelineConfig::default())
    }

    /// Create a new pipeline with custom configuration.
    pub fn with_config(config: PipelineConfig) -> Self {
        Self {
            chunker: SemanticChunker::with_config(config.chunker.clone()),
            entity_extractor: EntityExtractor::with_config(config.entity.clone()),
            relationship_extractor: RelationshipExtractor::with_config(config.relationship.clone()),
            context_generator: ContextGenerator::with_config(config.generator.clone()),
            config,
        }
    }

    /// Process a single document and return extracted information.
    pub fn process_document(&self, content: &str, source: Option<&Path>) -> Result<DocumentResult> {
        let source_path = source.map(|p| p.to_path_buf()).unwrap_or_default();
        let source_str = source.map(|p| p.to_string_lossy().to_string());

        debug!("Processing document: {:?}", source_path);

        // Step 1: Chunk the document
        let chunks = if let Some(ref src) = source_str {
            self.chunker.chunk_with_source(content, src)
        } else {
            self.chunker.chunk(content)
        };
        debug!("Created {} chunks", chunks.len());

        // Step 2: Extract entities from chunks
        let entities = self.entity_extractor.extract(&chunks);
        debug!("Extracted {} entities", entities.len());

        // Step 3: Extract relationships
        let relationships = self.relationship_extractor.extract(&entities, &chunks);
        debug!("Extracted {} relationships", relationships.len());

        Ok(DocumentResult {
            source: source_path,
            chunks,
            entities,
            relationships,
        })
    }

    /// Process a directory of files.
    pub fn process_directory(&self, dir: &Path) -> Result<PipelineResult> {
        let start_time = std::time::Instant::now();

        info!("Processing directory: {:?}", dir);

        let mut documents = Vec::new();
        let mut errors = Vec::new();
        let mut stats = PipelineStats::default();

        // Collect files to process
        let files = self.collect_files(dir)?;
        info!("Found {} files to process", files.len());

        for file_path in files {
            match self.process_file(&file_path) {
                Ok(doc_result) => {
                    stats.total_chunks += doc_result.chunks.len();
                    stats.total_entities += doc_result.entities.len();
                    stats.total_relationships += doc_result.relationships.len();
                    stats.files_processed += 1;
                    documents.push(doc_result);
                }
                Err(e) => {
                    warn!("Failed to process {:?}: {}", file_path, e);
                    errors.push((file_path, e.to_string()));
                    stats.files_with_errors += 1;
                }
            }
        }

        // Aggregate all entities and relationships
        let mut all_entities: Vec<Entity> = documents
            .iter()
            .flat_map(|d| d.entities.clone())
            .collect();
        let mut all_relationships: Vec<Relationship> = documents
            .iter()
            .flat_map(|d| d.relationships.clone())
            .collect();

        // Deduplicate across documents
        all_entities = deduplicate_entities(all_entities);
        all_relationships = deduplicate_relationships(all_relationships);

        stats.total_entities = all_entities.len();
        stats.total_relationships = all_relationships.len();

        // Generate contexts
        let contexts = self
            .context_generator
            .generate(&all_entities, &all_relationships);
        stats.total_contexts = contexts.len();

        stats.processing_time_ms = start_time.elapsed().as_millis() as u64;

        info!(
            "Pipeline complete: {} files, {} entities, {} relationships, {} contexts in {}ms",
            stats.files_processed,
            stats.total_entities,
            stats.total_relationships,
            stats.total_contexts,
            stats.processing_time_ms
        );

        Ok(PipelineResult {
            documents,
            all_entities,
            all_relationships,
            contexts,
            errors,
            stats,
        })
    }

    /// Process a single file.
    fn process_file(&self, path: &Path) -> Result<DocumentResult> {
        let metadata = std::fs::metadata(path)?;

        if metadata.len() > self.config.max_file_size as u64 {
            return Err(ContextError::InvalidFormat(format!(
                "File too large: {} bytes",
                metadata.len()
            )));
        }

        let content = std::fs::read_to_string(path)?;
        self.process_document(&content, Some(path))
    }

    /// Collect files to process from a directory.
    fn collect_files(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        self.collect_files_recursive(dir, &mut files)?;

        Ok(files)
    }

    /// Recursively collect files.
    fn collect_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();

            // Skip hidden files/directories
            if !self.config.process_hidden && file_name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                // Skip configured directories
                if self.config.skip_directories.contains(&file_name.to_string()) {
                    continue;
                }
                self.collect_files_recursive(&path, files)?;
            } else if path.is_file() {
                // Check extension
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if self.config.file_extensions.contains(&ext.to_string()) {
                        files.push(path);
                    }
                }
            }
        }

        Ok(())
    }

    /// Save generated contexts to a store.
    pub async fn save_contexts(
        &self,
        contexts: &[GeneratedContext],
        store: &mut ContextStore,
    ) -> Result<usize> {
        let mut saved = 0;

        for ctx in contexts {
            store.upsert(ctx.context_file.clone()).await?;
            saved += 1;
        }

        info!("Saved {} context files to store", saved);
        Ok(saved)
    }

    /// Get context files from generated contexts.
    pub fn get_context_files(&self, contexts: &[GeneratedContext]) -> Vec<ContextFile> {
        contexts.iter().map(|c| c.context_file.clone()).collect()
    }
}

impl Default for ContextPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Deduplicate entities by normalized name.
fn deduplicate_entities(entities: Vec<Entity>) -> Vec<Entity> {
    use std::collections::HashMap;

    let mut seen: HashMap<String, Entity> = HashMap::new();

    for entity in entities {
        let key = format!("{:?}:{}", entity.entity_type, entity.normalized_name);

        if let Some(existing) = seen.get_mut(&key) {
            // Merge mentions
            existing.mentions.extend(entity.mentions);
            // Keep higher confidence
            if entity.confidence > existing.confidence {
                existing.confidence = entity.confidence;
            }
            // Merge attributes
            for (k, v) in entity.attributes {
                existing.attributes.entry(k).or_insert(v);
            }
        } else {
            seen.insert(key, entity);
        }
    }

    seen.into_values().collect()
}

/// Deduplicate relationships.
fn deduplicate_relationships(relationships: Vec<Relationship>) -> Vec<Relationship> {
    use std::collections::HashMap;

    let mut seen: HashMap<String, Relationship> = HashMap::new();

    for rel in relationships {
        let key = format!(
            "{}:{}:{:?}",
            rel.source_id, rel.target_id, rel.relationship_type
        );

        if let Some(existing) = seen.get_mut(&key) {
            // Merge evidence
            existing.evidence.extend(rel.evidence);
            // Keep higher confidence
            if rel.confidence > existing.confidence {
                existing.confidence = rel.confidence;
            }
        } else {
            seen.insert(key, rel);
        }
    }

    seen.into_values().collect()
}

/// Builder for pipeline configuration.
pub struct PipelineBuilder {
    config: PipelineConfig,
}

impl PipelineBuilder {
    /// Create a new builder with default configuration.
    pub fn new() -> Self {
        Self {
            config: PipelineConfig::default(),
        }
    }

    /// Set the target chunk size in tokens.
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.config.chunker.target_tokens = size;
        self
    }

    /// Set the chunk overlap fraction.
    pub fn with_chunk_overlap(mut self, overlap: f32) -> Self {
        self.config.chunker.overlap_fraction = overlap;
        self
    }

    /// Set the minimum entity confidence.
    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.config.entity.min_confidence = confidence;
        self
    }

    /// Add file extensions to process.
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.config.file_extensions = extensions;
        self
    }

    /// Add directories to skip.
    pub fn with_skip_dirs(mut self, dirs: Vec<String>) -> Self {
        self.config.skip_directories = dirs;
        self
    }

    /// Set maximum file size.
    pub fn with_max_file_size(mut self, size: usize) -> Self {
        self.config.max_file_size = size;
        self
    }

    /// Enable processing of hidden files.
    pub fn with_hidden_files(mut self, process: bool) -> Self {
        self.config.process_hidden = process;
        self
    }

    /// Set source identifier for generated contexts.
    pub fn with_source_id(mut self, source_id: String) -> Self {
        self.config.generator.source_id = Some(source_id);
        self
    }

    /// Build the pipeline.
    pub fn build(self) -> ContextPipeline {
        ContextPipeline::with_config(self.config)
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_process_document() {
        let pipeline = ContextPipeline::new();

        let content = r#"
# Test Project

Created by Alice Smith on 2024-01-15.

This project uses Rust and Python for data processing.

## Features

- Fast processing with tokio
- Data analysis with pandas
"#;

        let result = pipeline.process_document(content, None).unwrap();

        assert!(!result.chunks.is_empty());
        assert!(!result.entities.is_empty());

        // Should find Alice Smith
        let people: Vec<_> = result
            .entities
            .iter()
            .filter(|e| matches!(e.entity_type, crate::entity::EntityType::Person))
            .collect();
        assert!(!people.is_empty());

        // Should find Rust and Python
        let techs: Vec<_> = result
            .entities
            .iter()
            .filter(|e| matches!(e.entity_type, crate::entity::EntityType::Technology))
            .collect();
        assert!(techs.len() >= 2);
    }

    #[test]
    fn test_process_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        std::fs::write(
            temp_dir.path().join("README.md"),
            "# My Project\nCreated by Bob.\nUses Rust.",
        )
        .unwrap();

        std::fs::write(
            temp_dir.path().join("config.toml"),
            "[package]\nname = \"test\"\nversion = \"1.0.0\"",
        )
        .unwrap();

        let pipeline = ContextPipeline::new();
        let result = pipeline.process_directory(temp_dir.path()).unwrap();

        assert_eq!(result.stats.files_processed, 2);
        assert!(!result.all_entities.is_empty());
        assert!(!result.contexts.is_empty());
    }

    #[test]
    fn test_pipeline_builder() {
        let pipeline = PipelineBuilder::new()
            .with_chunk_size(256)
            .with_min_confidence(0.5)
            .with_extensions(vec!["md".to_string(), "txt".to_string()])
            .build();

        assert_eq!(pipeline.config.chunker.target_tokens, 256);
        assert_eq!(pipeline.config.entity.min_confidence, 0.5);
        assert_eq!(pipeline.config.file_extensions.len(), 2);
    }

    #[test]
    fn test_skip_directories() {
        let temp_dir = TempDir::new().unwrap();

        // Create a node_modules directory (should be skipped)
        let node_modules = temp_dir.path().join("node_modules");
        std::fs::create_dir(&node_modules).unwrap();
        std::fs::write(node_modules.join("test.js"), "// This should be skipped").unwrap();

        // Create a regular file
        std::fs::write(
            temp_dir.path().join("main.js"),
            "// Main file\nconsole.log('hello');",
        )
        .unwrap();

        let pipeline = ContextPipeline::new();
        let result = pipeline.process_directory(temp_dir.path()).unwrap();

        // Only main.js should be processed
        assert_eq!(result.stats.files_processed, 1);
    }

    #[test]
    fn test_entity_deduplication() {
        use crate::entity::{EntityMention, EntityType};

        let entities = vec![
            Entity {
                id: "1".to_string(),
                name: "Rust".to_string(),
                normalized_name: "rust".to_string(),
                entity_type: EntityType::Technology,
                confidence: 0.8,
                mentions: vec![EntityMention {
                    chunk_id: "c1".to_string(),
                    position: 0,
                    matched_text: "Rust".to_string(),
                    context: Some("Uses Rust".to_string()),
                }],
                attributes: std::collections::HashMap::new(),
            },
            Entity {
                id: "2".to_string(),
                name: "rust".to_string(),
                normalized_name: "rust".to_string(),
                entity_type: EntityType::Technology,
                confidence: 0.9,
                mentions: vec![EntityMention {
                    chunk_id: "c2".to_string(),
                    position: 10,
                    matched_text: "rust".to_string(),
                    context: Some("built with rust".to_string()),
                }],
                attributes: std::collections::HashMap::new(),
            },
        ];

        let deduped = deduplicate_entities(entities);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].mentions.len(), 2);
        assert_eq!(deduped[0].confidence, 0.9); // Higher confidence kept
    }
}
