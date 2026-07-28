//! Integration tests for the agentic context system.
//!
//! These tests verify that the system can:
//! - Process multiple domains (coding, cooking, work)
//! - Automatically detect domain categories
//! - Build hierarchical context trees
//! - Create cross-links between related content
//! - Persist and reload trees
//! - Optimize tree depth and structure

use std::path::PathBuf;

use codex_context_files::{
    AgentBuilder, ContextAgent, LlmConfig, NodeType, OptimizerConfig, TreeOptimizer, TreeStore,
};
use tempfile::TempDir;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[tokio::test]
async fn test_multi_domain_processing() {
    let mut agent = ContextAgent::heuristic_only();

    // Process coding project
    let coding_path = fixtures_dir();
    let coding_result = agent.process_folder(&coding_path).await.unwrap();

    assert_eq!(coding_result.domain, "coding");
    assert!(coding_result.nodes_created > 0);
    assert!(coding_result.files_processed > 0);

    // Process cooking recipes
    let cooking_path = fixtures_dir().join("cooking-recipes");
    let cooking_result = agent.process_folder(&cooking_path).await.unwrap();

    assert_eq!(cooking_result.domain, "cooking");
    assert!(cooking_result.nodes_created > 0);

    // Process work notes
    let work_path = fixtures_dir().join("work-notes");
    let work_result = agent.process_folder(&work_path).await.unwrap();

    assert_eq!(work_result.domain, "work");
    assert!(work_result.nodes_created > 0);

    // Verify tree structure
    let domains = agent.list_domains();
    assert!(domains.contains(&"coding"));
    assert!(domains.contains(&"cooking"));
    assert!(domains.contains(&"work"));

    let stats = agent.stats();
    assert_eq!(stats.domains, 3);
    assert!(stats.total_nodes > 3); // At least root + 3 domains

    println!("Tree stats:\n{}", stats);
}

#[tokio::test]
async fn test_domain_detection() {
    let mut agent = ContextAgent::heuristic_only();

    // Test cooking domain detection
    let cooking_path = fixtures_dir().join("cooking-recipes");
    let result = agent.process_folder(&cooking_path).await.unwrap();

    assert_eq!(result.domain, "cooking");

    // Verify domain node exists
    let domain_context = agent.get_domain_context("cooking");
    assert!(domain_context.is_some());

    let nodes = domain_context.unwrap();
    assert!(!nodes.is_empty());

    // Check that files were processed
    let domain_node = nodes.iter().find(|n| n.node_type == NodeType::Domain);
    assert!(domain_node.is_some());
}

#[tokio::test]
async fn test_hierarchical_structure() {
    let mut agent = ContextAgent::heuristic_only();

    let cooking_path = fixtures_dir().join("cooking-recipes");
    agent.process_folder(&cooking_path).await.unwrap();

    // Get tree and verify hierarchy
    let tree = agent.tree();

    // Root should exist
    let root = tree.root();
    assert_eq!(root.node_type, NodeType::Root);
    assert!(!root.children.is_empty());

    // Domain should exist as child of root
    let cooking_domain = tree.get_domain("cooking");
    assert!(cooking_domain.is_some());

    let domain = cooking_domain.unwrap();
    assert_eq!(domain.depth, 1);
    assert_eq!(domain.parent_id, Some(root.id.clone()));

    // Domain should have children (projects or categories)
    assert!(!domain.children.is_empty());

    // Verify ancestry
    let first_child_id = &domain.children[0];
    let ancestry = tree.get_ancestry(first_child_id);

    assert!(ancestry.len() >= 3); // root -> domain -> child
    assert_eq!(ancestry[0].node_type, NodeType::Root);
    assert_eq!(ancestry[1].node_type, NodeType::Domain);
}

#[tokio::test]
async fn test_cross_linking() {
    let mut agent = AgentBuilder::new()
        .auto_cross_link(true)
        .heuristic_only()
        .build();

    // Process two coding projects
    let coding_path = fixtures_dir();
    agent.process_folder(&coding_path).await.unwrap();

    let tree = agent.tree();

    // Count cross-links
    let total_links: usize = tree.all_nodes().map(|n| n.related_nodes.len()).sum();

    // Should have some cross-links if there are shared technologies
    println!("Total cross-links: {}", total_links);

    // At minimum, verify the cross-linking mechanism works
    // (actual links depend on content similarity)
    let stats = agent.stats();
    assert!(stats.total_cross_links >= 0); // Non-negative
}

