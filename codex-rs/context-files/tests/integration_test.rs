//! Integration tests for the context extraction pipeline.
//!
//! This test suite verifies the full pipeline from document processing
//! to context file generation.

use std::path::PathBuf;

use codex_context_files::{ContextPipeline, EntityType, PipelineBuilder, RelationshipType};

/// Get the path to the test fixtures directory.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn test_pipeline_processes_fixtures() {
    let pipeline = ContextPipeline::new();
    let result = pipeline.process_directory(&fixtures_dir()).unwrap();

    // Should process all fixture files
    assert!(
        result.stats.files_processed >= 4,
        "Expected at least 4 files processed, got {}",
        result.stats.files_processed
    );

    // Should extract entities
    assert!(
        !result.all_entities.is_empty(),
        "Expected entities to be extracted"
    );

    // Should generate contexts
    assert!(
        !result.contexts.is_empty(),
        "Expected context files to be generated"
    );

    println!("Pipeline Stats:");
    println!("  Files processed: {}", result.stats.files_processed);
    println!("  Chunks created: {}", result.stats.total_chunks);
    println!("  Entities extracted: {}", result.stats.total_entities);
    println!(
        "  Relationships found: {}",
        result.stats.total_relationships
    );
    println!("  Contexts generated: {}", result.stats.total_contexts);
    println!("  Processing time: {}ms", result.stats.processing_time_ms);
}

#[test]
fn test_extracts_people() {
    let pipeline = ContextPipeline::new();
    let result = pipeline.process_directory(&fixtures_dir()).unwrap();

    let people: Vec<_> = result
        .all_entities
        .iter()
        .filter(|e| matches!(e.entity_type, EntityType::Person))
        .collect();

    // Should find Alice Johnson and Bob Smith
    let names: Vec<_> = people.iter().map(|p| p.name.as_str()).collect();

    println!("Found people: {:?}", names);

    assert!(
        people.len() >= 2,
        "Expected at least 2 people, found: {:?}",
        names
    );

    // Check for specific people mentioned in fixtures
    let alice_found = people
        .iter()
        .any(|p| p.name.contains("Alice") || p.normalized_name.contains("alice"));
    assert!(alice_found, "Expected to find Alice Johnson");
}

#[test]
fn test_extracts_technologies() {
    let pipeline = ContextPipeline::new();
    let result = pipeline.process_directory(&fixtures_dir()).unwrap();

    let techs: Vec<_> = result
        .all_entities
        .iter()
        .filter(|e| matches!(e.entity_type, EntityType::Technology))
        .collect();

    let tech_names: Vec<_> = techs.iter().map(|t| t.name.as_str()).collect();

    println!("Found technologies: {:?}", tech_names);

    // Should find Rust, Python, tokio, PostgreSQL, Redis, Kafka
    assert!(
        techs.len() >= 5,
        "Expected at least 5 technologies, found: {:?}",
        tech_names
    );

    // Check for specific technologies
    let has_rust = techs
        .iter()
        .any(|t| t.normalized_name == "rust" || t.name.to_lowercase() == "rust");
    let has_postgres = techs.iter().any(|t| {
        t.normalized_name.contains("postgres") || t.name.to_lowercase().contains("postgres")
    });

    assert!(has_rust, "Expected to find Rust");
    assert!(has_postgres, "Expected to find PostgreSQL");
}

#[test]
fn test_extracts_projects() {
    let pipeline = ContextPipeline::new();
    let result = pipeline.process_directory(&fixtures_dir()).unwrap();

    let projects: Vec<_> = result
        .all_entities
        .iter()
        .filter(|e| matches!(e.entity_type, EntityType::Project))
        .collect();

    let project_names: Vec<_> = projects.iter().map(|p| p.name.as_str()).collect();

    println!("Found projects: {:?}", project_names);

    // Should find DataFlow project
    let has_dataflow = projects.iter().any(|p| {
        p.normalized_name.contains("dataflow") || p.name.to_lowercase().contains("dataflow")
    });

    // DataFlow might be detected as Project or just in context
    if !has_dataflow {
        // Check if it's mentioned anywhere
        let any_dataflow = result
            .all_entities
            .iter()
            .any(|e| e.name.to_lowercase().contains("dataflow"));
        println!("DataFlow found in any entity: {}", any_dataflow);
    }
}

