//! Core types for the hierarchical context tree.
//!
//! This module defines the `ContextNode` and related types that form
//! the building blocks of the user's knowledge hierarchy.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::entity::Entity;

/// A node in the hierarchical context tree.
///
/// The tree represents the user's knowledge organized from high-level
/// domains (coding, cooking, work) down to individual file references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextNode {
    /// Unique identifier for this node.
    pub id: String,

    /// The type of node (Domain, Project, Document, etc.).
    pub node_type: NodeType,

    /// Human-readable name for this node.
    pub name: String,

    /// File system path (for folder/file nodes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,

    /// Depth in the tree (0 = root).
    pub depth: u32,

    /// AI-generated or heuristic summary of this node's content.
    pub summary: String,

    /// Entities extracted from this node's content.
    #[serde(default)]
    pub entities: Vec<Entity>,

    /// Keywords for search and retrieval.
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Parent node ID (None for root).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    /// Child node IDs.
    #[serde(default)]
    pub children: Vec<String>,

    /// Cross-links to related nodes in other branches.
    #[serde(default)]
    pub related_nodes: Vec<RelatedNode>,

    /// Confidence score for this node's categorization (0.0 to 1.0).
    pub confidence: f32,

    /// When this node was last updated.
    pub last_updated: DateTime<Utc>,

    /// Number of times this node has been accessed.
    #[serde(default)]
    pub access_count: u32,
}

impl ContextNode {
    /// Create a new context node with the given type and name.
    pub fn new(node_type: NodeType, name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            node_type,
            name: name.into(),
            path: None,
            depth: 0,
            summary: String::new(),
            entities: Vec::new(),
            keywords: Vec::new(),
            parent_id: None,
            children: Vec::new(),
            related_nodes: Vec::new(),
            confidence: 1.0,
            last_updated: Utc::now(),
            access_count: 0,
        }
    }

    /// Create a root node for the user's world model.
    pub fn root() -> Self {
        Self::new(NodeType::Root, "User Knowledge")
    }

    /// Create a domain node (e.g., "coding", "cooking").
    pub fn domain(name: impl Into<String>) -> Self {
        let mut node = Self::new(NodeType::Domain, name);
        node.depth = 1;
        node
    }

    /// Create a category node (e.g., "rust-projects").
    pub fn category(name: impl Into<String>) -> Self {
        let mut node = Self::new(NodeType::Category, name);
        node.depth = 2;
        node
    }

    /// Create a project node.
    pub fn project(name: impl Into<String>, path: PathBuf) -> Self {
        let mut node = Self::new(NodeType::Project, name);
        node.path = Some(path);
        node.depth = 3;
        node
    }

    /// Create a module node (sub-section of a project).
    pub fn module(name: impl Into<String>, path: PathBuf) -> Self {
        let mut node = Self::new(NodeType::Module, name);
        node.path = Some(path);
        node
    }

    /// Create a document node (summary of a single document).
    pub fn document(name: impl Into<String>, path: PathBuf) -> Self {
        let mut node = Self::new(NodeType::Document, name);
        node.path = Some(path);
        node
    }

    /// Create a file reference node (leaf node pointing to actual file).
    pub fn file_reference(name: impl Into<String>, path: PathBuf) -> Self {
        let mut node = Self::new(NodeType::FileReference, name);
        node.path = Some(path);
        node
    }

    /// Set the summary for this node.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    /// Set the depth for this node.
    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    /// Set the parent ID for this node.
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Add a child ID to this node.
    pub fn add_child(&mut self, child_id: impl Into<String>) {
        self.children.push(child_id.into());
    }

    /// Add a related node link.
    pub fn add_related(&mut self, related: RelatedNode) {
        self.related_nodes.push(related);
    }

    /// Add an entity to this node.
    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    /// Add a keyword to this node.
    pub fn add_keyword(&mut self, keyword: impl Into<String>) {
        let kw = keyword.into().to_lowercase();
        if !self.keywords.contains(&kw) {
            self.keywords.push(kw);
        }
    }

    /// Record an access to this node.
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_updated = Utc::now();
    }

    /// Check if this node is a leaf node (no children).
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Check if this node has a file system path.
    pub fn has_path(&self) -> bool {
        self.path.is_some()
    }

    /// Get the file extension if this node has a path.
    pub fn file_extension(&self) -> Option<&str> {
        self.path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|ext| ext.to_str())
    }
}

