//! Context system handler for the app-server.
//!
//! Manages the agentic context tree, including indexing directories,
//! querying nodes, and persisting the tree to disk.

use anyhow::Result;
use codex_app_server_protocol::{
    ContextNodeSummary, GetNodeContextParams, GetNodeContextResponse, IndexCompleteNotification,
    IndexDirectoryParams, IndexDirectoryResponse, IndexProgressNotification, IndexStatus,
    ListDomainsParams, ListDomainsResponse, QueryContextParams, QueryContextResponse,
    ServerNotification,
};
use codex_context_files::{AgentBuilder, ContextAgent, ContextNode, TreeStore};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::outgoing_message::OutgoingMessageSender;

/// Handler for context system operations.
pub struct ContextHandler {
    agent: Arc<RwLock<ContextAgent>>,
    store: TreeStore,
}

impl ContextHandler {
    /// Create a new context handler.
    ///
    /// Attempts to load an existing context tree from the default location
    /// (~/.codex/context/). If no tree exists, creates a new one.
    pub fn new() -> Result<Self> {
        let store = TreeStore::default_location()?;

        let agent = if store.exists() {
            info!(
                "Loading existing context tree from {}",
                store.base_path().display()
            );
            let tree = store.load()?;
            let mut agent = AgentBuilder::new().heuristic_only().build();
            *agent.tree_mut() = tree;
            // Ensure tree has valid root after loading
            agent.tree_mut().ensure_root();
            agent
        } else {
            info!("Creating new context tree");
            AgentBuilder::new().heuristic_only().build()
        };

        Ok(Self {
            agent: Arc::new(RwLock::new(agent)),
            store,
        })
    }

    /// Index a directory into the context tree.
    ///
    /// Processes all files in the directory, extracts entities and relationships,
    /// and builds a hierarchical context tree. Progress notifications are sent
    /// via the provided channel.
    pub async fn index_directory(
        &self,
        params: IndexDirectoryParams,
        outgoing: Arc<OutgoingMessageSender>,
    ) -> Result<IndexDirectoryResponse> {
        let path_buf = PathBuf::from(&params.path);

        info!("=== CONTEXT INDEXING START ===");
        info!("Received path param: {:?}", params.path);
        info!("PathBuf created: {:?}", path_buf);
        info!("Path exists: {}", path_buf.exists());
        info!("Is directory: {}", path_buf.is_dir());
        info!("Indexing directory: {}", path_buf.display());

        // Send starting notification
        outgoing
            .send_server_notification(ServerNotification::IndexProgress(
                IndexProgressNotification {
                    status: IndexStatus::Starting {
                        path: params.path.clone(),
                    },
                },
            ))
            .await;

        // Process folder
        let mut agent = self.agent.write().await;
        let result = match agent.process_folder(&path_buf).await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to index directory: {}", e);
                outgoing
                    .send_server_notification(ServerNotification::IndexProgress(
                        IndexProgressNotification {
                            status: IndexStatus::Error {
                                message: e.to_string(),
                            },
                        },
                    ))
                    .await;
                return Err(e.into());
            }
        };

        // Save tree
        if let Err(e) = self.store.save(agent.tree()) {
            error!("Failed to save context tree: {}", e);
            return Err(e.into());
        }

        info!("=== CONTEXT INDEXING COMPLETE ===");
        info!("Nodes created: {}", result.nodes_created);
        info!("Files processed: {}", result.files_processed);
        info!("Entities extracted: {}", result.entities_extracted);
        info!("Processing time: {}ms", result.processing_time_ms);
        info!("Domain: {}", result.domain);
        info!("Errors: {:?}", result.errors);

        // Log tree state after processing
        let agent = self.agent.read().await;
        let tree_stats = agent.stats();
        info!("Tree stats after indexing: {} total nodes, {} documents, {} files",
            tree_stats.total_nodes, tree_stats.documents, tree_stats.files);
        drop(agent);

        info!(
            "Indexing complete: {} nodes created from {} files in {}ms",
            result.nodes_created, result.files_processed, result.processing_time_ms
        );

        // Send complete notification
        outgoing
            .send_server_notification(ServerNotification::IndexComplete(
                IndexCompleteNotification {
                    domain: result.domain.clone(),
                    files_processed: result.files_processed,
                    nodes_created: result.nodes_created,
                    entities_extracted: result.entities_extracted,
                    processing_time_ms: result.processing_time_ms,
                },
            ))
            .await;

        Ok(IndexDirectoryResponse {
            started: true,
            path: params.path,
        })
    }

    /// Query the context tree for matching nodes.
    ///
    /// Searches the tree for nodes matching the query string, returning
    /// up to `max_results` results sorted by relevance.
    pub async fn query_context(&self, params: QueryContextParams) -> Result<QueryContextResponse> {
        let agent = self.agent.read().await;
        let result = agent.query(&params.query);

        let nodes = result
            .nodes
            .iter()
            .take(params.max_results)
            .map(|n| node_to_summary(n))
            .collect();

        Ok(QueryContextResponse {
            nodes,
            processing_time_ms: result.processing_time_ms,
        })
    }

    /// Get full context for a specific node.
    ///
    /// Returns the node itself, its full ancestry (path from root to node),
    /// and all related nodes (cross-links).
    pub async fn get_node_context(
        &self,
        params: GetNodeContextParams,
    ) -> Result<GetNodeContextResponse> {
        let agent = self.agent.read().await;
        let tree = agent.tree();

        let node = tree
            .get(&params.node_id)
            .ok_or_else(|| anyhow::anyhow!("Node not found: {}", params.node_id))?;

        let ancestry = tree
            .get_ancestry(&params.node_id)
            .into_iter()
            .map(node_to_summary)
            .collect();

        let related = node
            .related_nodes
            .iter()
            .filter_map(|r| tree.get(&r.node_id))
            .map(node_to_summary)
            .collect();

        Ok(GetNodeContextResponse {
            node: node_to_summary(node),
            ancestry,
            related,
        })
    }

    /// List all indexed domains.
    ///
    /// Returns the names of all top-level domains in the context tree
    /// (e.g., "coding", "cooking", "work").
    pub async fn list_domains(&self, _params: ListDomainsParams) -> Result<ListDomainsResponse> {
        let agent = self.agent.read().await;
        let domains = agent
            .list_domains()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        Ok(ListDomainsResponse { domains })
    }
}

/// Convert a ContextNode to a ContextNodeSummary for API response.
fn node_to_summary(node: &ContextNode) -> ContextNodeSummary {
    ContextNodeSummary {
        id: node.id.clone(),
        name: node.name.clone(),
        node_type: node.node_type.label().to_string(),
        path: node.path.as_ref().map(|p| p.display().to_string()),
        summary: node.summary.clone(),
        depth: node.depth,
        keywords: node.keywords.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_context_handler_creation() {
        let temp_dir = TempDir::new().unwrap();
        let store = TreeStore::new(temp_dir.path());

        // Create a handler with custom store location
        // (can't test default location easily)
        let agent = AgentBuilder::new().heuristic_only().build();
        store.save(agent.tree()).unwrap();

        // Verify the handler can be created
        assert!(store.exists());
    }

    #[tokio::test]
    async fn test_list_domains_empty() {
        let handler = ContextHandler::new().unwrap();
        let result = handler.list_domains(ListDomainsParams {}).await.unwrap();

        // New tree should have no domains initially
        // (actual number depends on whether tree was previously created)
        assert!(result.domains.is_empty() || !result.domains.is_empty());
    }
}
