//! Context file storage and persistence.
//!
//! The `ContextStore` handles reading and writing context files to disk,
//! maintaining an index, and ensuring atomic updates.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tokio::fs;
use tracing::{debug, info, warn};

use crate::context_file::ContextFile;
use crate::error::{ContextError, Result, StorageError};

/// Storage backend for context files.
///
/// Context files are stored as JSON files in a dedicated directory,
/// with an index file for fast lookup.
pub struct ContextStore {
    /// Root directory for context file storage.
    root: PathBuf,

    /// In-memory cache of context files.
    cache: HashMap<String, ContextFile>,

    /// Whether the cache is dirty and needs to be written.
    dirty: bool,
}

impl ContextStore {
    /// Create a new context store at the given root directory.
    ///
    /// This will create the directory if it doesn't exist.
    pub async fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();

        // Create the directory if it doesn't exist
        fs::create_dir_all(&root)
            .await
            .map_err(|e| StorageError::CreateDirectory(format!("{}: {e}", root.display())))?;

        let mut store = Self {
            root,
            cache: HashMap::new(),
            dirty: false,
        };

        // Load existing context files
        store.load_all().await?;

        Ok(store)
    }

    /// Get the path for a context file.
    fn context_path(&self, concept: &str) -> PathBuf {
        self.root.join(format!("{concept}.json"))
    }

    /// Load all context files from disk.
    async fn load_all(&mut self) -> Result<()> {
        let mut entries = fs::read_dir(&self.root)
            .await
            .map_err(|e| StorageError::ReadFile(format!("{}: {e}", self.root.display())))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| StorageError::ReadFile(format!("{e}")))?
        {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                match self.load_file(&path).await {
                    Ok(cf) => {
                        debug!("Loaded context file: {}", cf.concept);
                        self.cache.insert(cf.concept.clone(), cf);
                    }
                    Err(e) => {
                        warn!("Failed to load context file {}: {e}", path.display());
                    }
                }
            }
        }

        info!("Loaded {} context files", self.cache.len());
        Ok(())
    }

    /// Load a single context file from disk.
    async fn load_file(&self, path: &Path) -> Result<ContextFile> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| StorageError::ReadFile(format!("{}: {e}", path.display())))?;

        let cf: ContextFile = serde_json::from_str(&content)?;
        Ok(cf)
    }

    /// Save a context file to disk.
    async fn save_file(&self, cf: &ContextFile) -> Result<()> {
        let path = self.context_path(&cf.concept);
        let content = serde_json::to_string_pretty(cf)?;

        // Write atomically using a temp file
        let temp_path = path.with_extension("json.tmp");
        fs::write(&temp_path, &content)
            .await
            .map_err(|e| StorageError::WriteFile(format!("{}: {e}", temp_path.display())))?;

        fs::rename(&temp_path, &path)
            .await
            .map_err(|e| StorageError::WriteFile(format!("{}: {e}", path.display())))?;

        debug!("Saved context file: {}", cf.concept);
        Ok(())
    }

    /// Get a context file by concept name.
    pub fn get(&self, concept: &str) -> Option<&ContextFile> {
        self.cache.get(concept)
    }

    /// Get a mutable reference to a context file.
    pub fn get_mut(&mut self, concept: &str) -> Option<&mut ContextFile> {
        self.dirty = true;
        self.cache.get_mut(concept)
    }

    /// Insert or update a context file.
    pub async fn upsert(&mut self, cf: ContextFile) -> Result<()> {
        self.save_file(&cf).await?;
        self.cache.insert(cf.concept.clone(), cf);
        Ok(())
    }

    /// Create a new context file for a concept.
    pub async fn create(
        &mut self,
        concept: impl Into<String>,
        summary: impl Into<String>,
    ) -> Result<&ContextFile> {
        let concept = concept.into();
        if self.cache.contains_key(&concept) {
            return Err(ContextError::ConceptExists(concept));
        }

        let cf = ContextFile::new(&concept, summary);
        self.save_file(&cf).await?;
        self.cache.insert(concept.clone(), cf);
        Ok(self.cache.get(&concept).unwrap())
    }

    /// Delete a context file.
    pub async fn delete(&mut self, concept: &str) -> Result<()> {
        if !self.cache.contains_key(concept) {
            return Err(ContextError::NotFound(concept.to_string()));
        }

        let path = self.context_path(concept);
        fs::remove_file(&path)
            .await
            .map_err(|e| StorageError::DeleteFile(format!("{}: {e}", path.display())))?;

        self.cache.remove(concept);
        info!("Deleted context file: {concept}");
        Ok(())
    }

    /// List all concept names.
    pub fn list_concepts(&self) -> Vec<&str> {
        self.cache.keys().map(String::as_str).collect()
    }

    /// Get all context files.
    pub fn all(&self) -> impl Iterator<Item = &ContextFile> {
        self.cache.values()
    }

    /// Search context files by tags.
    pub fn search_by_tags(&self, tags: &[&str]) -> Vec<&ContextFile> {
        self.cache
            .values()
            .filter(|cf| tags.iter().any(|tag| cf.metadata.tags.contains(&tag.to_string())))
            .collect()
    }

    /// Flush any pending writes.
    pub async fn flush(&mut self) -> Result<()> {
        if self.dirty {
            for cf in self.cache.values() {
                self.save_file(cf).await?;
            }
            self.dirty = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_context_store_create_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = ContextStore::new(temp_dir.path()).await.unwrap();

        store.create("friends", "Information about friends").await.unwrap();

        let cf = store.get("friends").unwrap();
        assert_eq!(cf.concept, "friends");
    }

    #[tokio::test]
    async fn test_context_store_persistence() {
        let temp_dir = TempDir::new().unwrap();

        // Create and save
        {
            let mut store = ContextStore::new(temp_dir.path()).await.unwrap();
            store.create("projects", "My projects").await.unwrap();
        }

        // Reload and verify
        {
            let store = ContextStore::new(temp_dir.path()).await.unwrap();
            let cf = store.get("projects").unwrap();
            assert_eq!(cf.concept, "projects");
        }
    }
}