/// The type of a context node in the hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// Root node representing the user's entire knowledge model.
    Root,

    /// High-level domain (coding, cooking, work, etc.).
    Domain,

    /// Sub-category within a domain (rust-projects, recipes).
    Category,

    /// A project or major folder.
    Project,

    /// A module or sub-section of a project.
    Module,

    /// Summary of a single document.
    Document,

    /// Leaf node pointing to an actual file.
    FileReference,
}

impl NodeType {
    /// Get the typical depth for this node type.
    pub fn typical_depth(&self) -> u32 {
        match self {
            NodeType::Root => 0,
            NodeType::Domain => 1,
            NodeType::Category => 2,
            NodeType::Project => 3,
            NodeType::Module => 4,
            NodeType::Document => 5,
            NodeType::FileReference => 6,
        }
    }

    /// Check if this node type represents a container (has children).
    pub fn is_container(&self) -> bool {
        matches!(
            self,
            NodeType::Root
                | NodeType::Domain
                | NodeType::Category
                | NodeType::Project
                | NodeType::Module
        )
    }

    /// Get a human-readable label for this node type.
    pub fn label(&self) -> &'static str {
        match self {
            NodeType::Root => "Root",
            NodeType::Domain => "Domain",
            NodeType::Category => "Category",
            NodeType::Project => "Project",
            NodeType::Module => "Module",
            NodeType::Document => "Document",
            NodeType::FileReference => "File",
        }
    }
}

/// A cross-link to a related node in another branch of the tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedNode {
    /// ID of the related node.
    pub node_id: String,

    /// Type of relationship.
    pub relationship: CrossLinkType,

    /// Strength of the relationship (0.0 to 1.0).
    pub strength: f32,

    /// Optional explanation of why these nodes are related.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl RelatedNode {
    /// Create a new related node link.
    pub fn new(node_id: impl Into<String>, relationship: CrossLinkType, strength: f32) -> Self {
        Self {
            node_id: node_id.into(),
            relationship,
            strength,
            reason: None,
        }
    }

    /// Add a reason for this relationship.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Types of cross-links between nodes in different branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossLinkType {
    /// Both nodes use the same technology (e.g., Rust, React).
    SameTechnology,

    /// Same person is involved in both nodes.
    SameAuthor,

    /// One node references the other.
    References,

    /// AI detected topical similarity.
    SimilarTopic,

    /// User manually created this link.
    UserDefined,

    /// Both nodes are part of the same project ecosystem.
    SameEcosystem,

    /// Nodes share common dependencies.
    SharedDependency,
}

impl CrossLinkType {
    /// Get a human-readable label for this link type.
    pub fn label(&self) -> &'static str {
        match self {
            CrossLinkType::SameTechnology => "Same Technology",
            CrossLinkType::SameAuthor => "Same Author",
            CrossLinkType::References => "References",
            CrossLinkType::SimilarTopic => "Similar Topic",
            CrossLinkType::UserDefined => "User Defined",
            CrossLinkType::SameEcosystem => "Same Ecosystem",
            CrossLinkType::SharedDependency => "Shared Dependency",
        }
    }
}

/// Analysis result for a document processed by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentAnalysis {
    /// Generated summary of the document.
    pub summary: String,

    /// Extracted entities from the document.
    pub entities: Vec<Entity>,

    /// Detected topics/themes.
    pub topics: Vec<String>,

    /// Suggested domain if not already known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_domain: Option<String>,

    /// Confidence in the analysis (0.0 to 1.0).
    pub confidence: f32,
}

impl Default for DocumentAnalysis {
    fn default() -> Self {
        Self {
            summary: String::new(),
            entities: Vec::new(),
            topics: Vec::new(),
            suggested_domain: None,
            confidence: 0.0,
        }
    }
}

/// Result of domain detection for a folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainDetection {
    /// The detected domain name.
    pub domain: String,

    /// Optional sub-category within the domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,

    /// Whether this is a new domain not seen before.
    pub is_new_domain: bool,

    /// Confidence in the detection (0.0 to 1.0).
    pub confidence: f32,
}

