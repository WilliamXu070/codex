//! Hierarchical context tree structure.
//!
//! The `ContextTree` manages a hierarchy of context nodes representing
//! the user's knowledge organized by domains, categories, and projects.

use std::collections::HashMap;
use std::path::Path;

use tracing::{debug, info, warn};

use crate::error::{ContextError, Result};
use crate::node::{ContextNode, CrossLinkType, DomainDetection, NodeType, RelatedNode};

/// The main hierarchical context tree.
///
/// The tree organizes the user's knowledge from high-level domains
/// (coding, cooking, work) down to individual file references.
#[derive(Debug, Clone)]
pub struct ContextTree {
    /// All nodes in the tree, indexed by ID.
    nodes: HashMap<String, ContextNode>,

    /// ID of the root node.
    root_id: String,

    /// Index from domain names to domain node IDs.
    domain_index: HashMap<String, String>,

    /// Index from file paths to node IDs.
    path_index: HashMap<String, String>,
}

impl Default for ContextTree {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextTree {
    /// Create a new empty context tree with a root node.
    pub fn new() -> Self {
        let root = ContextNode::root();
        let root_id = root.id.clone();

        let mut nodes = HashMap::new();
        nodes.insert(root_id.clone(), root);

        Self {
            nodes,
            root_id,
            domain_index: HashMap::new(),
            path_index: HashMap::new(),
        }
    }

    /// Get the root node.
    pub fn root(&self) -> &ContextNode {
        match self.nodes.get(&self.root_id) {
            Some(node) => node,
            None => {
                // This should never happen - log the state for debugging
                warn!(
                    "Root node '{}' missing! Tree has {} nodes. Node IDs: {:?}",
                    self.root_id,
                    self.nodes.len(),
                    self.nodes.keys().collect::<Vec<_>>()
                );
                panic!("Root node '{}' must exist but was not found in tree with {} nodes",
                    self.root_id, self.nodes.len());
            }
        }
    }

    /// Get a mutable reference to the root node.
    pub fn root_mut(&mut self) -> &mut ContextNode {
        let root_id = self.root_id.clone();
        let node_count = self.nodes.len();
        match self.nodes.get_mut(&root_id) {
            Some(node) => node,
            None => {
                warn!(
                    "Root node '{}' missing! Tree has {} nodes.",
                    root_id,
                    node_count
                );
                panic!("Root node '{}' must exist but was not found", root_id);
            }
        }
    }

    /// Check if the tree has a valid root node.
    pub fn has_valid_root(&self) -> bool {
        self.nodes.contains_key(&self.root_id)
    }

    /// Ensure the tree has a valid root, creating one if needed.
    pub fn ensure_root(&mut self) {
        if !self.nodes.contains_key(&self.root_id) {
            warn!("Tree missing root node, creating new one");
            let root = ContextNode::root();
            self.root_id = root.id.clone();
            self.nodes.insert(root.id.clone(), root);
        }
    }

    /// Get a node by ID.
    pub fn get(&self, id: &str) -> Option<&ContextNode> {
        self.nodes.get(id)
    }

    /// Get a mutable node by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut ContextNode> {
        self.nodes.get_mut(id)
    }

    /// Get a node by file path.
    pub fn get_by_path(&self, path: &Path) -> Option<&ContextNode> {
        let path_str = path.to_string_lossy().to_string();
        self.path_index
            .get(&path_str)
            .and_then(|id| self.nodes.get(id))
    }

    /// Get all nodes in the tree.
    pub fn all_nodes(&self) -> impl Iterator<Item = &ContextNode> {
        self.nodes.values()
    }

    /// Get the total number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Insert a node into the tree.
    pub fn insert(&mut self, node: ContextNode) -> String {
        let id = node.id.clone();

        // Update path index if node has a path
        if let Some(ref path) = node.path {
            let path_str = path.to_string_lossy().to_string();
            self.path_index.insert(path_str, id.clone());
        }

        // Update domain index if node is a domain
        if node.node_type == NodeType::Domain {
            let name = node.name.to_lowercase();
            self.domain_index.insert(name, id.clone());
        }

        self.nodes.insert(id.clone(), node);
        id
    }

    /// Remove a node from the tree.
    ///
    /// This also removes the node from its parent's children list.
    pub fn remove(&mut self, id: &str) -> Option<ContextNode> {
        let node = self.nodes.remove(id)?;

        // Remove from parent's children
        if let Some(ref parent_id) = node.parent_id {
            if let Some(parent) = self.nodes.get_mut(parent_id) {
                parent.children.retain(|c| c != id);
            }
        }

        // Remove from path index
        if let Some(ref path) = node.path {
            let path_str = path.to_string_lossy().to_string();
            self.path_index.remove(&path_str);
        }

        // Remove from domain index
        if node.node_type == NodeType::Domain {
            let name = node.name.to_lowercase();
            self.domain_index.remove(&name);
        }

        Some(node)
    }