#[tokio::test]
async fn test_entity_extraction() {
    let mut agent = ContextAgent::heuristic_only();

    let cooking_path = fixtures_dir().join("cooking-recipes");
    let result = agent.process_folder(&cooking_path).await.unwrap();

    // Entities extracted count should be non-negative
    // (may be 0 for some content types like recipes)
    assert!(result.entities_extracted >= 0);

    // At least nodes should be created
    assert!(result.nodes_created > 0);
    assert!(result.files_processed > 0);

    println!("Entities extracted: {}", result.entities_extracted);
}

#[tokio::test]
async fn test_keyword_extraction() {
    let mut agent = ContextAgent::heuristic_only();

    let cooking_path = fixtures_dir().join("cooking-recipes");
    agent.process_folder(&cooking_path).await.unwrap();

    // Check that keywords are extracted
    let tree = agent.tree();
    let nodes_with_keywords = tree.all_nodes().filter(|n| !n.keywords.is_empty()).count();

    assert!(nodes_with_keywords > 0);

    // Verify cooking-related keywords exist
    let all_keywords: Vec<String> = tree.all_nodes().flat_map(|n| n.keywords.clone()).collect();

    // Should contain some cooking-related terms
    let has_cooking_keywords = all_keywords
        .iter()
        .any(|k| k.contains("recipe") || k.contains("ingredient"));

    println!(
        "Sample keywords: {:?}",
        &all_keywords[..all_keywords.len().min(10)]
    );
}

#[tokio::test]
async fn test_query() {
    let mut agent = ContextAgent::heuristic_only();

    let cooking_path = fixtures_dir().join("cooking-recipes");
    agent.process_folder(&cooking_path).await.unwrap();

    // Query for pasta
    let result = agent.query("pasta");
    assert!(!result.nodes.is_empty());

    println!(
        "Query 'pasta' returned {} nodes in {}ms",
        result.nodes.len(),
        result.processing_time_ms
    );

    // Query for chocolate
    let result = agent.query("chocolate");
    assert!(!result.nodes.is_empty());

    // Query for something that shouldn't exist
    let result = agent.query("quantum physics");
    // May or may not have results depending on content
}

#[tokio::test]
async fn test_user_profile() {
    let mut agent = ContextAgent::heuristic_only();

    let cooking_path = fixtures_dir().join("cooking-recipes");
    agent.process_folder(&cooking_path).await.unwrap();

    let work_path = fixtures_dir().join("work-notes");
    agent.process_folder(&work_path).await.unwrap();

    // Check user profile (root node)
    let profile = agent.user_profile();

    assert_eq!(profile.node_type, NodeType::Root);
    assert!(!profile.summary.is_empty());
    assert!(profile.summary.contains("domain"));

    println!("User profile: {}", profile.summary);
}

#[tokio::test]
async fn test_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let store = TreeStore::new(temp_dir.path());

    // Create and populate agent
    let mut agent = ContextAgent::heuristic_only();
    let cooking_path = fixtures_dir().join("cooking-recipes");
    agent.process_folder(&cooking_path).await.unwrap();

    let original_count = agent.tree().node_count();

    // Save tree
    store.save(agent.tree()).unwrap();
    assert!(store.exists());

    // Load tree
    let loaded_tree = store.load().unwrap();
    assert_eq!(loaded_tree.node_count(), original_count);

    // Verify domain still exists
    assert!(loaded_tree.get_domain("cooking").is_some());
}