impl DomainDetection {
    /// Create a new domain detection result.
    pub fn new(domain: impl Into<String>, confidence: f32) -> Self {
        Self {
            domain: domain.into(),
            subcategory: None,
            is_new_domain: false,
            confidence,
        }
    }

    /// Mark this as a new domain.
    pub fn as_new(mut self) -> Self {
        self.is_new_domain = true;
        self
    }

    /// Set the subcategory.
    pub fn with_subcategory(mut self, subcategory: impl Into<String>) -> Self {
        self.subcategory = Some(subcategory.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = ContextNode::new(NodeType::Project, "test-project");
        assert_eq!(node.name, "test-project");
        assert_eq!(node.node_type, NodeType::Project);
        assert!(!node.id.is_empty());
    }

    #[test]
    fn test_root_node() {
        let root = ContextNode::root();
        assert_eq!(root.node_type, NodeType::Root);
        assert_eq!(root.depth, 0);
    }

    #[test]
    fn test_domain_node() {
        let domain = ContextNode::domain("coding");
        assert_eq!(domain.node_type, NodeType::Domain);
        assert_eq!(domain.depth, 1);
        assert_eq!(domain.name, "coding");
    }

    #[test]
    fn test_project_node_with_path() {
        let path = PathBuf::from("/home/user/projects/test");
        let project = ContextNode::project("test", path.clone());
        assert_eq!(project.node_type, NodeType::Project);
        assert_eq!(project.path, Some(path));
    }

    #[test]
    fn test_node_builder_pattern() {
        let node = ContextNode::domain("cooking")
            .with_summary("Recipes and meal planning")
            .with_depth(1)
            .with_parent("root-id");

        assert_eq!(node.summary, "Recipes and meal planning");
        assert_eq!(node.depth, 1);
        assert_eq!(node.parent_id, Some("root-id".to_string()));
    }

    #[test]
    fn test_add_child() {
        let mut parent = ContextNode::domain("coding");
        parent.add_child("child-1");
        parent.add_child("child-2");

        assert_eq!(parent.children.len(), 2);
        assert!(parent.children.contains(&"child-1".to_string()));
    }

    #[test]
    fn test_add_keyword_dedup() {
        let mut node = ContextNode::domain("coding");
        node.add_keyword("Rust");
        node.add_keyword("rust"); // Should be deduped
        node.add_keyword("Python");

        assert_eq!(node.keywords.len(), 2);
        assert!(node.keywords.contains(&"rust".to_string()));
        assert!(node.keywords.contains(&"python".to_string()));
    }

    #[test]
    fn test_related_node() {
        let related = RelatedNode::new("other-id", CrossLinkType::SameTechnology, 0.8)
            .with_reason("Both use Rust");

        assert_eq!(related.node_id, "other-id");
        assert_eq!(related.relationship, CrossLinkType::SameTechnology);
        assert_eq!(related.strength, 0.8);
        assert_eq!(related.reason, Some("Both use Rust".to_string()));
    }

    #[test]
    fn test_node_type_properties() {
        assert!(NodeType::Domain.is_container());
        assert!(!NodeType::FileReference.is_container());

        assert_eq!(NodeType::Root.typical_depth(), 0);
        assert_eq!(NodeType::Domain.typical_depth(), 1);
        assert_eq!(NodeType::FileReference.typical_depth(), 6);
    }

    #[test]
    fn test_is_leaf() {
        let leaf = ContextNode::file_reference("test.rs", PathBuf::from("/test.rs"));
        assert!(leaf.is_leaf());

        let mut parent = ContextNode::domain("coding");
        parent.add_child("child");
        assert!(!parent.is_leaf());
    }

    #[test]
    fn test_file_extension() {
        let node = ContextNode::file_reference("test.rs", PathBuf::from("/path/test.rs"));
        assert_eq!(node.file_extension(), Some("rs"));

        let domain = ContextNode::domain("coding");
        assert_eq!(domain.file_extension(), None);
    }

    #[test]
    fn test_domain_detection() {
        let detection = DomainDetection::new("coding", 0.9)
            .as_new()
            .with_subcategory("rust-projects");

        assert_eq!(detection.domain, "coding");
        assert!(detection.is_new_domain);
        assert_eq!(detection.subcategory, Some("rust-projects".to_string()));
        assert_eq!(detection.confidence, 0.9);
    }
}