    /// Get or create a domain node.
    ///
    /// If the domain already exists, returns its ID.
    /// Otherwise, creates a new domain node and adds it to the root.
    pub fn ensure_domain(&mut self, domain: &str) -> String {
        let domain_lower = domain.to_lowercase();

        // Check if domain already exists
        if let Some(id) = self.domain_index.get(&domain_lower) {
            return id.clone();
        }

        // Create new domain node
        let mut domain_node = ContextNode::domain(domain);
        domain_node.parent_id = Some(self.root_id.clone());
        let domain_id = domain_node.id.clone();

        // Add to root's children
        if let Some(root) = self.nodes.get_mut(&self.root_id) {
            root.add_child(&domain_id);
        }

        // Insert into tree
        self.domain_index.insert(domain_lower, domain_id.clone());
        self.nodes.insert(domain_id.clone(), domain_node);

        info!("Created new domain: {}", domain);
        domain_id
    }

    /// Get a domain node by name.
    pub fn get_domain(&self, domain: &str) -> Option<&ContextNode> {
        let domain_lower = domain.to_lowercase();
        self.domain_index
            .get(&domain_lower)
            .and_then(|id| self.nodes.get(id))
    }

    /// List all domain names.
    pub fn list_domains(&self) -> Vec<&str> {
        self.domain_index.keys().map(|s| s.as_str()).collect()
    }

    /// Get the ancestry (path from root) for a node.
    pub fn get_ancestry(&self, node_id: &str) -> Vec<&ContextNode> {
        let mut ancestry = Vec::new();
        let mut current_id = Some(node_id.to_string());

        while let Some(id) = current_id {
            if let Some(node) = self.nodes.get(&id) {
                ancestry.push(node);
                current_id = node.parent_id.clone();
            } else {
                break;
            }
        }

        ancestry.reverse();
        ancestry
    }

    /// Get the domain for a node (first Domain node in ancestry).
    pub fn get_domain_for_node(&self, node_id: &str) -> Option<&ContextNode> {
        self.get_ancestry(node_id)
            .into_iter()
            .find(|n| n.node_type == NodeType::Domain)
    }

    /// Get all nodes at a specific depth.
    pub fn nodes_at_depth(&self, depth: u32) -> Vec<&ContextNode> {
        self.nodes.values().filter(|n| n.depth == depth).collect()
    }

    /// Get all descendants of a node.
    pub fn get_descendants(&self, node_id: &str) -> Vec<&ContextNode> {
        let mut descendants = Vec::new();
        let mut to_visit = vec![node_id.to_string()];

        while let Some(id) = to_visit.pop() {
            if let Some(node) = self.nodes.get(&id) {
                if id != node_id {
                    descendants.push(node);
                }
                to_visit.extend(node.children.clone());
            }
        }

        descendants
    }

    /// Get all leaf nodes (nodes with no children).
    pub fn get_leaves(&self) -> Vec<&ContextNode> {
        self.nodes.values().filter(|n| n.is_leaf()).collect()
    }

    /// Get the maximum depth in the tree.
    pub fn max_depth(&self) -> u32 {
        self.nodes.values().map(|n| n.depth).max().unwrap_or(0)
    }

    /// Add a node as a child of another node.
    pub fn add_child(&mut self, parent_id: &str, mut child: ContextNode) -> Result<String> {
        // Check parent exists
        let parent = self.nodes.get(parent_id).ok_or_else(|| {
            ContextError::InvalidFormat(format!("Parent node not found: {}", parent_id))
        })?;

        // Set child's parent and depth
        child.parent_id = Some(parent_id.to_string());
        child.depth = parent.depth + 1;

        let child_id = child.id.clone();

        // Insert child
        self.insert(child);

        // Add to parent's children
        if let Some(parent) = self.nodes.get_mut(parent_id) {
            parent.add_child(&child_id);
        }

        Ok(child_id)
    }

