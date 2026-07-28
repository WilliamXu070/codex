//! Persistent storage for the context tree.
//!
//! The `TreeStore` handles saving and loading the context tree to/from disk,
//! enabling persistence across sessions.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{ContextError, Result};
use crate::node::ContextNode;
use crate::tree::ContextTree;

/// Storage format for the context tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TreeData {
    /// Version of the storage format.
    version: u32,

    /// ID of the root node.
    root_id: String,

    /// All nodes in the tree.
    nodes: Vec<ContextNode>,

    /// Domain name to node ID mapping.
    domain_index: HashMap<String, String>,
}

impl TreeData {
    const CURRENT_VERSION: u32 = 1;

    fn from_tree(tree: &ContextTree) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            root_id: tree.root().id.clone(),
            nodes: tree.all_nodes().cloned().collect(),
            domain_index: tree
                .list_domains()
                .iter()
                .filter_map(|domain| {
                    tree.get_domain(domain)
                        .map(|node| (domain.to_string(), node.id.clone()))
                })
                .collect(),
        }
    }

    fn into_tree(self) -> Result<ContextTree> {
        // If no nodes were stored, return a fresh tree
        if self.nodes.is_empty() {
            info!("Stored tree is empty, creating fresh tree");
            return Ok(ContextTree::new());
        }

        // Check if root node exists in the stored nodes
        let has_root = self.nodes.iter().any(|n| n.id == self.root_id);
        if !has_root {
            warn!("Stored tree missing root node '{}', creating fresh tree", self.root_id);
            return Ok(ContextTree::new());
        }

        let mut tree = ContextTree::new();

        // Clear the default root
        let default_root_id = tree.root().id.clone();

        // Insert all nodes
        for node in self.nodes {
            tree.insert(node);
        }

        // Remove the default root if it's different from the stored one
        if default_root_id != self.root_id {
            tree.remove(&default_root_id);
        }

        // Final safety check - ensure root exists
        if tree.get(&self.root_id).is_none() {
            warn!("Tree reconstruction failed, creating fresh tree");
            return Ok(ContextTree::new());
        }

        Ok(tree)
    }
}

/// Persistent storage for context trees.
///
/// Supports saving and loading trees to/from a directory structure.
pub struct TreeStore {
    /// Base directory for storage.
    base_path: PathBuf,
}

impl TreeStore {
    /// Create a new tree store at the given path.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    /// Create a tree store in the default location (~/.codex/context).
    pub fn default_location() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| {
            ContextError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not find home directory",
            ))
        })?;

        let path = home.join(".codex").join("context");
        Ok(Self::new(path))
    }

    /// Get the base path of the store.
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Ensure the storage directory exists.
    fn ensure_dir(&self) -> Result<()> {
        if !self.base_path.exists() {
            fs::create_dir_all(&self.base_path).map_err(ContextError::Io)?;
            info!("Created storage directory: {}", self.base_path.display());
        }
        Ok(())
    }

    /// Get the path to the main tree file.
    fn tree_file_path(&self) -> PathBuf {
        self.base_path.join("tree.json")
    }

    /// Get the path to a backup file.
    fn backup_path(&self) -> PathBuf {
        self.base_path.join("tree.json.bak")
    }

    /// Save the context tree to disk.
    pub fn save(&self, tree: &ContextTree) -> Result<()> {
        self.ensure_dir()?;

        let tree_path = self.tree_file_path();
        let backup_path = self.backup_path();

        // Create backup of existing file
        if tree_path.exists() {
            fs::copy(&tree_path, &backup_path).map_err(ContextError::Io)?;
            debug!("Created backup at {}", backup_path.display());
        }

        // Serialize tree
        let data = TreeData::from_tree(tree);
        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| ContextError::InvalidFormat(format!("Failed to serialize tree: {}", e)))?;

        // Write to file
        fs::write(&tree_path, json).map_err(ContextError::Io)?;

        info!(
            "Saved context tree ({} nodes) to {}",
            tree.node_count(),
            tree_path.display()
        );

        Ok(())
    }

    /// Load the context tree from disk.
    pub fn load(&self) -> Result<ContextTree> {
        let tree_path = self.tree_file_path();

        if !tree_path.exists() {
            info!(
                "No existing tree found at {}, creating new tree",
                tree_path.display()
            );
            return Ok(ContextTree::new());
        }

        let json = fs::read_to_string(&tree_path).map_err(ContextError::Io)?;

        let data: TreeData = serde_json::from_str(&json).map_err(|e| {
            ContextError::InvalidFormat(format!("Failed to deserialize tree: {}", e))
        })?;

        // Check version
        if data.version != TreeData::CURRENT_VERSION {
            warn!(
                "Tree version mismatch: found {}, expected {}",
                data.version,
                TreeData::CURRENT_VERSION
            );
            // In the future, we could migrate old versions here
        }

        let tree = data.into_tree()?;

        info!(
            "Loaded context tree ({} nodes) from {}",
            tree.node_count(),
            tree_path.display()
        );

        Ok(tree)
    }

    /// Check if a tree exists at the storage location.
    pub fn exists(&self) -> bool {
        self.tree_file_path().exists()
    }

    /// Delete the stored tree.
    pub fn delete(&self) -> Result<()> {
        let tree_path = self.tree_file_path();
        let backup_path = self.backup_path();

        if tree_path.exists() {
            fs::remove_file(&tree_path).map_err(ContextError::Io)?;
        }

        if backup_path.exists() {
            fs::remove_file(&backup_path).map_err(ContextError::Io)?;
        }

        info!("Deleted stored tree at {}", self.base_path.display());
        Ok(())
    }

    /// Save a single node (for incremental updates).
    ///
    /// This saves the node to a separate file for faster incremental saves.
    pub fn save_node(&self, node: &ContextNode) -> Result<()> {
        self.ensure_dir()?;

        let nodes_dir = self.base_path.join("nodes");
        if !nodes_dir.exists() {
            fs::create_dir_all(&nodes_dir).map_err(ContextError::Io)?;
        }

        let node_path = nodes_dir.join(format!("{}.json", node.id));
        let json = serde_json::to_string_pretty(node)
            .map_err(|e| ContextError::InvalidFormat(format!("Failed to serialize node: {}", e)))?;

        fs::write(&node_path, json).map_err(ContextError::Io)?;

        debug!("Saved node {} to {}", node.id, node_path.display());
        Ok(())
    }

    /// Load a single node by ID.
    pub fn load_node(&self, id: &str) -> Result<Option<ContextNode>> {
        let node_path = self.base_path.join("nodes").join(format!("{}.json", id));

        if !node_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&node_path).map_err(ContextError::Io)?;
        let node: ContextNode = serde_json::from_str(&json).map_err(|e| {
            ContextError::InvalidFormat(format!("Failed to deserialize node: {}", e))
        })?;

        Ok(Some(node))
    }

    /// Export tree structure for visualization.
    pub fn export_structure(&self, tree: &ContextTree) -> TreeVisualization {
        let mut viz = TreeVisualization::new();

        fn build_viz(
            tree: &ContextTree,
            node: &ContextNode,
            viz: &mut TreeVisualization,
            depth: usize,
        ) {
            let indent = "  ".repeat(depth);
            let type_label = node.node_type.label();
            let line = format!(
                "{}{} [{}] - {}",
                indent,
                node.name,
                type_label,
                if node.summary.len() > 50 {
                    format!("{}...", &node.summary[..50])
                } else {
                    node.summary.clone()
                }
            );
            viz.lines.push(line);

            for child_id in &node.children {
                if let Some(child) = tree.get(child_id) {
                    build_viz(tree, child, viz, depth + 1);
                }
            }
        }

        build_viz(tree, tree.root(), &mut viz, 0);
        viz
    }
}

