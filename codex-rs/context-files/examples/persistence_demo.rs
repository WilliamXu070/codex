//! Demo of context tree persistence.
//!
//! Shows how to save and load context trees to/from disk.
//!
//! Usage: cargo run -p codex-context-files --example persistence_demo

use codex_context_files::{AgentBuilder, TreeStore};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("💾 Context Tree Persistence Demo\n");

    // Method 1: Use default location (~/.codex/context/)
    println!("📍 Method 1: Default Location");
    let default_store = TreeStore::default_location()?;
    println!("   Location: {}\n", default_store.base_path().display());

    // Method 2: Use custom location
    println!("📍 Method 2: Custom Location");
    let custom_path = PathBuf::from("./my_context_data");
    let custom_store = TreeStore::new(&custom_path);
    println!("   Location: {}\n", custom_store.base_path().display());

    // Create and populate an agent
    println!("🔨 Building context tree...");
    let mut agent = AgentBuilder::new().heuristic_only().build();

    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    // Process some content
    if fixtures_dir.join("cooking-recipes").exists() {
        agent
            .process_folder(&fixtures_dir.join("cooking-recipes"))
            .await?;
        println!("   ✓ Processed cooking recipes");
    }

    if fixtures_dir.join("work-notes").exists() {
        agent
            .process_folder(&fixtures_dir.join("work-notes"))
            .await?;
        println!("   ✓ Processed work notes");
    }

    let stats = agent.stats();
    println!("   ✓ Tree has {} nodes\n", stats.total_nodes);

    // Save to custom location
    println!("💾 Saving tree...");
    custom_store.save(agent.tree())?;
    println!("   ✓ Saved to: {}", custom_store.base_path().display());
    println!(
        "   ✓ File: {}",
        custom_store.base_path().join("tree.json").display()
    );

    // Check file size
    let tree_file = custom_store.base_path().join("tree.json");
    if tree_file.exists() {
        let metadata = std::fs::metadata(&tree_file)?;
        println!("   ✓ Size: {} KB\n", metadata.len() / 1024);
    }

    // Load it back
    println!("📂 Loading tree from disk...");
    let loaded_tree = custom_store.load()?;
    println!("   ✓ Loaded {} nodes", loaded_tree.node_count());
    println!("   ✓ Domains: {:?}\n", loaded_tree.list_domains());

    // Demonstrate incremental save (single node)
    println!("📝 Incremental Save (single node)...");
    if let Some(first_node) = loaded_tree.all_nodes().next() {
        custom_store.save_node(first_node)?;
        println!("   ✓ Saved node: {}", first_node.name);
        println!(
            "   ✓ Location: {}",
            custom_store
                .base_path()
                .join("nodes")
                .join(format!("{}.json", first_node.id))
                .display()
        );
    }
    println!();

    // Show JSON structure
    println!("📄 JSON Structure:");
    let json_content = std::fs::read_to_string(&tree_file)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_content)?;

    println!("   {{");
    println!("     \"version\": {},", parsed["version"]);
    println!(
        "     \"root_id\": \"{}\",",
        parsed["root_id"].as_str().unwrap()
    );
    println!(
        "     \"nodes\": [{} items],",
        parsed["nodes"].as_array().unwrap().len()
    );
    println!(
        "     \"domain_index\": {{...}}  // {} domains",
        parsed["domain_index"].as_object().unwrap().len()
    );
    println!("   }}\n");

    // Show a sample node
    if let Some(nodes) = parsed["nodes"].as_array() {
        if let Some(first_node) = nodes.first() {
            println!("📋 Sample Node:");
            println!("{}", serde_json::to_string_pretty(first_node).unwrap());
        }
    }

    println!("\n✅ Demo complete!");
    println!("\n💡 The tree is saved as JSON and can be:");
    println!("   • Backed up and versioned with git");
    println!("   • Shared between machines");
    println!("   • Inspected with any JSON viewer");
    println!("   • Migrated or exported to other formats");

    Ok(())
}
