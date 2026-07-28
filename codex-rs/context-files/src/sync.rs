//! Bidirectional synchronization for context files.
//!
//! The `SyncManager` handles synchronization between:
//! - File system changes
//! - UI edits
//! - AI-generated content
//! - Context file updates

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn};

use crate::context_file::ContextFile;
use crate::error::{ContextError, Result};
use crate::storage::ContextStore;

/// Synchronization event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncEvent {
    /// A file on disk was created.
    FileCreated { path: PathBuf },

    /// A file on disk was modified.
    FileModified { path: PathBuf },

    /// A file on disk was deleted.
    FileDeleted { path: PathBuf },

    /// Context file was updated by AI.
    ContextUpdated { concept: String },

    /// User edited content in the UI.
    UiEdit { concept: String, field: String },

    /// Conversation produced new information.
    ConversationUpdate {
        conversation_id: String,
        concepts: Vec<String>,
    },
}

/// The current sync state of a context file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// Last known sync time.
    pub last_sync: DateTime<Utc>,

    /// Hash of the content at last sync.
    pub content_hash: String,

    /// Whether there are pending changes.
    pub dirty: bool,

    /// Source of the last change.
    pub last_change_source: ChangeSource,
}

/// Source of a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeSource {
    /// Change came from file system.
    FileSystem,

    /// Change came from UI.
    Ui,

    /// Change came from AI.
    Ai,

    /// Change came from conversation.
    Conversation,

    /// Initial creation.
    Initial,
}

/// Conflict resolution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Keep the local version.
    KeepLocal,

    /// Keep the remote/incoming version.
    KeepIncoming,

    /// Merge changes (if possible).
    Merge,

    /// Ask the user to resolve.
    AskUser,
}

/// Manager for bidirectional synchronization.
///
/// The sync manager:
/// - Tracks sync state for each context file
/// - Processes incoming sync events
/// - Detects and resolves conflicts
/// - Maintains consistency across all data sources
pub struct SyncManager {
    /// Sync state for each concept.
    states: Arc<RwLock<HashMap<String, SyncState>>>,

    /// Event sender for notifying listeners.
    event_tx: mpsc::Sender<SyncEvent>,

    /// Event receiver for processing events.
    event_rx: Arc<RwLock<mpsc::Receiver<SyncEvent>>>,

    /// Conflict resolution strategy.
    conflict_strategy: ConflictResolution,
}

