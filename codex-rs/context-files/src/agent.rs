//! Agentic orchestration for context extraction and management.
//!
//! The `ContextAgent` is the main entry point for processing folders,
//! querying context, and managing the user's knowledge tree.

use std::path::{Path, PathBuf};
use std::time::Instant;

use tracing::{debug, info, warn};
use walkdir::WalkDir;

use crate::chunker::{Chunk, SemanticChunker};
use crate::entity::EntityExtractor;
use crate::error::{ContextError, Result};
use crate::llm::{AnalysisContext, LlmAnalyzer, LlmConfig};
use crate::node::{ContextNode, DomainDetection, NodeType};
use crate::tree::ContextTree;

/// Configuration for the context agent.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Maximum depth for the tree (None = unlimited).
    pub max_depth: Option<u32>,

    /// Whether to automatically create cross-links.
    pub auto_cross_link: bool,

    /// Minimum confidence for including entities.
    pub min_confidence: f32,

    /// Maximum files to process per folder.
    pub max_files_per_folder: usize,

    /// File extensions to process.
    pub extensions: Vec<String>,

    /// Whether to process recursively.
    pub recursive: bool,

    /// Whether to create file reference nodes.
    pub create_file_refs: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_depth: None,
            auto_cross_link: true,
            min_confidence: 0.3,
            max_files_per_folder: 1000,
            extensions: vec![
                "md".to_string(),
                "txt".to_string(),
                "rs".to_string(),
                "py".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "go".to_string(),
                "java".to_string(),
                "toml".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
                "json".to_string(),
            ],
            recursive: true,
            create_file_refs: true,
        }
    }
}

/// Result of processing a folder.
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    /// ID of the root node created for this folder.
    pub root_node_id: String,

    /// Number of nodes created.
    pub nodes_created: usize,

    /// Detected domain for this folder.
    pub domain: String,

    /// Number of cross-links created.
    pub cross_links_created: usize,

    /// Processing time in milliseconds.
    pub processing_time_ms: u64,

    /// Files processed.
    pub files_processed: usize,

    /// Total entities extracted.
    pub entities_extracted: usize,

    /// Errors encountered during processing.
    pub errors: Vec<String>,
}

/// Result of querying the context tree.
#[derive(Debug, Clone)]
pub struct AgentQueryResult {
    /// Matching nodes.
    pub nodes: Vec<ContextNode>,

    /// Query processing time in milliseconds.
    pub processing_time_ms: u64,

    /// The query that was executed.
    pub query: String,

    /// Whether results were truncated.
    pub truncated: bool,
}

/// The main context agent for building and querying the knowledge tree.
///
/// The agent orchestrates:
/// - Folder processing and file analysis
/// - Domain detection and tree organization
/// - Cross-linking between related content
/// - Query execution and context retrieval
pub struct ContextAgent {
    /// The hierarchical context tree.
    tree: ContextTree,

    /// LLM analyzer for AI-powered analysis.
    analyzer: LlmAnalyzer,

    /// Agent configuration.
    config: AgentConfig,

    /// Chunker for document processing.
    chunker: SemanticChunker,

    /// Entity extractor.
    entity_extractor: EntityExtractor,
}

impl Default for ContextAgent {
    fn default() -> Self {
        Self::new(AgentConfig::default(), LlmConfig::default())
    }
}

impl ContextAgent {
    /// Create a new context agent.
    pub fn new(config: AgentConfig, llm_config: LlmConfig) -> Self {
        Self {
            tree: ContextTree::new(),
            analyzer: LlmAnalyzer::new(llm_config),
            config,
            chunker: SemanticChunker::new(),
            entity_extractor: EntityExtractor::new(),
        }
    }

    /// Create an agent with heuristic-only mode (no LLM).
    pub fn heuristic_only() -> Self {
        Self {
            tree: ContextTree::new(),
            analyzer: LlmAnalyzer::heuristic_only(),
            config: AgentConfig::default(),
            chunker: SemanticChunker::new(),
            entity_extractor: EntityExtractor::new(),
        }
    }

    /// Create an agent with an existing tree.
    pub fn with_tree(tree: ContextTree, config: AgentConfig, llm_config: LlmConfig) -> Self {
        Self {
            tree,
            analyzer: LlmAnalyzer::new(llm_config),
            config,
            chunker: SemanticChunker::new(),
            entity_extractor: EntityExtractor::new(),
        }
    }

    /// Get a reference to the context tree.
    pub fn tree(&self) -> &ContextTree {
        &self.tree
    }

    /// Get a mutable reference to the context tree.
    pub fn tree_mut(&mut self) -> &mut ContextTree {
        &mut self.tree
    }

    /// Get the user's world model (root node).
    pub fn user_profile(&self) -> &ContextNode {
        self.tree.root()
    }

