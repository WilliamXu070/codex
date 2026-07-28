//! Test .docx file processing.
//!
//! Usage: cargo run -p codex-context-files --example test_docx

use codex_context_files::AgentBuilder;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("Testing .docx file processing\n");

    // Create an agent with heuristic-only mode
    let mut agent = AgentBuilder::new()
        .heuristic_only()
        .build();

    // Process the Gambling folder
    let gambling_path = PathBuf::from(r"C:\Users\William\Desktop\Testing\Gambling");

    println!("Path: {}", gambling_path.display());
    println!("Exists: {}", gambling_path.exists());

    // List files in directory
    println!("\nFiles in directory:");
    for entry in std::fs::read_dir(&gambling_path)? {
        let entry = entry?;
        println!("  {:?}", entry.path());
    }

    println!("\nProcessing folder...");
    let result = agent.process_folder(&gambling_path).await?;

    println!("\nResults:");
    println!("  Domain: {}", result.domain);
    println!("  Files processed: {}", result.files_processed);
    println!("  Nodes created: {}", result.nodes_created);
    println!("  Entities extracted: {}", result.entities_extracted);
    println!("  Processing time: {}ms", result.processing_time_ms);

    if !result.errors.is_empty() {
        println!("\nErrors:");
        for error in &result.errors {
            println!("  {}", error);
        }
    }

    // Show tree stats
    println!("\nTree Statistics:");
    let stats = agent.stats();
    println!("{}", stats);

    // Show all nodes
    println!("\nAll nodes:");
    for node in agent.tree().all_nodes() {
        println!("  {} ({}) - children: {:?}",
            node.name,
            node.node_type.label(),
            node.children.len()
        );
    }

    Ok(())
}