impl SyncManager {
    /// Create a new sync manager.
    pub fn new(conflict_strategy: ConflictResolution) -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);

        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
            conflict_strategy,
        }
    }

    /// Create a sync manager with default conflict resolution (merge).
    pub fn with_defaults() -> Self {
        Self::new(ConflictResolution::Merge)
    }

    /// Get a sender for sync events.
    pub fn event_sender(&self) -> mpsc::Sender<SyncEvent> {
        self.event_tx.clone()
    }

    /// Initialize sync state for a context file.
    pub async fn init_state(&self, cf: &ContextFile) {
        let state = SyncState {
            last_sync: Utc::now(),
            content_hash: Self::compute_hash(cf),
            dirty: false,
            last_change_source: ChangeSource::Initial,
        };

        self.states.write().await.insert(cf.concept.clone(), state);

        debug!("Initialized sync state for concept: {}", cf.concept);
    }

    /// Mark a context file as dirty.
    pub async fn mark_dirty(&self, concept: &str, source: ChangeSource) {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(concept) {
            state.dirty = true;
            state.last_change_source = source;
            debug!("Marked concept as dirty: {concept}");
        }
    }

    /// Check if a context file has pending changes.
    pub async fn is_dirty(&self, concept: &str) -> bool {
        self.states
            .read()
            .await
            .get(concept)
            .map_or(false, |s| s.dirty)
    }

    /// Process a sync event.
    pub async fn process_event(&self, event: SyncEvent, store: &mut ContextStore) -> Result<()> {
        match event {
            SyncEvent::FileCreated { path } => {
                info!("Processing file created: {}", path.display());
                self.handle_file_created(&path, store).await?;
            }
            SyncEvent::FileModified { path } => {
                debug!("Processing file modified: {}", path.display());
                self.handle_file_modified(&path, store).await?;
            }
            SyncEvent::FileDeleted { path } => {
                info!("Processing file deleted: {}", path.display());
                self.handle_file_deleted(&path, store).await?;
            }
            SyncEvent::ContextUpdated { concept } => {
                debug!("Processing context updated: {concept}");
                self.handle_context_updated(&concept, store).await?;
            }
            SyncEvent::UiEdit { concept, field } => {
                debug!("Processing UI edit: {concept}.{field}");
                self.handle_ui_edit(&concept, &field, store).await?;
            }
            SyncEvent::ConversationUpdate {
                conversation_id,
                concepts,
            } => {
                debug!(
                    "Processing conversation update: {conversation_id} -> {:?}",
                    concepts
                );
                self.handle_conversation_update(&conversation_id, &concepts, store)
                    .await?;
            }
        }

        Ok(())
    }

    /// Handle a newly created file.
    async fn handle_file_created(&self, path: &PathBuf, _store: &mut ContextStore) -> Result<()> {
        // TODO: Extract concept from file, create context file if needed
        info!(
            "File created handler not yet implemented: {}",
            path.display()
        );
        Ok(())
    }

    /// Handle a modified file.
    async fn handle_file_modified(&self, path: &PathBuf, _store: &mut ContextStore) -> Result<()> {
        // TODO: Update relevant context files with new information
        debug!(
            "File modified handler not yet implemented: {}",
            path.display()
        );
        Ok(())
    }

    /// Handle a deleted file.
    async fn handle_file_deleted(&self, path: &PathBuf, _store: &mut ContextStore) -> Result<()> {
        // TODO: Update context files to remove references
        info!(
            "File deleted handler not yet implemented: {}",
            path.display()
        );
        Ok(())
    }

    /// Handle AI-generated context update.
    async fn handle_context_updated(&self, concept: &str, store: &mut ContextStore) -> Result<()> {
        if let Some(cf) = store.get(concept) {
            let hash = Self::compute_hash(cf);
            let mut states = self.states.write().await;

            if let Some(state) = states.get_mut(concept) {
                // Check for conflicts
                if state.dirty && state.content_hash != hash {
                    warn!("Potential conflict detected for concept: {concept}");
                    self.resolve_conflict(concept, store).await?;
                }

                state.content_hash = hash;
                state.last_sync = Utc::now();
                state.dirty = false;
            }
        }

        Ok(())
    }

    /// Handle UI edit.
    async fn handle_ui_edit(
        &self,
        concept: &str,
        _field: &str,
        _store: &mut ContextStore,
    ) -> Result<()> {
        self.mark_dirty(concept, ChangeSource::Ui).await;
        Ok(())
    }

    /// Handle conversation update.
    async fn handle_conversation_update(
        &self,
        _conversation_id: &str,
        concepts: &[String],
        _store: &mut ContextStore,
    ) -> Result<()> {
        for concept in concepts {
            self.mark_dirty(concept, ChangeSource::Conversation).await;
        }
        Ok(())
    }

    /// Resolve a sync conflict.
    async fn resolve_conflict(&self, concept: &str, store: &mut ContextStore) -> Result<()> {
        match self.conflict_strategy {
            ConflictResolution::KeepLocal => {
                // Keep the current version in store
                debug!("Keeping local version for conflict: {concept}");
            }
            ConflictResolution::KeepIncoming => {
                // Would reload from external source
                debug!("Would keep incoming version for conflict: {concept}");
            }
            ConflictResolution::Merge => {
                // Attempt to merge changes
                debug!("Would attempt merge for conflict: {concept}");
                // TODO: Implement merge logic
            }
            ConflictResolution::AskUser => {
                // Would notify user to resolve
                return Err(ContextError::SyncConflict(format!(
                    "User resolution required for concept: {concept}"
                )));
            }
        }

        // Mark as synced after resolution
        if let Some(cf) = store.get(concept) {
            let hash = Self::compute_hash(cf);
            let mut states = self.states.write().await;
            if let Some(state) = states.get_mut(concept) {
                state.content_hash = hash;
                state.last_sync = Utc::now();
                state.dirty = false;
            }
        }

        Ok(())
    }

    /// Compute a hash of the context file content.
    fn compute_hash(cf: &ContextFile) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        cf.summary.hash(&mut hasher);
        cf.metadata.version.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get sync status for all concepts.
    pub async fn get_status(&self) -> HashMap<String, SyncState> {
        self.states.read().await.clone()
    }

    /// Force sync a specific concept.
    pub async fn force_sync(&self, concept: &str, store: &mut ContextStore) -> Result<()> {
        store.flush().await?;

        if let Some(cf) = store.get(concept) {
            let hash = Self::compute_hash(cf);
            let mut states = self.states.write().await;

            states.insert(
                concept.to_string(),
                SyncState {
                    last_sync: Utc::now(),
                    content_hash: hash,
                    dirty: false,
                    last_change_source: ChangeSource::FileSystem,
                },
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_manager_creation() {
        let manager = SyncManager::with_defaults();
        assert_eq!(manager.conflict_strategy, ConflictResolution::Merge);
    }

    #[tokio::test]
    async fn test_mark_dirty() {
        let manager = SyncManager::with_defaults();
        let cf = ContextFile::new("test", "Test concept");

        manager.init_state(&cf).await;
        assert!(!manager.is_dirty("test").await);

        manager.mark_dirty("test", ChangeSource::Ai).await;
        assert!(manager.is_dirty("test").await);
    }
}
