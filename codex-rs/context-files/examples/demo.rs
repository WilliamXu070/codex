//! Demo of the agentic context system.
//!
//! Usage: cargo run -p codex-context-files --example demo

use codex_context_files::{AgentBuilder, TreeStore};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logs
    tracing_subscriber::fmt::init();

    println!("🚀 Codex Agentic Context System Demo\n");

    // Create an agent with heuristic-only mode (no LLM required)
    let mut agent = AgentBuilder::new()
        .auto_cross_link(true)
        .min_confidence(0.3)
        .heuristic_only()
        .build();

    // Get the test fixtures directory
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    // Process cooking recipes domain
    println!("📁 Processing cooking recipes...");
    let cooking_path = fixtures_dir.join("cooking-recipes");
    if cooking_path.exists() {
        let result = agent.process_folder(&cooking_path).await?;
        println!("   ✓ Domain: {}", result.domain);
        println!("   ✓ Files processed: {}", result.files_processed);
        println!("   ✓ Nodes created: {}", result.nodes_created);
        println!("   ✓ Entities extracted: {}", result.entities_extracted);
        println!("   ✓ Processing time: {}ms\n", result.processing_time_ms);
    }

    // Process work notes domain
    println!("📁 Processing work notes...");
    let work_path = fixtures_dir.join("work-notes");
    if work_path.exists() {
        let result = agent.process_folder(&work_path).await?;
        println!("   ✓ Domain: {}", result.domain);
        println!("   ✓ Files processed: {}", result.files_processed);
        println!("   ✓ Nodes created: {}", result.nodes_created);
        println!("   ✓ Processing time: {}ms\n", result.processing_time_ms);
    }

    // Process the main fixtures (coding)
    println!("📁 Processing coding project...");
    let result = agent.process_folder(&fixtures_dir).await?;
    println!("   ✓ Domain: {}", result.domain);
    println!("   ✓ Files processed: {}", result.files_processed);
    println!("   ✓ Nodes created: {}", result.nodes_created);
    println!("   ✓ Processing time: {}ms\n", result.processing_time_ms);

    // Show tree statistics
    println!("📊 Tree Statistics:");
    let stats = agent.stats();
    println!("{}\n", stats);

    // List all domains
    println!("🌍 Detected Domains:");
    for domain in agent.list_domains() {
        println!("   • {}", domain);
        if let Some(context) = agent.get_domain_context(domain) {
            println!("     {} nodes in this domain", context.len());
        }
    }
    println!();

    // Query the tree
    println!("🔍 Query Examples:");

    let pasta_query = agent.query("pasta");
    println!(
        "   'pasta' → {} results in {}ms",
        pasta_query.nodes.len(),
        pasta_query.processing_time_ms
    );
    for node in pasta_query.nodes.iter().take(3) {
        println!("      • {} ({})", node.name, node.node_type.label());
    }

    let meeting_query = agent.query("meeting");
    println!(
        "   'meeting' → {} results in {}ms",
        meeting_query.nodes.len(),
        meeting_query.processing_time_ms
    );
    for node in meeting_query.nodes.iter().take(3) {
        println!("      • {} ({})", node.name, node.node_type.label());
    }
    println!();

    // Show user profile (root node)
    println!("👤 User Profile:");
    let profile = agent.user_profile();
    println!("   {}\n", profile.summary);

    // Export tree structure
    println!("🌳 Tree Structure:");
    let temp_dir = tempfile::tempdir()?;
    let store = TreeStore::new(temp_dir.path());
    let viz = store.export_structure(agent.tree());

    // Print first 30 lines of tree structure
    let tree_str = viz.to_string();
    let lines: Vec<&str> = tree_str.lines().collect();
    for line in lines.iter().take(30) {
        println!("{}", line);
    }
    if lines.len() > 30 {
        println!("   ... ({} more lines)", lines.len() - 30);
    }
    println!();

    // Save the tree
    println!("💾 Saving tree to disk...");
    store.save(agent.tree())?;
    println!("   ✓ Saved to: {}\n", store.base_path().display());

    // Show optimization recommendations
    println!("⚡ Optimization Analysis:");
    let optimizer = codex_context_files::TreeOptimizer::default();
    let analysis = optimizer.analyze(agent.tree());
    println!("{}", analysis);

    println!("\n✅ Demo complete!");

    Ok(())
}
