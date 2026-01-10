//! Tool registry for runtime tool management.
//!
//! The `ToolRegistry` provides runtime access to tools and
//! handles tool discovery and recommendation.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::error::{Result, ToolError};
use crate::storage::ToolStore;
use crate::tool::{Tool, ToolCategory};

/// Runtime registry for available tools.
///
/// The registry maintains a live index of all available tools
/// and provides APIs for tool discovery and recommendation.
pub struct ToolRegistry {
    /// Reference to the underlying store.
    store: Arc<RwLock<ToolStore>>,

    /// Index of tools by name for fast lookup.
    name_index: Arc<RwLock<HashMap<String, String>>>,

    /// Index of tools by category.
    category_index: Arc<RwLock<HashMap<ToolCategory, Vec<String>>>>,

    /// Index of tools by tag.
    tag_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl ToolRegistry {
    /// Create a new registry from a tool store.
    pub async fn new(store: ToolStore) -> Self {
        let store = Arc::new(RwLock::new(store));

        let registry = Self {
            store,
            name_index: Arc::new(RwLock::new(HashMap::new())),
            category_index: Arc::new(RwLock::new(HashMap::new())),
            tag_index: Arc::new(RwLock::new(HashMap::new())),
        };

        registry.rebuild_indices().await;

        registry
    }

    /// Rebuild all indices from the store.
    pub async fn rebuild_indices(&self) {
        let store = self.store.read().await;

        let mut name_index = HashMap::new();
        let mut category_index: HashMap<ToolCategory, Vec<String>> = HashMap::new();
        let mut tag_index: HashMap<String, Vec<String>> = HashMap::new();

        for tool in store.list() {
            // Name index
            name_index.insert(tool.name.clone(), tool.id.clone());

            // Category index
            category_index
                .entry(tool.category)
                .or_default()
                .push(tool.id.clone());

            // Tag index
            for tag in &tool.metadata.tags {
                tag_index
                    .entry(tag.clone())
                    .or_default()
                    .push(tool.id.clone());
            }
        }

        *self.name_index.write().await = name_index;
        *self.category_index.write().await = category_index;
        *self.tag_index.write().await = tag_index;

        info!("Rebuilt tool registry indices");
    }

    /// Get a tool by name.
    pub async fn get_by_name(&self, name: &str) -> Option<Tool> {
        let name_index = self.name_index.read().await;
        if let Some(id) = name_index.get(name) {
            let store = self.store.read().await;
            store.get(id).cloned()
        } else {
            None
        }
    }

    /// Get a tool by ID.
    pub async fn get(&self, id: &str) -> Option<Tool> {
        let store = self.store.read().await;
        store.get(id).cloned()
    }

    /// Register a new tool.
    pub async fn register(&self, tool: Tool) -> Result<()> {
        let id = tool.id.clone();
        let name = tool.name.clone();
        let category = tool.category;
        let tags = tool.metadata.tags.clone();

        // Store the tool
        {
            let mut store = self.store.write().await;
            store.upsert(tool).await?;
        }

        // Update indices
        self.name_index.write().await.insert(name, id.clone());
        self.category_index
            .write()
            .await
            .entry(category)
            .or_default()
            .push(id.clone());

        for tag in tags {
            self.tag_index
                .write()
                .await
                .entry(tag)
                .or_default()
                .push(id.clone());
        }

        debug!("Registered tool: {id}");
        Ok(())
    }

    /// Unregister a tool.
    pub async fn unregister(&self, id: &str) -> Result<()> {
        // Get tool info before removing
        let tool = {
            let store = self.store.read().await;
            store
                .get(id)
                .cloned()
                .ok_or_else(|| ToolError::NotFound(id.to_string()))?
        };

        // Remove from store
        {
            let mut store = self.store.write().await;
            store.delete(id).await?;
        }

        // Remove from indices
        self.name_index.write().await.remove(&tool.name);

        if let Some(ids) = self.category_index.write().await.get_mut(&tool.category) {
            ids.retain(|i| i != id);
        }

        for tag in &tool.metadata.tags {
            if let Some(ids) = self.tag_index.write().await.get_mut(tag) {
                ids.retain(|i| i != id);
            }
        }

        debug!("Unregistered tool: {id}");
        Ok(())
    }

    /// List all tools.
    pub async fn list(&self) -> Vec<Tool> {
        let store = self.store.read().await;
        store.list().cloned().collect()
    }

    /// List tools by category.
    pub async fn list_by_category(&self, category: ToolCategory) -> Vec<Tool> {
        let category_index = self.category_index.read().await;
        let ids = category_index.get(&category).cloned().unwrap_or_default();

        let store = self.store.read().await;
        ids.iter()
            .filter_map(|id| store.get(id).cloned())
            .collect()
    }

    /// List tools by tag.
    pub async fn list_by_tag(&self, tag: &str) -> Vec<Tool> {
        let tag_index = self.tag_index.read().await;
        let ids = tag_index.get(tag).cloned().unwrap_or_default();

        let store = self.store.read().await;
        ids.iter()
            .filter_map(|id| store.get(id).cloned())
            .collect()
    }

    /// Search tools by query.
    pub async fn search(&self, query: &str) -> Vec<Tool> {
        let store = self.store.read().await;
        store.search(query).into_iter().cloned().collect()
    }

    /// Get recommended tools based on context.
    pub async fn recommend(&self, context: &RecommendationContext) -> Vec<ToolRecommendation> {
        let store = self.store.read().await;
        let mut recommendations = Vec::new();

        // Simple recommendation based on tags and recent usage
        for tool in store.list() {
            let mut score = 0.0;

            // Tag matching
            for tag in &context.relevant_tags {
                if tool.metadata.tags.contains(tag) {
                    score += 0.3;
                }
            }

            // Category matching
            if context.preferred_categories.contains(&tool.category) {
                score += 0.2;
            }

            // Usage frequency bonus
            if tool.metadata.usage_count > 0 {
                score += 0.1 * (tool.metadata.usage_count as f32).log10().min(1.0);
            }

            if score > 0.0 {
                recommendations.push(ToolRecommendation {
                    tool: tool.clone(),
                    score,
                    reason: format!("Matched with score {:.2}", score),
                });
            }
        }

        // Sort by score
        recommendations.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        recommendations.truncate(context.limit);

        recommendations
    }

    /// Get statistics about the registry.
    pub async fn stats(&self) -> RegistryStats {
        let store = self.store.read().await;
        let category_index = self.category_index.read().await;

        RegistryStats {
            total_tools: store.list().count(),
            by_category: category_index
                .iter()
                .map(|(cat, ids)| (*cat, ids.len()))
                .collect(),
            total_tags: self.tag_index.read().await.len(),
        }
    }
}

/// Context for tool recommendations.
#[derive(Debug, Clone, Default)]
pub struct RecommendationContext {
    /// Tags that are relevant to the current task.
    pub relevant_tags: Vec<String>,