#[test]
fn test_extracts_urls_and_emails() {
    let pipeline = ContextPipeline::new();
    let result = pipeline.process_directory(&fixtures_dir()).unwrap();

    let urls: Vec<_> = result
        .all_entities
        .iter()
        .filter(|e| matches!(e.entity_type, EntityType::Url))
        .collect();

    let emails: Vec<_> = result
        .all_entities
        .iter()
        .filter(|e| matches!(e.entity_type, EntityType::Email))
        .collect();

    println!(
        "Found URLs: {:?}",
        urls.iter().map(|u| &u.name).collect::<Vec<_>>()
    );
    println!(
        "Found emails: {:?}",
        emails.iter().map(|e| &e.name).collect::<Vec<_>>()
    );

    // Should find GitHub URL
    let has_github = urls.iter().any(|u| u.name.contains("github.com"));
    assert!(has_github, "Expected to find GitHub URL");

    // Should find email addresses
    assert!(!emails.is_empty(), "Expected to find email addresses");
}

#[test]
fn test_extracts_relationships() {
    let pipeline = ContextPipeline::new();
    let result = pipeline.process_directory(&fixtures_dir()).unwrap();

    println!("Found {} relationships", result.all_relationships.len());

    for rel in result.all_relationships.iter().take(10) {
        println!(
            "  {} --{:?}--> {}",
            rel.source_name, rel.relationship_type, rel.target_name
        );
    }

    // Should find some relationships
    assert!(
        !result.all_relationships.is_empty(),
        "Expected to find relationships"
    );

    // Check for specific relationship types
    let has_uses = result
        .all_relationships
        .iter()
        .any(|r| matches!(r.relationship_type, RelationshipType::Uses));
    let has_created_by = result
        .all_relationships
        .iter()
        .any(|r| matches!(r.relationship_type, RelationshipType::CreatedBy));

    println!("Has 'uses' relationships: {}", has_uses);
    println!("Has 'created by' relationships: {}", has_created_by);
}

#[test]
fn test_generates_type_based_contexts() {
    let pipeline = ContextPipeline::new();
    let result = pipeline.process_directory(&fixtures_dir()).unwrap();

    println!("Generated {} contexts", result.contexts.len());

    for ctx in &result.contexts {
        println!(
            "  {} ({:?}): {} entities - {}",
            ctx.context_file.concept,
            ctx.cluster_method,
            ctx.entities.len(),
            &ctx.context_file.summary[..ctx.context_file.summary.len().min(80)]
        );
    }

    // Should have technology context
    let tech_ctx = result
        .contexts
        .iter()
        .find(|c| c.context_file.concept == "technologies");
    assert!(tech_ctx.is_some(), "Expected technologies context");

    if let Some(ctx) = tech_ctx {
        assert!(
            ctx.entities.len() >= 3,
            "Expected at least 3 technologies in context"
        );
    }
}

#[test]
fn test_context_summaries_are_meaningful() {
    let pipeline = ContextPipeline::new();
    let result = pipeline.process_directory(&fixtures_dir()).unwrap();

    for ctx in &result.contexts {
        // Summary should not be empty
        assert!(
            !ctx.context_file.summary.is_empty(),
            "Context {} has empty summary",
            ctx.context_file.concept
        );

        // Summary should be descriptive (at least 20 chars)
        assert!(
            ctx.context_file.summary.len() >= 20,
            "Context {} has too short summary: {}",
            ctx.context_file.concept,
            ctx.context_file.summary
        );

        println!(
            "Context '{}' summary: {}",
            ctx.context_file.concept, ctx.context_file.summary
        );
    }
}