    /// Process a folder and integrate it into the context tree.
    pub async fn process_folder(&mut self, path: &Path) -> Result<ProcessingResult> {
        let start = Instant::now();
        let mut result = ProcessingResult {
            root_node_id: String::new(),
            nodes_created: 0,
            domain: String::new(),
            cross_links_created: 0,
            processing_time_ms: 0,
            files_processed: 0,
            entities_extracted: 0,
            errors: Vec::new(),
        };

        // Verify path exists
        if !path.exists() {
            return Err(ContextError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path does not exist: {}", path.display()),
            )));
        }

        info!("Processing folder: {}", path.display());

        // Collect files to process
        let files = self.collect_files(path)?;
        result.files_processed = files.len();

        // Analyze files to build folder summary
        let (folder_summary, file_extensions) = self.analyze_folder_contents(&files).await;

        // Detect domain
        let existing_domains = self
            .tree
            .list_domains()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let detection = self
            .analyzer
            .detect_domain(&folder_summary, &file_extensions, &existing_domains)
            .await?;

        result.domain = detection.domain.clone();
        info!(
            "Detected domain: {} (confidence: {})",
            detection.domain, detection.confidence
        );

        // Create project node
        let folder_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut project_node = ContextNode::project(&folder_name, path.to_path_buf());
        project_node.summary = folder_summary;
        project_node.confidence = detection.confidence;

        // Apply domain detection to place in tree
        let project_id = self.tree.apply_domain_detection(project_node, &detection)?;
        result.root_node_id = project_id.clone();
        result.nodes_created += 1;

        // Process each file
        for file_path in &files {
            match self.process_file(file_path, &project_id).await {
                Ok((nodes, entities)) => {
                    result.nodes_created += nodes;
                    result.entities_extracted += entities;
                }
                Err(e) => {
                    result
                        .errors
                        .push(format!("{}: {}", file_path.display(), e));
                    warn!("Error processing file {}: {}", file_path.display(), e);
                }
            }
        }

        // Build cross-links if enabled
        if self.config.auto_cross_link {
            let before = self.count_cross_links();
            self.tree.build_cross_links();
            result.cross_links_created = self.count_cross_links() - before;
        }

        // Update root summary
        self.update_root_summary().await;

        result.processing_time_ms = start.elapsed().as_millis() as u64;

        info!(
            "Processed folder {} in {}ms: {} nodes, {} files, {} entities",
            path.display(),
            result.processing_time_ms,
            result.nodes_created,
            result.files_processed,
            result.entities_extracted
        );

        Ok(result)
    }

    /// Collect files to process from a folder.
    fn collect_files(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        let walker = if self.config.recursive {
            WalkDir::new(path)
        } else {
            WalkDir::new(path).max_depth(1)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            // Check extension
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if self.config.extensions.contains(&ext.to_lowercase()) {
                    files.push(path.to_path_buf());
                }
            }

            // Respect max files limit
            if files.len() >= self.config.max_files_per_folder {
                warn!(
                    "Reached max files limit ({}), stopping collection",
                    self.config.max_files_per_folder
                );
                break;
            }
        }

        Ok(files)
    }

    /// Analyze folder contents to build a summary.
    async fn analyze_folder_contents(&self, files: &[PathBuf]) -> (String, Vec<String>) {
        let mut summaries = Vec::new();
        let mut extensions = Vec::new();

        for file in files.iter().take(10) {
            // Sample first 10 files
            if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
                if !extensions.contains(&ext.to_string()) {
                    extensions.push(ext.to_string());
                }
            }

            // Read and summarize file
            if let Ok(content) = std::fs::read_to_string(file) {
                let preview = content.lines().take(5).collect::<Vec<_>>().join(" ");
                if !preview.is_empty() {
                    summaries.push(preview);
                }
            }
        }

        let folder_summary = summaries.join(". ");
        (folder_summary, extensions)
    }

    /// Process a single file and add nodes to the tree.
    async fn process_file(&mut self, file_path: &Path, parent_id: &str) -> Result<(usize, usize)> {
        let content = std::fs::read_to_string(file_path).map_err(ContextError::Io)?;

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Analyze document
        let context = AnalysisContext {
            file_path: Some(file_path.to_string_lossy().to_string()),
            file_extension: file_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_string()),
            parent_folder: file_path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(|s| s.to_string()),
            ..Default::default()
        };

        let analysis = self.analyzer.analyze_document(&content, &context).await?;

        let mut nodes_created = 0;
        let entities_count = analysis.entities.len();

        // Create document node
        let mut doc_node = ContextNode::document(&file_name, file_path.to_path_buf());
        doc_node.summary = analysis.summary;
        doc_node.entities = analysis.entities;
        doc_node.confidence = analysis.confidence;

        for topic in &analysis.topics {
            doc_node.add_keyword(topic);
        }

        let doc_id = self.tree.add_child(parent_id, doc_node)?;
        nodes_created += 1;

        // Create file reference node if enabled
        if self.config.create_file_refs {
            let file_ref = ContextNode::file_reference(&file_name, file_path.to_path_buf());
            self.tree.add_child(&doc_id, file_ref)?;
            nodes_created += 1;
        }

        debug!(
            "Processed file {}: {} entities, {} topics",
            file_name,
            entities_count,
            analysis.topics.len()
        );

        Ok((nodes_created, entities_count))
    }

    /// Count total cross-links in the tree.
    fn count_cross_links(&self) -> usize {
        self.tree.all_nodes().map(|n| n.related_nodes.len()).sum()
    }

    /// Update the root node summary based on domains.
    async fn update_root_summary(&mut self) {
        let domains = self.tree.list_domains();
        let domain_count = domains.len();

        if domain_count == 0 {
            return;
        }

        // Collect domain summaries
        let mut domain_info = Vec::new();
        for domain in domains {
            if let Some(node) = self.tree.get_domain(domain) {
                let project_count = node.children.len();
                domain_info.push(format!("{} ({} items)", domain, project_count));
            }
        }

        let summary = format!(
            "User knowledge across {} domain{}: {}",
            domain_count,
            if domain_count > 1 { "s" } else { "" },
            domain_info.join(", ")
        );

        if let Some(root) = self.tree.get_mut(&self.tree.root().id.clone()) {
            root.summary = summary;
        }
    }

    /// Query the context tree.
    pub fn query(&self, query: &str) -> AgentQueryResult {
        let start = Instant::now();

        let nodes = self.tree.search(query);
        let result_nodes: Vec<ContextNode> = nodes.into_iter().cloned().collect();

        let truncated = result_nodes.len() > 20;
        let nodes = if truncated {
            result_nodes.into_iter().take(20).collect()
        } else {
            result_nodes
        };

        AgentQueryResult {
            nodes,
            processing_time_ms: start.elapsed().as_millis() as u64,
            query: query.to_string(),
            truncated,
        }
    }

    /// Get context for a specific domain.
    pub fn get_domain_context(&self, domain: &str) -> Option<Vec<&ContextNode>> {
        let domain_node = self.tree.get_domain(domain)?;
        let descendants = self.tree.get_descendants(&domain_node.id);
        Some(std::iter::once(domain_node).chain(descendants).collect())
    }

    /// Get the ancestry path for a file.
    pub fn get_file_context(&self, file_path: &Path) -> Option<Vec<&ContextNode>> {
        let node = self.tree.get_by_path(file_path)?;
        Some(self.tree.get_ancestry(&node.id))
    }

    /// List all domains in the tree.
    pub fn list_domains(&self) -> Vec<&str> {
        self.tree.list_domains()
    }

    /// Get tree statistics.
    pub fn stats(&self) -> crate::tree::TreeStats {
        self.tree.stats()
    }
}

