use std::path::Path;
use std::sync::Arc;

use codex_app_server_protocol::ContextNodeSummary;
use codex_app_server_protocol::GetNodeContextParams;
use codex_app_server_protocol::GetNodeContextResponse;
use codex_app_server_protocol::IndexCompleteNotification;
use codex_app_server_protocol::IndexDirectoryParams;
use codex_app_server_protocol::IndexDirectoryResponse;
use codex_app_server_protocol::IndexProgressNotification;
use codex_app_server_protocol::IndexStatus;
use codex_app_server_protocol::ListDomainsParams;
use codex_app_server_protocol::ListDomainsResponse;
use codex_app_server_protocol::QueryContextParams;
use codex_app_server_protocol::QueryContextResponse;
use codex_app_server_protocol::ServerNotification;
use codex_context_files::AgentBuilder;
use codex_context_files::ContextAgent;
use codex_context_files::ContextNode;
use codex_context_files::TreeStore;
use tokio::sync::RwLock;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::error_code::internal_error;
use crate::error_code::invalid_params;
use crate::outgoing_message::OutgoingMessageSender;
use codex_app_server_protocol::JSONRPCErrorError;

/// Processes requests for William's persistent agentic context tree.
pub(crate) struct ContextRequestProcessor {
    agent: RwLock<ContextAgent>,
    outgoing: Arc<OutgoingMessageSender>,
    store: TreeStore,
}

impl ContextRequestProcessor {
    pub(crate) fn new(
        codex_home: &Path,
        outgoing: Arc<OutgoingMessageSender>,
    ) -> ContextRequestProcessor {
        let store = TreeStore::new(codex_home.join("context"));
        let mut agent = AgentBuilder::new().heuristic_only().build();

        if store.exists() {
            match store.load() {
                Ok(tree) => {
                    *agent.tree_mut() = tree;
                    agent.tree_mut().ensure_root();
                }
                Err(err) => {
                    warn!(
                        store = %store.base_path().display(),
                        "failed to load context tree; starting with an empty tree: {err}"
                    );
                }
            }
        }

        Self {
            agent: RwLock::new(agent),
            outgoing,
            store,
        }
    }

    #[tracing::instrument(skip_all, fields(path = %params.path))]
    pub(crate) async fn index_directory(
        &self,
        params: IndexDirectoryParams,
    ) -> Result<IndexDirectoryResponse, JSONRPCErrorError> {
        let path = Path::new(&params.path);
        if !path.is_dir() {
            return Err(invalid_params(format!(
                "context index path is not a directory: {}",
                path.display()
            )));
        }

        self.outgoing
            .send_server_notification(ServerNotification::IndexProgress(
                IndexProgressNotification {
                    status: IndexStatus::Starting {
                        path: params.path.clone(),
                    },
                },
            ))
            .await;

        let mut agent = self.agent.write().await;
        let result = match agent.process_folder(path).await {
            Ok(result) => result,
            Err(err) => {
                error!("failed to index context directory: {err}");
                self.outgoing
                    .send_server_notification(ServerNotification::IndexProgress(
                        IndexProgressNotification {
                            status: IndexStatus::Error {
                                message: err.to_string(),
                            },
                        },
                    ))
                    .await;
                return Err(internal_error(format!(
                    "failed to index context directory: {err}"
                )));
            }
        };

        self.store
            .save(agent.tree())
            .map_err(|err| internal_error(format!("failed to persist context index: {err}")))?;
        let stats = agent.stats();
        info!(
            files_processed = result.files_processed,
            nodes_created = result.nodes_created,
            entities_extracted = result.entities_extracted,
            processing_time_ms = result.processing_time_ms,
            total_nodes = stats.total_nodes,
            "context indexing completed"
        );
        drop(agent);

        self.outgoing
            .send_server_notification(ServerNotification::IndexComplete(
                IndexCompleteNotification {
                    domain: result.domain,
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

    pub(crate) async fn query_context(
        &self,
        params: QueryContextParams,
    ) -> Result<QueryContextResponse, JSONRPCErrorError> {
        let agent = self.agent.read().await;
        let result = agent.query(&params.query);
        let nodes = result
            .nodes
            .iter()
            .take(params.max_results)
            .map(node_to_summary)
            .collect();

        Ok(QueryContextResponse {
            nodes,
            processing_time_ms: result.processing_time_ms,
        })
    }

    pub(crate) async fn get_node_context(
        &self,
        params: GetNodeContextParams,
    ) -> Result<GetNodeContextResponse, JSONRPCErrorError> {
        let agent = self.agent.read().await;
        let tree = agent.tree();
        let node = tree
            .get(&params.node_id)
            .ok_or_else(|| invalid_params(format!("context node not found: {}", params.node_id)))?;
        let ancestry = tree
            .get_ancestry(&params.node_id)
            .into_iter()
            .map(node_to_summary)
            .collect();
        let related = node
            .related_nodes
            .iter()
            .filter_map(|related| tree.get(&related.node_id))
            .map(node_to_summary)
            .collect();

        Ok(GetNodeContextResponse {
            node: node_to_summary(node),
            ancestry,
            related,
        })
    }

    pub(crate) async fn list_domains(
        &self,
        _params: ListDomainsParams,
    ) -> Result<ListDomainsResponse, JSONRPCErrorError> {
        let agent = self.agent.read().await;
        let domains = agent
            .list_domains()
            .into_iter()
            .map(str::to_owned)
            .collect();

        Ok(ListDomainsResponse { domains })
    }
}

fn node_to_summary(node: &ContextNode) -> ContextNodeSummary {
    ContextNodeSummary {
        id: node.id.clone(),
        name: node.name.clone(),
        node_type: node.node_type.label().to_string(),
        path: node.path.as_ref().map(|path| path.display().to_string()),
        summary: node.summary.clone(),
        depth: node.depth,
        keywords: node.keywords.clone(),
    }
}
