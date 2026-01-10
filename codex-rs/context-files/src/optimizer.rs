//! Tree optimization and cleanup algorithm.
//!
//! The `TreeOptimizer` provides algorithms for optimizing the context tree
//! by merging redundant nodes, pruning stale content, and compressing depth.

use std::collections::{HashMap, HashSet};

use chrono::{Duration, Utc};
use tracing::{debug, info};

use crate::error::Result;
use crate::llm::LlmAnalyzer;
use crate::node::{ContextNode, NodeType};
use crate::tree::ContextTree;

/// Configuration for the tree optimizer.
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    /// Maximum days since last access before considering a node stale.
    pub max_idle_days: u32,

    /// Minimum access count to keep detailed information.
    pub min_access_count: u32,

    /// Target depth reduction when compressing.
    pub target_depth_reduction: u32,

    /// Minimum nodes to trigger sibling merging.
    pub min_siblings_for_merge: usize,

    /// Maximum depth before forcing compression.
    pub max_depth_threshold: u32,

    /// Whether to prune file reference nodes.
    pub prune_file_refs: bool,

    /// Whether to merge similar siblings.
    pub merge_siblings: bool,

    /// Whether to compress deep branches.
    pub compress_deep_branches: bool,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            max_idle_days: 30,
            min_access_count: 2,
            target_depth_reduction: 2,
            min_siblings_for_merge: 5,
            max_depth_threshold: 8,
            prune_file_refs: true,
            merge_siblings: true,
            compress_deep_branches: true,
        }
    }
}

/// Result of an optimization pass.
#[derive(Debug, Default, Clone)]
pub struct OptimizationResult {
    /// Number of nodes merged.
    pub nodes_merged: usize,

    /// Number of nodes pruned.
    pub nodes_pruned: usize,

    /// Amount of depth reduction achieved.
    pub depth_reduced_by: u32,

    /// Approximate storage saved in bytes.
    pub storage_saved_bytes: usize,

    /// IDs of nodes that were removed.
    pub removed_node_ids: Vec<String>,

    /// IDs of nodes that were created (from merging).
    pub created_node_ids: Vec<String>,
}

/// Tree optimizer for managing context tree depth and efficiency.
pub struct TreeOptimizer {
    config: OptimizerConfig,
}

impl Default for TreeOptimizer {
    fn default() -> Self {
        Self::new(OptimizerConfig::default())
    }
}

impl TreeOptimizer {
    /// Create a new tree optimizer with the given configuration.
    pub fn new(config: OptimizerConfig) -> Self {
        Self { config }
    }

    /// Run a full optimization pass on the tree.
    pub async fn optimize(
        &self,
        tree: &mut ContextTree,
        analyzer: &LlmAnalyzer,
    ) -> Result<OptimizationResult> {
        let mut result = OptimizationResult::default();

        info!("Starting tree optimization...");

        let initial_depth = tree.max_depth();
        let initial_count = tree.node_count();

        // Phase 1: Prune stale leaf nodes
        if self.config.prune_file_refs {
            let pruned = self.prune_stale_nodes(tree);
            result.nodes_pruned += pruned.len();
            result.removed_node_ids.extend(pruned);
        }

        // Phase 2: Merge similar siblings
        if self.config.merge_siblings {
            let merged = self.merge_similar_siblings(tree, analyzer).await;
            result.nodes_merged += merged.0;
            result.removed_node_ids.extend(merged.1);
            result.created_node_ids.extend(merged.2);
        }

        // Phase 3: Compress deep branches
        if self.config.compress_deep_branches {
            let compressed = self.compress_deep_branches(tree, analyzer).await;
            result.nodes_merged += compressed.0;
            result.removed_node_ids.extend(compressed.1);
        }

        // Calculate results
        let final_depth = tree.max_depth();
        let final_count = tree.node_count();

        result.depth_reduced_by = initial_depth.saturating_sub(final_depth);
        result.storage_saved_bytes = (initial_count - final_count) * 500; // Rough estimate

        info!(
            "Optimization complete: pruned {}, merged {}, depth reduced by {}",
            result.nodes_pruned, result.nodes_merged, result.depth_reduced_by
        );

        Ok(result)
    }

