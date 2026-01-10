//! Tool storage and persistence.
//!
//! The `ToolStore` handles reading and writing tools to disk.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tokio::fs;
use tracing::{debug, info, warn};

use crate::error::{Result, StorageError, ToolError};
use crate::tool::{Tool, ToolCategory};

/// Storage backend for tools.
pub struct ToolStore {
    /// Root directory for tool storage.
    root: PathBuf,

    /// In-memory cache of tools.
    cache: HashMap<String, Tool>,
}

impl ToolStore {
    /// Create a new tool store at the given root directory.
    pub async fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();

        // Create directory structure
        fs::create_dir_all(&root)
            .await
            .map_err(|e| StorageError::CreateDirectory(format!("{}: {e}", root.display())))?;

        // Create category subdirectories
        for category in &["mcp-servers", "file-handlers", "app-integrators", "workflows", "utilities"] {
            let category_path = root.join(category);
            fs::create_dir_all(&category_path)
                .await
                .map_err(|e| StorageError::CreateDirectory(format!("{}: {e}", category_path.display())))?;
        }

        let mut store = Self {
            root,
            cache: HashMap::new(),
        };

        store.load_all().await?;

        Ok(store)
    }

    /// Get the path for a tool file.
    fn tool_path(&self, tool: &Tool) -> PathBuf {
        let category_dir = self.category_dir(tool.category);
        category_dir.join(format!("{}.json", tool.name))
    }

    /// Get the directory for a tool category.
    fn category_dir(&self, category: ToolCategory) -> PathBuf {
        let dir_name = match category {
            ToolCategory::McpServer => "mcp-servers",
            ToolCategory::FileHandler => "file-handlers",
            ToolCategory::AppIntegrator => "app-integrators",
            ToolCategory::Workflow => "workflows",
            ToolCategory::Utility | ToolCategory::Custom => "utilities",
        };
        self.root.join(dir_name)
    }

    /// Load all tools from disk.
    async fn load_all(&mut self) -> Result<()> {
        let categories = ["mcp-servers", "file-handlers", "app-integrators", "workflows", "utilities"];

        for category in categories {
            let category_path = self.root.join(category);
            if !category_path.exists() {
                continue;
            }

            let mut entries = fs::read_dir(&category_path)
                .await
                .map_err(|e| StorageError::ReadFile(format!("{e}")))?;

            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| StorageError::ReadFile(format!("{e}")))?
            {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "json") {
                    match self.load_file(&path).await {
                        Ok(tool) => {
                            debug!("Loaded tool: {}", tool.name);
                            self.cache.insert(tool.id.clone(), tool);
                        }
                        Err(e) => {
                            warn!("Failed to load tool {}: {e}", path.display());
                        }
                    }
                }
            }
        }

        info!("Loaded {} tools", self.cache.len());
        Ok(())
    }

    /// Load a single tool from disk.
    async fn load_file(&self, path: &Path) -> Result<Tool> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| StorageError::ReadFile(format!("{}: {e}", path.display())))?;

        let tool: Tool = serde_json::from_str(&content)?;
        Ok(tool)
    }

    /// Save a tool to disk.
    async fn save_file(&self, tool: &Tool) -> Result<()> {
        let path = self.tool_path(tool);
        let content = serde_json::to_string_pretty(tool)?;

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::CreateDirectory(format!("{e}")))?;
        }

        // Write atomically
        let temp_path = path.with_extension("json.tmp");
        fs::write(&temp_path, &content)
            .await
            .map_err(|e| StorageError::WriteFile(format!("{}: {e}", temp_path.display())))?;

        fs::rename(&temp_path, &path)
            .await
            .map_err(|e| StorageError::WriteFile(format!("{}: {e}", path.display())))?;

        debug!("Saved tool: {}", tool.name);
        Ok(())
    }

    /// Get a tool by ID.
    pub fn get(&self, id: &str) -> Option<&Tool> {
        self.cache.get(id)
    }

    /// Get a tool by name.
    pub fn get_by_name(&self, name: &str) -> Option<&Tool> {
        self.cache.values().find(|t| t.name == name)
    }

    /// Insert or update a tool.
    pub async fn upsert(&mut self, tool: Tool) -> Result<()> {
        self.save_file(&tool).await?;
        self.cache.insert(tool.id.clone(), tool);
        Ok(())
    }

    /// Delete a tool.
    pub async fn delete(&mut self, id: &str) -> Result<()> {
        let tool = self
            .cache
            .get(id)
            .ok_or_else(|| ToolError::NotFound(id.to_string()))?;

        let path = self.tool_path(tool);
        fs::remove_file(&path)
            .await
            .map_err(|e| StorageError::DeleteFile(format!("{}: {e}", path.display())))?;

        self.cache.remove(id);
        info!("Deleted tool: {id}");
        Ok(())
    }

    /// List all tools.
    pub fn list(&self) -> impl Iterator<Item = &Tool> {
        self.cache.values()
    }

    /// List tools by category.
    pub fn list_by_category(&self, category: ToolCategory) -> Vec<&Tool> {
        self.cache
            .values()
            .filter(|t| t.category == category)
            .collect()
    }

    /// Search tools by name or description.
    pub fn search(&self, query: &str) -> Vec<&Tool> {
        let query_lower = query.to_lowercase();
        self.cache
            .values()
            .filter(|t| {
                t.name.to_lowercase().contains(&query_lower)
                    || t.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Get tools with a specific tag.
    pub fn find_by_tag(&self, tag: &str) -> Vec<&Tool> {
        self.cache
            .values()
            .filter(|t| t.metadata.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Get the most used tools.
    pub fn most_used(&self, limit: usize) -> Vec<&Tool> {
        let mut tools: Vec<_> = self.cache.values().collect();
        tools.sort_by(|a, b| b.metadata.usage_count.cmp(&a.metadata.usage_count));
        tools.truncate(limit);
        tools
    }

    /// Get recently updated tools.
    pub fn recently_updated(&self, limit: usize) -> Vec<&Tool> {
        let mut tools: Vec<_> = self.cache.values().collect();
        tools.sort_by(|a, b| b.metadata.last_updated.cmp(&a.metadata.last_updated));
        tools.truncate(limit);
        tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_tool_store_creation() {
        let temp_dir = TempDir::new().unwrap();
        let store = ToolStore::new(temp_dir.path()).await.unwrap();
        assert_eq!(store.cache.len(), 0);
    }

    #[tokio::test]
    async fn test_tool_persistence() {
        let temp_dir = TempDir::new().unwrap();

        let tool_id;
        {
            let mut store = ToolStore::new(temp_dir.path()).await.unwrap();
            let tool = Tool::new("test-tool", "A test tool", ToolCategory::Utility);
            tool_id = tool.id.clone();
            store.upsert(tool).await.unwrap();
        }

        // Reload and verify
        {
            let store = ToolStore::new(temp_dir.path()).await.unwrap();
            let tool = store.get(&tool_id).unwrap();
            assert_eq!(tool.name, "test-tool");
        }
    }
}