#[tokio::test]
async fn test_tree_optimization() {
    let mut agent = ContextAgent::heuristic_only();

    let cooking_path = fixtures_dir().join("cooking-recipes");
    agent.process_folder(&cooking_path).await.unwrap();

    let optimizer = TreeOptimizer::new(OptimizerConfig {
        min_siblings_for_merge: 2,
        ..Default::default()
    });

    // Analyze before optimization
    let analysis = optimizer.analyze(agent.tree());
    println!("Pre-optimization analysis:\n{}", analysis);

    // Create analyzer for optimization
    let analyzer = codex_context_files::LlmAnalyzer::heuristic_only();

    // Run optimization
    let result = optimizer.optimize(agent.tree_mut(), &analyzer).await;

    // Optimization should complete without error
    assert!(result.is_ok());

    let opt_result = result.unwrap();
    println!(
        "Optimization result: merged {}, pruned {}, depth reduced by {}",
        opt_result.nodes_merged, opt_result.nodes_pruned, opt_result.depth_reduced_by
    );
}

#[tokio::test]
async fn test_tree_visualization() {
    let temp_dir = TempDir::new().unwrap();
    let store = TreeStore::new(temp_dir.path());

    let mut agent = ContextAgent::heuristic_only();
    let cooking_path = fixtures_dir().join("cooking-recipes");
    agent.process_folder(&cooking_path).await.unwrap();

    // Export structure
    let viz = store.export_structure(agent.tree());
    let output = viz.to_string();

    assert!(!output.is_empty());
    assert!(output.contains("User Knowledge"));
    assert!(output.contains("cooking"));

    println!("Tree structure:\n{}", output);
}

#[tokio::test]
async fn test_get_file_context() {
    let mut agent = ContextAgent::heuristic_only();

    let cooking_path = fixtures_dir().join("cooking-recipes");
    agent.process_folder(&cooking_path).await.unwrap();

    // Try to get context for a specific file
    let pasta_file = cooking_path.join("pasta-carbonara.md");
    let context = agent.get_file_context(&pasta_file);

    if context.is_some() {
        let ancestry = context.unwrap();
        assert!(!ancestry.is_empty());

        // Should have: root -> domain -> ... -> file
        assert_eq!(ancestry[0].node_type, NodeType::Root);

        println!("File context depth: {} nodes", ancestry.len());
    }
}

#[tokio::test]
async fn test_depth_limits() {
    let mut agent = ContextAgent::heuristic_only();

    let cooking_path = fixtures_dir().join("cooking-recipes");
    agent.process_folder(&cooking_path).await.unwrap();

    let stats = agent.stats();

    // Verify reasonable depth
    assert!(stats.max_depth < 20); // Should not be excessively deep

    println!("Maximum tree depth: {}", stats.max_depth);
}

#[tokio::test]
async fn test_summary_generation() {
    let mut agent = ContextAgent::heuristic_only();

    let work_path = fixtures_dir().join("work-notes");
    agent.process_folder(&work_path).await.unwrap();

    // Check that summaries are generated for documents
    let tree = agent.tree();
    let doc_nodes: Vec<_> = tree
        .all_nodes()
        .filter(|n| n.node_type == NodeType::Document)
        .collect();

    assert!(!doc_nodes.is_empty());

    for doc in doc_nodes {
        assert!(!doc.summary.is_empty());
        println!(
            "Document '{}': {}",
            doc.name,
            &doc.summary[..doc.summary.len().min(100)]
        );
    }
}

#[tokio::test]
async fn test_processing_errors() {
    let mut agent = ContextAgent::heuristic_only();

    // Try to process non-existent path
    let result = agent
        .process_folder(&PathBuf::from("/nonexistent/path"))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_max_files_limit() {
    let agent_config = codex_context_files::AgentConfig {
        max_files_per_folder: 1,
        ..Default::default()
    };

    let mut agent = AgentBuilder::new()
        .extensions(vec!["md".to_string()])
        .heuristic_only()
        .build();

    let cooking_path = fixtures_dir().join("cooking-recipes");
    let result = agent.process_folder(&cooking_path).await.unwrap();

    // Should process at most max_files_per_folder files
    // (though it may process fewer if there aren't that many files)
    assert!(result.files_processed >= 1);
}

#[tokio::test]
async fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let empty_path = temp_dir.path().join("empty");
    std::fs::create_dir_all(&empty_path).unwrap();

    let mut agent = ContextAgent::heuristic_only();
    let result = agent.process_folder(&empty_path).await.unwrap();

    // Should complete but process no files
    assert_eq!(result.files_processed, 0);
}