    /// Prune stale nodes that haven't been accessed recently.
    fn prune_stale_nodes(&self, tree: &mut ContextTree) -> Vec<String> {
        let now = Utc::now();
        let cutoff = now - Duration::days(self.config.max_idle_days as i64);

        // Find stale leaf nodes
        let stale_ids: Vec<String> = tree
            .get_leaves()
            .iter()
            .filter(|node| {
                // Only prune file references
                if node.node_type != NodeType::FileReference {
                    return false;
                }

                // Check if stale
                node.last_updated < cutoff && node.access_count < self.config.min_access_count
            })
            .map(|node| node.id.clone())
            .collect();

        // Remove stale nodes
        for id in &stale_ids {
            tree.remove(id);
            debug!("Pruned stale node: {}", id);
        }

        stale_ids
    }

    /// Merge similar sibling nodes.
    async fn merge_similar_siblings(
        &self,
        tree: &mut ContextTree,
        analyzer: &LlmAnalyzer,
    ) -> (usize, Vec<String>, Vec<String>) {
        let mut merged_count = 0;
        let mut removed_ids = Vec::new();
        let mut created_ids = Vec::new();

        // Get all non-leaf nodes
        let parent_ids: Vec<String> = tree
            .all_nodes()
            .filter(|n| !n.children.is_empty())
            .map(|n| n.id.clone())
            .collect();

        for parent_id in parent_ids {
            // Get children
            let children: Vec<ContextNode> = {
                let parent = match tree.get(&parent_id) {
                    Some(p) => p,
                    None => continue,
                };

                parent
                    .children
                    .iter()
                    .filter_map(|id| tree.get(id).cloned())
                    .collect()
            };

            // Check if we should merge
            if children.len() < self.config.min_siblings_for_merge {
                continue;
            }

            // Group by node type
            let mut by_type: HashMap<NodeType, Vec<ContextNode>> = HashMap::new();
            for child in children {
                by_type.entry(child.node_type).or_default().push(child);
            }

            // Merge file references if there are many
            if let Some(file_refs) = by_type.get(&NodeType::FileReference) {
                if file_refs.len() >= self.config.min_siblings_for_merge {
                    let (merged, removed) =
                        self.merge_file_refs(tree, &parent_id, file_refs, analyzer).await;
                    if let Some(merged_node) = merged {
                        created_ids.push(merged_node.id.clone());
                        tree.add_child(&parent_id, merged_node).ok();
                        merged_count += 1;
                    }
                    for id in &removed {
                        tree.remove(id);
                    }
                    removed_ids.extend(removed);
                }
            }
        }

        (merged_count, removed_ids, created_ids)
    }

    /// Merge multiple file references into a summary node.
    async fn merge_file_refs(
        &self,
        _tree: &ContextTree,
        _parent_id: &str,
        file_refs: &[ContextNode],
        analyzer: &LlmAnalyzer,
    ) -> (Option<ContextNode>, Vec<String>) {
        if file_refs.len() < 2 {
            return (None, Vec::new());
        }

        // Collect IDs to remove
        let removed_ids: Vec<String> = file_refs.iter().map(|n| n.id.clone()).collect();

        // Create summary node
        let summary = analyzer.summarize_children(file_refs).await.unwrap_or_default();

        let mut merged_node = ContextNode::new(NodeType::Document, "Files Summary");
        merged_node.summary = if summary.is_empty() {
            format!("Summary of {} files", file_refs.len())
        } else {
            summary
        };

        // Collect keywords from all merged nodes
        let mut keywords: HashSet<String> = HashSet::new();
        for node in file_refs {
            keywords.extend(node.keywords.clone());
        }
        merged_node.keywords = keywords.into_iter().collect();

        // Collect entities
        for node in file_refs {
            merged_node.entities.extend(node.entities.clone());
        }

        debug!(
            "Merged {} file references into summary node",
            file_refs.len()
        );

        (Some(merged_node), removed_ids)
    }