    /// Build cross-links between related nodes.
    ///
    /// This finds nodes that share common attributes (technologies, authors, etc.)
    /// and creates cross-links between them.
    pub fn build_cross_links(&mut self) {
        let node_ids: Vec<String> = self.nodes.keys().cloned().collect();

        // Build technology index
        let mut tech_index: HashMap<String, Vec<String>> = HashMap::new();
        for id in &node_ids {
            if let Some(node) = self.nodes.get(id) {
                for entity in &node.entities {
                    if entity.entity_type == crate::entity::EntityType::Technology {
                        tech_index
                            .entry(entity.normalized_name.clone())
                            .or_default()
                            .push(id.clone());
                    }
                }
            }
        }

        // Create cross-links for shared technologies
        for (_tech, ids) in tech_index {
            if ids.len() > 1 {
                for i in 0..ids.len() {
                    for j in (i + 1)..ids.len() {
                        let id_a = &ids[i];
                        let id_b = &ids[j];

                        // Don't link nodes in the same branch
                        if !self.are_in_same_branch(id_a, id_b) {
                            // Add bidirectional links
                            let link_a =
                                RelatedNode::new(id_b.clone(), CrossLinkType::SameTechnology, 0.7);
                            let link_b =
                                RelatedNode::new(id_a.clone(), CrossLinkType::SameTechnology, 0.7);

                            if let Some(node_a) = self.nodes.get_mut(id_a) {
                                if !node_a.related_nodes.iter().any(|r| r.node_id == *id_b) {
                                    node_a.add_related(link_a);
                                }
                            }
                            if let Some(node_b) = self.nodes.get_mut(id_b) {
                                if !node_b.related_nodes.iter().any(|r| r.node_id == *id_a) {
                                    node_b.add_related(link_b);
                                }
                            }
                        }
                    }
                }
            }
        }

        debug!("Built cross-links for tree");
    }

    /// Check if two nodes are in the same branch (one is an ancestor of the other).
    fn are_in_same_branch(&self, id_a: &str, id_b: &str) -> bool {
        let ancestry_a: Vec<String> = self
            .get_ancestry(id_a)
            .iter()
            .map(|n| n.id.clone())
            .collect();

        let ancestry_b: Vec<String> = self
            .get_ancestry(id_b)
            .iter()
            .map(|n| n.id.clone())
            .collect();

        ancestry_a.contains(&id_b.to_string()) || ancestry_b.contains(&id_a.to_string())
    }

    /// Apply a domain detection result to place a project in the tree.
    pub fn apply_domain_detection(
        &mut self,
        project_node: ContextNode,
        detection: &DomainDetection,
    ) -> Result<String> {
        // Ensure domain exists
        let domain_id = self.ensure_domain(&detection.domain);

        // If there's a subcategory, ensure it exists
        let parent_id = if let Some(ref subcategory) = detection.subcategory {
            self.ensure_category(&domain_id, subcategory)?
        } else {
            domain_id
        };

        // Add project as child of the parent
        self.add_child(&parent_id, project_node)
    }

    /// Ensure a category node exists under a domain.
    fn ensure_category(&mut self, domain_id: &str, category: &str) -> Result<String> {
        // Check if category already exists
        if let Some(domain) = self.nodes.get(domain_id) {
            for child_id in &domain.children {
                if let Some(child) = self.nodes.get(child_id) {
                    if child.node_type == NodeType::Category
                        && child.name.to_lowercase() == category.to_lowercase()
                    {
                        return Ok(child_id.clone());
                    }
                }
            }
        }

        // Create new category node
        let category_node = ContextNode::category(category);
        self.add_child(domain_id, category_node)
    }

    /// Get statistics about the tree.
    pub fn stats(&self) -> TreeStats {
        let mut stats = TreeStats::default();

        stats.total_nodes = self.nodes.len();
        stats.max_depth = self.max_depth();
        stats.domain_count = self.domain_index.len();

        for node in self.nodes.values() {
            match node.node_type {
                NodeType::Root => stats.root_count += 1,
                NodeType::Domain => stats.domains += 1,
                NodeType::Category => stats.categories += 1,
                NodeType::Project => stats.projects += 1,
                NodeType::Module => stats.modules += 1,
                NodeType::Document => stats.documents += 1,
                NodeType::FileReference => stats.files += 1,
            }

            stats.total_cross_links += node.related_nodes.len();
            stats.total_entities += node.entities.len();
        }

        stats
    }