/// Builder for creating a context agent with custom configuration.
pub struct AgentBuilder {
    config: AgentConfig,
    llm_config: LlmConfig,
    tree: Option<ContextTree>,
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentBuilder {
    /// Create a new agent builder.
    pub fn new() -> Self {
        Self {
            config: AgentConfig::default(),
            llm_config: LlmConfig::default(),
            tree: None,
        }
    }

    /// Set maximum tree depth.
    pub fn max_depth(mut self, depth: u32) -> Self {
        self.config.max_depth = Some(depth);
        self
    }

    /// Enable or disable auto cross-linking.
    pub fn auto_cross_link(mut self, enabled: bool) -> Self {
        self.config.auto_cross_link = enabled;
        self
    }

    /// Set minimum confidence threshold.
    pub fn min_confidence(mut self, confidence: f32) -> Self {
        self.config.min_confidence = confidence;
        self
    }

    /// Set file extensions to process.
    pub fn extensions(mut self, extensions: Vec<String>) -> Self {
        self.config.extensions = extensions;
        self
    }

    /// Set recursive processing.
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.config.recursive = recursive;
        self
    }

    /// Set heuristic-only mode.
    pub fn heuristic_only(mut self) -> Self {
        self.llm_config.fallback_to_heuristic = true;
        self
    }

    /// Set known domains.
    pub fn known_domains(mut self, domains: Vec<String>) -> Self {
        self.llm_config.known_domains = domains;
        self
    }

    /// Use an existing tree.
    pub fn with_tree(mut self, tree: ContextTree) -> Self {
        self.tree = Some(tree);
        self
    }