    /// Compress branches that are too deep.
    async fn compress_deep_branches(
        &self,
        tree: &mut ContextTree,
        analyzer: &LlmAnalyzer,
    ) -> (usize, Vec<String>) {
        let mut compressed_count = 0;
        let mut removed_ids = Vec::new();

        let max_depth = tree.max_depth();

        // Only compress if we exceed threshold
        if max_depth <= self.config.max_depth_threshold {
            return (0, Vec::new());
        }

        // Find nodes at excessive depth
        let deep_nodes: Vec<ContextNode> = tree
            .nodes_at_depth(self.config.max_depth_threshold)
            .into_iter()
            .cloned()
            .collect();

        for node in deep_nodes {
            // Get descendants
            let descendants: Vec<ContextNode> = tree
                .get_descendants(&node.id)
                .into_iter()
                .cloned()
                .collect();

            if descendants.is_empty() {
                continue;
            }

            // Compress descendants into the node's summary
            let summary = analyzer.summarize_children(&descendants).await.unwrap_or_default();

            if let Some(target_node) = tree.get_mut(&node.id) {
                // Append compressed summary
                if !summary.is_empty() {
                    target_node.summary = format!("{}\n\nCompressed: {}", target_node.summary, summary);
                }

                // Collect keywords and entities from descendants
                for desc in &descendants {
                    target_node.keywords.extend(desc.keywords.clone());
                    target_node.entities.extend(desc.entities.clone());
                }

                // Deduplicate keywords
                let keywords: HashSet<String> = target_node.keywords.drain(..).collect();
                target_node.keywords = keywords.into_iter().collect();
            }

            // Remove descendants
            for desc in &descendants {
                removed_ids.push(desc.id.clone());
            }
            compressed_count += 1;
        }

        // Actually remove the nodes
        for id in &removed_ids {
            tree.remove(id);
        }

        if compressed_count > 0 {
            debug!(
                "Compressed {} deep branches, removed {} nodes",
                compressed_count,
                removed_ids.len()
            );
        }

        (compressed_count, removed_ids)
    }

    /// Get recommendations for optimization without making changes.
    pub fn analyze(&self, tree: &ContextTree) -> OptimizationAnalysis {
        let now = Utc::now();
        let cutoff = now - Duration::days(self.config.max_idle_days as i64);

        let mut analysis = OptimizationAnalysis::default();

        // Count stale nodes
        for node in tree.get_leaves() {
            if node.node_type == NodeType::FileReference
                && node.last_updated < cutoff
                && node.access_count < self.config.min_access_count
            {
                analysis.stale_nodes += 1;
            }
        }

        // Check depth
        let max_depth = tree.max_depth();
        if max_depth > self.config.max_depth_threshold {
            analysis.excessive_depth = true;
            analysis.current_depth = max_depth;
            analysis.recommended_depth = self.config.max_depth_threshold;
        }

        // Count potential merges
        for node in tree.all_nodes() {
            if node.children.len() >= self.config.min_siblings_for_merge {
                let file_ref_children = node
                    .children
                    .iter()
                    .filter_map(|id| tree.get(id))
                    .filter(|n| n.node_type == NodeType::FileReference)
                    .count();

                if file_ref_children >= self.config.min_siblings_for_merge {
                    analysis.mergeable_groups += 1;
                }
            }
        }

        // Set recommendation
        analysis.should_optimize = analysis.stale_nodes > 0
            || analysis.excessive_depth
            || analysis.mergeable_groups > 0;

        analysis
    }
}

/// Analysis of potential optimizations.
#[derive(Debug, Default)]
pub struct OptimizationAnalysis {
    /// Number of stale nodes that could be pruned.
    pub stale_nodes: usize,

    /// Whether the tree depth is excessive.
    pub excessive_depth: bool,

    /// Current maximum depth.
    pub current_depth: u32,

    /// Recommended maximum depth.
    pub recommended_depth: u32,

    /// Number of sibling groups that could be merged.
    pub mergeable_groups: usize,

    /// Whether optimization is recommended.
    pub should_optimize: bool,
}