    /// Search for nodes by keyword.
    ///
    /// Returns nodes that match ANY of the search terms (more lenient).
    /// Nodes are scored by how many terms they match and sorted by relevance.
    pub fn search(&self, query: &str) -> Vec<&ContextNode> {
        let query_lower = query.to_lowercase();

        // Filter out common stop words for better matching
        let stop_words: std::collections::HashSet<&str> = [
            "a", "an", "the", "is", "are", "was", "were", "be", "been", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "can",
            "to", "of", "in", "for", "on", "with", "at", "by", "from", "as",
            "and", "but", "if", "or", "what", "who", "whom", "which", "when", "where",
            "why", "how", "i", "my", "me", "we", "our", "you", "your", "that", "this",
        ].into_iter().collect();

        let terms: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|t| t.len() >= 2 && !stop_words.contains(t))
            .collect();

        if terms.is_empty() {
            // If no meaningful terms, return top-level content nodes
            return self.nodes
                .values()
                .filter(|n| matches!(n.node_type, NodeType::Project | NodeType::Document))
                .take(10)
                .collect();
        }

        // Score nodes by how many terms they match
        let mut scored: Vec<(&ContextNode, usize)> = self.nodes
            .values()
            .filter_map(|node| {
                let name_lower = node.name.to_lowercase();
                let summary_lower = node.summary.to_lowercase();

                let match_count = terms.iter().filter(|term| {
                    name_lower.contains(*term)
                        || summary_lower.contains(*term)
                        || node.keywords.iter().any(|k| k.to_lowercase().contains(*term))
                }).count();

                if match_count > 0 {
                    Some((node, match_count))
                } else {
                    None
                }
            })
            .collect();

        // Sort by match count (descending)
        scored.sort_by(|a, b| b.1.cmp(&a.1));

        scored.into_iter().map(|(node, _)| node).collect()
    }
}

/// Statistics about the context tree.
#[derive(Debug, Default, Clone)]
pub struct TreeStats {
    pub total_nodes: usize,
    pub max_depth: u32,
    pub domain_count: usize,
    pub root_count: usize,
    pub domains: usize,
    pub categories: usize,
    pub projects: usize,
    pub modules: usize,
    pub documents: usize,
    pub files: usize,
    pub total_cross_links: usize,
    pub total_entities: usize,
}