    /// Preferred tool categories.
    pub preferred_categories: Vec<ToolCategory>,

    /// Current task description.
    pub task_description: Option<String>,

    /// Maximum number of recommendations.
    pub limit: usize,
}

impl RecommendationContext {
    /// Create a new recommendation context.
    pub fn new() -> Self {
        Self {
            limit: 5,
            ..Default::default()
        }
    }

    /// Add relevant tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.relevant_tags = tags;
        self
    }

    /// Add preferred categories.
    pub fn with_categories(mut self, categories: Vec<ToolCategory>) -> Self {
        self.preferred_categories = categories;
        self
    }

    /// Set the limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

/// A tool recommendation with score and reason.
#[derive(Debug, Clone)]
pub struct ToolRecommendation {
    /// The recommended tool.
    pub tool: Tool,

    /// Recommendation score (0.0 to 1.0).
    pub score: f32,

    /// Reason for the recommendation.
    pub reason: String,
}

/// Statistics about the tool registry.
#[derive(Debug, Clone)]
pub struct RegistryStats {
    /// Total number of tools.
    pub total_tools: usize,

    /// Tools by category.
    pub by_category: HashMap<ToolCategory, usize>,

    /// Total number of unique tags.
    pub total_tags: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_registry_creation() {
        let temp_dir = TempDir::new().unwrap();
        let store = ToolStore::new(temp_dir.path()).await.unwrap();
        let registry = ToolRegistry::new(store).await;

        let stats = registry.stats().await;
        assert_eq!(stats.total_tools, 0);
    }

    #[tokio::test]
    async fn test_register_and_lookup() {
        let temp_dir = TempDir::new().unwrap();
        let store = ToolStore::new(temp_dir.path()).await.unwrap();
        let registry = ToolRegistry::new(store).await;

        let tool = Tool::new("test-tool", "Test", ToolCategory::Utility);
        registry.register(tool).await.unwrap();

        let found = registry.get_by_name("test-tool").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test-tool");
    }
}