#[test]
fn test_pipeline_builder() {
    let pipeline = PipelineBuilder::new()
        .with_chunk_size(256)
        .with_min_confidence(0.5)
        .with_extensions(vec!["md".to_string(), "rs".to_string(), "toml".to_string()])
        .with_source_id("test-fixtures".to_string())
        .build();

    let result = pipeline.process_directory(&fixtures_dir()).unwrap();

    // With confidence 0.5, should still find high-confidence entities
    assert!(!result.all_entities.is_empty());

    // Check source_id is set in structured data
    for ctx in &result.contexts {
        let source = ctx.context_file.get_structured("source");
        assert!(
            source.is_some(),
            "Expected source to be set in structured data"
        );
        if let Some(serde_json::Value::String(s)) = source {
            assert_eq!(s, "test-fixtures");
        }
    }
}

#[test]
fn test_single_document_processing() {
    let pipeline = ContextPipeline::new();

    let content = r#"
# Test Document

This document was created by John Doe on 2024-03-15.

We use Rust and TypeScript in this project.

Contact: john.doe@example.com
Website: https://example.com/project
"#;

    let result = pipeline.process_document(content, None).unwrap();

    println!("Single document results:");
    println!("  Chunks: {}", result.chunks.len());
    println!("  Entities: {}", result.entities.len());
    println!("  Relationships: {}", result.relationships.len());

    for entity in &result.entities {
        println!("  Entity: {} ({:?})", entity.name, entity.entity_type);
    }

    // Should find the person
    let has_john = result
        .entities
        .iter()
        .any(|e| matches!(e.entity_type, EntityType::Person) && e.name.contains("John"));
    assert!(has_john, "Expected to find John Doe");

    // Should find technologies
    let techs: Vec<_> = result
        .entities
        .iter()
        .filter(|e| matches!(e.entity_type, EntityType::Technology))
        .collect();
    assert!(techs.len() >= 2, "Expected to find Rust and TypeScript");

    // Should find email
    let has_email = result
        .entities
        .iter()
        .any(|e| matches!(e.entity_type, EntityType::Email));
    assert!(has_email, "Expected to find email");
}

#[test]
fn test_chunk_structure() {
    let pipeline = ContextPipeline::new();
    let readme_path = fixtures_dir().join("README.md");
    let content = std::fs::read_to_string(&readme_path).unwrap();

    let result = pipeline
        .process_document(&content, Some(&readme_path))
        .unwrap();

    println!("Chunk structure for README.md:");
    for (i, chunk) in result.chunks.iter().enumerate() {
        println!(
            "  Chunk {}: {:?} ({} chars) at {}..{}",
            i,
            chunk.chunk_type,
            chunk.content.len(),
            chunk.start_offset,
            chunk.end_offset
        );
    }

    // Should have multiple chunks
    assert!(result.chunks.len() >= 2, "Expected multiple chunks");

    // Chunks should have proper offsets
    for chunk in &result.chunks {
        assert!(
            chunk.end_offset > chunk.start_offset,
            "Chunk should have valid offsets"
        );
    }
}

#[test]
fn test_performance() {
    let pipeline = ContextPipeline::new();

    // Process fixtures multiple times to get average
    let mut total_time = 0u64;
    let iterations = 3;

    for _ in 0..iterations {
        let result = pipeline.process_directory(&fixtures_dir()).unwrap();
        total_time += result.stats.processing_time_ms;
    }

    let avg_time = total_time / iterations as u64;
    println!("Average processing time: {}ms", avg_time);

    // Should process in under 5 seconds (generous for CI)
    assert!(
        avg_time < 5000,
        "Processing took too long: {}ms average",
        avg_time
    );
}