impl std::fmt::Display for TreeStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Context Tree Statistics:")?;
        writeln!(f, "  Total nodes: {}", self.total_nodes)?;
        writeln!(f, "  Max depth: {}", self.max_depth)?;
        writeln!(f, "  Domains: {}", self.domains)?;
        writeln!(f, "  Categories: {}", self.categories)?;
        writeln!(f, "  Projects: {}", self.projects)?;
        writeln!(f, "  Modules: {}", self.modules)?;
        writeln!(f, "  Documents: {}", self.documents)?;
        writeln!(f, "  Files: {}", self.files)?;
        writeln!(f, "  Cross-links: {}", self.total_cross_links)?;
        writeln!(f, "  Total entities: {}", self.total_entities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_new_tree() {
        let tree = ContextTree::new();
        assert_eq!(tree.node_count(), 1); // Just root
        assert_eq!(tree.root().node_type, NodeType::Root);
    }

    #[test]
    fn test_ensure_domain() {
        let mut tree = ContextTree::new();

        let id1 = tree.ensure_domain("coding");
        let id2 = tree.ensure_domain("Coding"); // Should return same ID

        assert_eq!(id1, id2);
        assert_eq!(tree.list_domains().len(), 1);

        let id3 = tree.ensure_domain("cooking");
        assert_ne!(id1, id3);
        assert_eq!(tree.list_domains().len(), 2);
    }

    #[test]
    fn test_add_child() {
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        let project = ContextNode::project("test-project", PathBuf::from("/test"));
        let project_id = tree.add_child(&domain_id, project).unwrap();

        let project_node = tree.get(&project_id).unwrap();
        assert_eq!(project_node.parent_id, Some(domain_id.clone()));
        assert_eq!(project_node.depth, 2); // root=0, domain=1, project=2

        let domain_node = tree.get(&domain_id).unwrap();
        assert!(domain_node.children.contains(&project_id));
    }

    #[test]
    fn test_get_ancestry() {
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        let project = ContextNode::project("test-project", PathBuf::from("/test"));
        let project_id = tree.add_child(&domain_id, project).unwrap();

        let file = ContextNode::file_reference("test.rs", PathBuf::from("/test/test.rs"));
        let file_id = tree.add_child(&project_id, file).unwrap();

        let ancestry = tree.get_ancestry(&file_id);
        assert_eq!(ancestry.len(), 4); // root, domain, project, file
        assert_eq!(ancestry[0].node_type, NodeType::Root);
        assert_eq!(ancestry[1].node_type, NodeType::Domain);
        assert_eq!(ancestry[2].node_type, NodeType::Project);
        assert_eq!(ancestry[3].node_type, NodeType::FileReference);
    }

    #[test]
    fn test_get_domain_for_node() {
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        let project = ContextNode::project("test-project", PathBuf::from("/test"));
        let project_id = tree.add_child(&domain_id, project).unwrap();

        let domain = tree.get_domain_for_node(&project_id).unwrap();
        assert_eq!(domain.name, "coding");
    }

    #[test]
    fn test_nodes_at_depth() {
        let mut tree = ContextTree::new();
        tree.ensure_domain("coding");
        tree.ensure_domain("cooking");

        let domains = tree.nodes_at_depth(1);
        assert_eq!(domains.len(), 2);
    }

    #[test]
    fn test_get_descendants() {
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        let project1 = ContextNode::project("project1", PathBuf::from("/p1"));
        let p1_id = tree.add_child(&domain_id, project1).unwrap();

        let project2 = ContextNode::project("project2", PathBuf::from("/p2"));
        let _p2_id = tree.add_child(&domain_id, project2).unwrap();

        let file = ContextNode::file_reference("test.rs", PathBuf::from("/p1/test.rs"));
        let _file_id = tree.add_child(&p1_id, file).unwrap();

        let descendants = tree.get_descendants(&domain_id);
        assert_eq!(descendants.len(), 3); // 2 projects + 1 file
    }

    #[test]
    fn test_get_leaves() {
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        let project = ContextNode::project("test-project", PathBuf::from("/test"));
        let project_id = tree.add_child(&domain_id, project).unwrap();

        let file1 = ContextNode::file_reference("a.rs", PathBuf::from("/test/a.rs"));
        tree.add_child(&project_id, file1).unwrap();

        let file2 = ContextNode::file_reference("b.rs", PathBuf::from("/test/b.rs"));
        tree.add_child(&project_id, file2).unwrap();

        let leaves = tree.get_leaves();
        assert_eq!(leaves.len(), 2);
    }

    #[test]
    fn test_remove_node() {
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        let project = ContextNode::project("test-project", PathBuf::from("/test"));
        let project_id = tree.add_child(&domain_id, project).unwrap();

        assert!(tree.get(&project_id).is_some());

        tree.remove(&project_id);

        assert!(tree.get(&project_id).is_none());
        let domain = tree.get(&domain_id).unwrap();
        assert!(!domain.children.contains(&project_id));
    }

    #[test]
    fn test_get_by_path() {
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        let path = PathBuf::from("/test/project");
        let project = ContextNode::project("test-project", path.clone());
        tree.add_child(&domain_id, project).unwrap();

        let found = tree.get_by_path(&path);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test-project");
    }

    #[test]
    fn test_apply_domain_detection() {
        let mut tree = ContextTree::new();

        let detection = DomainDetection::new("coding", 0.9).with_subcategory("rust-projects");

        let project = ContextNode::project("my-rust-app", PathBuf::from("/code/my-rust-app"));
        let project_id = tree.apply_domain_detection(project, &detection).unwrap();

        // Check project is at correct depth
        let project_node = tree.get(&project_id).unwrap();
        assert_eq!(project_node.depth, 3); // root=0, domain=1, category=2, project=3

        // Check ancestry
        let ancestry = tree.get_ancestry(&project_id);
        assert_eq!(ancestry[1].name.to_lowercase(), "coding");
        assert_eq!(ancestry[2].name.to_lowercase(), "rust-projects");
    }

    #[test]
    fn test_search() {
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        let mut project = ContextNode::project("rust-server", PathBuf::from("/code/server"));
        project.summary = "A web server written in Rust".to_string();
        project.add_keyword("rust");
        project.add_keyword("server");
        tree.add_child(&domain_id, project).unwrap();

        let mut project2 = ContextNode::project("python-client", PathBuf::from("/code/client"));
        project2.summary = "A client written in Python".to_string();
        project2.add_keyword("python");
        tree.add_child(&domain_id, project2).unwrap();

        let results = tree.search("rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust-server");

        let results = tree.search("written");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_stats() {
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        let project = ContextNode::project("test", PathBuf::from("/test"));
        let project_id = tree.add_child(&domain_id, project).unwrap();

        let file = ContextNode::file_reference("test.rs", PathBuf::from("/test/test.rs"));
        tree.add_child(&project_id, file).unwrap();

        let stats = tree.stats();
        assert_eq!(stats.total_nodes, 4); // root + domain + project + file
        assert_eq!(stats.domains, 1);
        assert_eq!(stats.projects, 1);
        assert_eq!(stats.files, 1);
        assert_eq!(stats.max_depth, 3);
    }
}