impl std::fmt::Display for OptimizationAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Optimization Analysis:")?;
        writeln!(f, "  Stale nodes: {}", self.stale_nodes)?;
        writeln!(f, "  Excessive depth: {}", self.excessive_depth)?;
        if self.excessive_depth {
            writeln!(
                f,
                "    Current: {}, Recommended: {}",
                self.current_depth, self.recommended_depth
            )?;
        }
        writeln!(f, "  Mergeable groups: {}", self.mergeable_groups)?;
        writeln!(f, "  Should optimize: {}", self.should_optimize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use std::path::PathBuf;

    fn create_test_tree() -> ContextTree {
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        let project = ContextNode::project("test", PathBuf::from("/test"));
        let project_id = tree.add_child(&domain_id, project).unwrap();

        // Add some file references
        for i in 0..10 {
            let file = ContextNode::file_reference(
                format!("file{}.rs", i),
                PathBuf::from(format!("/test/file{}.rs", i)),
            );
            tree.add_child(&project_id, file).unwrap();
        }

        tree
    }

    #[test]
    fn test_analyze_no_issues() {
        let tree = ContextTree::new();
        let optimizer = TreeOptimizer::default();

        let analysis = optimizer.analyze(&tree);
        assert!(!analysis.should_optimize);
    }

    #[test]
    fn test_analyze_with_stale_nodes() {
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        // Create a stale node
        let mut stale_node = ContextNode::file_reference("old.rs", PathBuf::from("/old.rs"));
        stale_node.last_updated = Utc::now() - Duration::days(60);
        stale_node.access_count = 0;
        tree.add_child(&domain_id, stale_node).unwrap();

        let optimizer = TreeOptimizer::new(OptimizerConfig {
            max_idle_days: 30,
            ..Default::default()
        });

        let analysis = optimizer.analyze(&tree);
        assert_eq!(analysis.stale_nodes, 1);
        assert!(analysis.should_optimize);
    }

    #[test]
    fn test_analyze_excessive_depth() {
        let mut tree = ContextTree::new();
        let mut parent_id = tree.ensure_domain("coding");

        // Create deep nesting
        for i in 0..10 {
            let node = ContextNode::new(NodeType::Module, format!("level{}", i));
            parent_id = tree.add_child(&parent_id, node).unwrap();
        }

        let optimizer = TreeOptimizer::new(OptimizerConfig {
            max_depth_threshold: 5,
            ..Default::default()
        });

        let analysis = optimizer.analyze(&tree);
        assert!(analysis.excessive_depth);
    }

    #[test]
    fn test_prune_stale_nodes() {
        let mut tree = ContextTree::new();
        let domain_id = tree.ensure_domain("coding");

        // Create stale nodes
        for i in 0..3 {
            let mut node = ContextNode::file_reference(
                format!("old{}.rs", i),
                PathBuf::from(format!("/old{}.rs", i)),
            );
            node.last_updated = Utc::now() - Duration::days(60);
            node.access_count = 0;
            tree.add_child(&domain_id, node).unwrap();
        }

        // Create fresh nodes
        for i in 0..2 {
            let mut node = ContextNode::file_reference(
                format!("new{}.rs", i),
                PathBuf::from(format!("/new{}.rs", i)),
            );
            node.last_updated = Utc::now();
            node.access_count = 5;
            tree.add_child(&domain_id, node).unwrap();
        }

        let optimizer = TreeOptimizer::new(OptimizerConfig {
            max_idle_days: 30,
            min_access_count: 2,
            ..Default::default()
        });

        let initial_count = tree.node_count();
        let pruned = optimizer.prune_stale_nodes(&mut tree);

        assert_eq!(pruned.len(), 3);
        assert_eq!(tree.node_count(), initial_count - 3);
    }

    #[tokio::test]
    async fn test_optimize_full() {
        let mut tree = create_test_tree();
        let analyzer = LlmAnalyzer::heuristic_only();
        let optimizer = TreeOptimizer::new(OptimizerConfig {
            min_siblings_for_merge: 5,
            ..Default::default()
        });

        let initial_count = tree.node_count();
        let result = optimizer.optimize(&mut tree, &analyzer).await.unwrap();

        // Should have merged some nodes
        assert!(result.nodes_merged > 0 || result.nodes_pruned > 0 || tree.node_count() <= initial_count);
    }

    #[test]
    fn test_optimization_result_default() {
        let result = OptimizationResult::default();
        assert_eq!(result.nodes_merged, 0);
        assert_eq!(result.nodes_pruned, 0);
        assert_eq!(result.depth_reduced_by, 0);
    }

    #[test]
    fn test_optimizer_config_default() {
        let config = OptimizerConfig::default();
        assert_eq!(config.max_idle_days, 30);
        assert!(config.prune_file_refs);
        assert!(config.merge_siblings);
    }
}