    /// Build the agent.
    pub fn build(self) -> ContextAgent {
        if let Some(tree) = self.tree {
            ContextAgent::with_tree(tree, self.config, self.llm_config)
        } else {
            ContextAgent::new(self.config, self.llm_config)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_project(dir: &Path) {
        // Create a simple project structure
        fs::write(
            dir.join("README.md"),
            "# Test Project\n\nA test project using Rust and tokio.\n",
        )
        .unwrap();

        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            dir.join("src/main.rs"),
            "fn main() {\n    println!(\"Hello, world!\");\n}\n",
        )
        .unwrap();

        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
    }

    fn create_cooking_project(dir: &Path) {
        fs::write(
            dir.join("chocolate-cake.md"),
            "# Chocolate Cake Recipe\n\n## Ingredients\n\n- 2 cups flour\n- 1 cup sugar\n- 1/2 cup cocoa\n\n## Instructions\n\n1. Preheat oven to 350F\n2. Mix ingredients\n3. Bake for 30 minutes\n",
        )
        .unwrap();

        fs::write(
            dir.join("pasta.md"),
            "# Pasta Carbonara\n\n## Ingredients\n\n- 1 lb pasta\n- 4 eggs\n- Parmesan cheese\n\n## Instructions\n\n1. Cook pasta\n2. Mix eggs and cheese\n3. Combine\n",
        )
        .unwrap();
    }

    #[tokio::test]
    async fn test_process_coding_folder() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path());

        let mut agent = ContextAgent::heuristic_only();
        let result = agent.process_folder(temp_dir.path()).await.unwrap();

        assert_eq!(result.domain, "coding");
        assert!(result.nodes_created >= 3); // project + files
        assert!(result.files_processed >= 2);
    }

    #[tokio::test]
    async fn test_process_cooking_folder() {
        let temp_dir = TempDir::new().unwrap();
        create_cooking_project(temp_dir.path());

        let mut agent = ContextAgent::heuristic_only();
        let result = agent.process_folder(temp_dir.path()).await.unwrap();

        assert_eq!(result.domain, "cooking");
        assert!(result.nodes_created >= 2);
    }

    #[tokio::test]
    async fn test_multi_domain_processing() {
        let temp_dir = TempDir::new().unwrap();

        // Create coding project
        let coding_dir = temp_dir.path().join("my-rust-app");
        fs::create_dir_all(&coding_dir).unwrap();
        create_test_project(&coding_dir);

        // Create cooking folder
        let cooking_dir = temp_dir.path().join("recipes");
        fs::create_dir_all(&cooking_dir).unwrap();
        create_cooking_project(&cooking_dir);

        let mut agent = ContextAgent::heuristic_only();

        // Process coding folder
        let coding_result = agent.process_folder(&coding_dir).await.unwrap();
        assert_eq!(coding_result.domain, "coding");

        // Process cooking folder
        let cooking_result = agent.process_folder(&cooking_dir).await.unwrap();
        assert_eq!(cooking_result.domain, "cooking");

        // Verify tree structure
        let domains = agent.list_domains();
        assert!(domains.contains(&"coding"));
        assert!(domains.contains(&"cooking"));

        // Verify stats
        let stats = agent.stats();
        assert_eq!(stats.domains, 2);
    }

    #[tokio::test]
    async fn test_query() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path());

        let mut agent = ContextAgent::heuristic_only();
        agent.process_folder(temp_dir.path()).await.unwrap();

        let result = agent.query("rust");
        assert!(!result.nodes.is_empty());
    }

    #[tokio::test]
    async fn test_get_domain_context() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path());

        let mut agent = ContextAgent::heuristic_only();
        agent.process_folder(temp_dir.path()).await.unwrap();

        let context = agent.get_domain_context("coding");
        assert!(context.is_some());
        let nodes = context.unwrap();
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_agent_builder() {
        let agent = AgentBuilder::new()
            .max_depth(5)
            .auto_cross_link(false)
            .min_confidence(0.5)
            .heuristic_only()
            .build();

        assert_eq!(agent.config.max_depth, Some(5));
        assert!(!agent.config.auto_cross_link);
        assert_eq!(agent.config.min_confidence, 0.5);
    }

    #[tokio::test]
    async fn test_user_profile() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path());

        let mut agent = ContextAgent::heuristic_only();
        agent.process_folder(temp_dir.path()).await.unwrap();

        let profile = agent.user_profile();
        assert_eq!(profile.node_type, NodeType::Root);
        assert!(!profile.summary.is_empty());
    }

    #[tokio::test]
    async fn test_nonexistent_path() {
        let mut agent = ContextAgent::heuristic_only();
        let result = agent.process_folder(Path::new("/nonexistent/path")).await;
        assert!(result.is_err());
    }
}