/// A visualization of the tree structure.
#[derive(Debug, Default)]
pub struct TreeVisualization {
    /// Lines of the visualization.
    pub lines: Vec<String>,
}

impl TreeVisualization {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }

    /// Convert to a string.
    pub fn to_string(&self) -> String {
        self.lines.join("\n")
    }
}

impl std::fmt::Display for TreeVisualization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let store = TreeStore::new(temp_dir.path());

        // Create a tree with some content
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        let mut project = ContextNode::project("test-project", PathBuf::from("/test"));
        project.summary = "A test project".to_string();
        tree.add_child(&domain_id, project).unwrap();

        // Save
        store.save(&tree).unwrap();
        assert!(store.exists());

        // Load
        let loaded_tree = store.load().unwrap();

        // Verify
        assert_eq!(loaded_tree.node_count(), tree.node_count());
        assert!(loaded_tree.get_domain("coding").is_some());
    }

    #[test]
    fn test_save_creates_backup() {
        let temp_dir = TempDir::new().unwrap();
        let store = TreeStore::new(temp_dir.path());

        let tree = ContextTree::new();

        // Save twice
        store.save(&tree).unwrap();
        store.save(&tree).unwrap();

        // Check backup exists
        assert!(store.backup_path().exists());
    }

    #[test]
    fn test_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store = TreeStore::new(temp_dir.path());

        // Should return new tree
        let tree = store.load().unwrap();
        assert_eq!(tree.node_count(), 1); // Just root
    }

    #[test]
    fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let store = TreeStore::new(temp_dir.path());

        let tree = ContextTree::new();
        store.save(&tree).unwrap();
        assert!(store.exists());

        store.delete().unwrap();
        assert!(!store.exists());
    }

    #[test]
    fn test_save_and_load_node() {
        let temp_dir = TempDir::new().unwrap();
        let store = TreeStore::new(temp_dir.path());

        let mut node = ContextNode::project("test", PathBuf::from("/test"));
        node.summary = "Test project".to_string();
        node.add_keyword("rust");

        // Save node
        store.save_node(&node).unwrap();

        // Load node
        let loaded = store.load_node(&node.id).unwrap();
        assert!(loaded.is_some());

        let loaded_node = loaded.unwrap();
        assert_eq!(loaded_node.name, "test");
        assert_eq!(loaded_node.summary, "Test project");
        assert!(loaded_node.keywords.contains(&"rust".to_string()));
    }

    #[test]
    fn test_load_nonexistent_node() {
        let temp_dir = TempDir::new().unwrap();
        let store = TreeStore::new(temp_dir.path());

        let loaded = store.load_node("nonexistent-id").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_export_structure() {
        let temp_dir = TempDir::new().unwrap();
        let store = TreeStore::new(temp_dir.path());

        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        let mut project = ContextNode::project("my-app", PathBuf::from("/app"));
        project.summary = "My application".to_string();
        tree.add_child(&domain_id, project).unwrap();

        let viz = store.export_structure(&tree);
        let output = viz.to_string();

        assert!(output.contains("User Knowledge"));
        assert!(output.contains("coding"));
        assert!(output.contains("my-app"));
    }

    #[test]
    fn test_tree_data_roundtrip() {
        let mut tree = ContextTree::new();
        tree.ensure_domain("test-domain");

        let data = TreeData::from_tree(&tree);
        let restored = data.into_tree().unwrap();

        assert_eq!(restored.node_count(), tree.node_count());
    }
}
