//! Directory watcher implementation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

use crate::config::{DirectoryConfig, WatchMode};
use crate::error::{Result, WatcherError};
use crate::event::{FileAttributes, FileEvent, FileEventKind};

/// Directory watcher that monitors file system changes.
pub struct DirectoryWatcher {
    /// Watched directories.
    configs: Arc<RwLock<HashMap<PathBuf, DirectoryConfig>>>,

    /// Internal notify watcher.
    watcher: Option<RecommendedWatcher>,

    /// Event sender.
    event_tx: mpsc::Sender<FileEvent>,

    /// Event receiver (for consumers).
    event_rx: Arc<RwLock<mpsc::Receiver<FileEvent>>>,

    /// Whether the watcher is running.
    running: Arc<RwLock<bool>>,
}

impl DirectoryWatcher {
    /// Create a new directory watcher.
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel(1000);

        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            watcher: None,
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Add a directory to watch.
    pub async fn add(&mut self, config: DirectoryConfig) -> Result<()> {
        let path = config.path.clone();

        // Validate path exists
        if !path.exists() {
            return Err(WatcherError::DirectoryNotFound(path.display().to_string()));
        }

        if !path.is_dir() {
            return Err(WatcherError::Config(format!(
                "Path is not a directory: {}",
                path.display()
            )));
        }

        // Check if already watching
        {
            let configs = self.configs.read().await;
            if configs.contains_key(&path) {
                return Err(WatcherError::AlreadyWatching(path.display().to_string()));
            }
        }

        info!("Adding directory to watch: {}", path.display());
        self.configs.write().await.insert(path, config);

        Ok(())
    }

    /// Remove a directory from watching.
    pub async fn remove(&mut self, path: &Path) -> Result<()> {
        let mut configs = self.configs.write().await;

        if configs.remove(path).is_none() {
            return Err(WatcherError::DirectoryNotFound(path.display().to_string()));
        }

        info!("Removed directory from watch: {}", path.display());
        Ok(())
    }

    /// Start watching all configured directories.
    pub async fn start(&mut self) -> Result<()> {
        if *self.running.read().await {
            return Ok(()); // Already running
        }

        let event_tx = self.event_tx.clone();
        let configs = self.configs.clone();

        // Create the notify watcher
        let watcher = notify::recommended_watcher(
            move |res: std::result::Result<notify::Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        let kind = FileEventKind::from(event.kind);

                        for path in event.paths {
                            // Check if path should be excluded
                            let should_process = {
                                let configs_guard = configs.blocking_read();
                                !configs_guard.values().any(|c| c.should_exclude(&path))
                            };

                            if should_process {
                                let file_event = FileEvent::new(kind, &path).with_attributes(
                                    FileAttributes::from_path(&path).with_mime_type(),
                                );

                                if let Err(e) = event_tx.blocking_send(file_event) {
                                    error!("Failed to send file event: {e}");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Watch error: {e}");
                    }
                }
            },
        )?;

        self.watcher = Some(watcher);

        // Add all realtime directories to the watcher
        let configs = self.configs.read().await;
        for (path, config) in configs.iter() {
            if config.enabled && config.watch_mode == WatchMode::Realtime {
                if let Some(ref mut w) = self.watcher {
                    let mode = if config.max_depth == Some(0) {
                        RecursiveMode::NonRecursive
                    } else {
                        RecursiveMode::Recursive
                    };

                    match w.watch(path, mode) {
                        Ok(_) => debug!("Started watching: {}", path.display()),
                        Err(e) => warn!("Failed to watch {}: {e}", path.display()),
                    }
                }
            }
        }

        *self.running.write().await = true;
        info!("Directory watcher started");

        Ok(())
    }

    /// Stop watching all directories.
    pub async fn stop(&mut self) {
        if let Some(ref mut watcher) = self.watcher {
            let configs = self.configs.read().await;
            for path in configs.keys() {
                let _ = watcher.unwatch(path);
            }
        }

        self.watcher = None;
        *self.running.write().await = false;
        info!("Directory watcher stopped");
    }

    /// Check if the watcher is running.
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get the event receiver.
    pub fn events(&self) -> &Arc<RwLock<mpsc::Receiver<FileEvent>>> {
        &self.event_rx
    }

    /// Get configured directories.
    pub async fn directories(&self) -> Vec<DirectoryConfig> {
        self.configs.read().await.values().cloned().collect()
    }

    /// Update a directory configuration.
    pub async fn update(&mut self, config: DirectoryConfig) -> Result<()> {
        let path = config.path.clone();
        let was_running = self.is_running().await;

        // Check if already exists
        {
            let configs = self.configs.read().await;
            if !configs.contains_key(&path) {
                return Err(WatcherError::DirectoryNotFound(path.display().to_string()));
            }
        }

        // Stop, update, restart if needed
        if was_running {
            // Unwatch the old config
            if let Some(ref mut w) = self.watcher {
                let _ = w.unwatch(&path);
            }
        }

        // Update config
        self.configs
            .write()
            .await
            .insert(path.clone(), config.clone());

        if was_running && config.enabled && config.watch_mode == WatchMode::Realtime {
            // Re-add to watcher
            if let Some(ref mut w) = self.watcher {
                let mode = if config.max_depth == Some(0) {
                    RecursiveMode::NonRecursive
                } else {
                    RecursiveMode::Recursive
                };
                w.watch(&path, mode)?;
            }
        }

        info!("Updated directory config: {}", path.display());
        Ok(())
    }

    /// Get statistics about watched directories.
    pub async fn stats(&self) -> WatcherStats {
        let configs = self.configs.read().await;

        WatcherStats {
            total_directories: configs.len(),
            enabled_directories: configs.values().filter(|c| c.enabled).count(),
            realtime_watches: configs
                .values()
                .filter(|c| c.enabled && c.watch_mode == WatchMode::Realtime)
                .count(),
            scheduled_watches: configs
                .values()
                .filter(|c| c.enabled && c.watch_mode == WatchMode::Scheduled)
                .count(),
        }
    }
}

impl Default for DirectoryWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the directory watcher.
#[derive(Debug, Clone)]
pub struct WatcherStats {
    /// Total configured directories.
    pub total_directories: usize,

    /// Number of enabled directories.
    pub enabled_directories: usize,

    /// Number of realtime watches.
    pub realtime_watches: usize,

    /// Number of scheduled watches.
    pub scheduled_watches: usize,
}

/// A scheduled watcher that periodically scans directories.
pub struct ScheduledWatcher {
    /// Directory configurations.
    configs: Vec<DirectoryConfig>,

    /// Scan interval.
    interval: Duration,

    /// Event sender.
    event_tx: mpsc::Sender<FileEvent>,

    /// Whether the watcher is running.
    running: Arc<RwLock<bool>>,
}

impl ScheduledWatcher {
    /// Create a new scheduled watcher.
    pub fn new(interval: Duration) -> (Self, mpsc::Receiver<FileEvent>) {
        let (event_tx, event_rx) = mpsc::channel(1000);

        let watcher = Self {
            configs: Vec::new(),
            interval,
            event_tx,
            running: Arc::new(RwLock::new(false)),
        };

        (watcher, event_rx)
    }

    /// Add a directory to scan.
    pub fn add(&mut self, config: DirectoryConfig) {
        if config.watch_mode == WatchMode::Scheduled {
            self.configs.push(config);
        }
    }

    /// Start the scheduled scans.
    pub async fn start(&self) {
        *self.running.write().await = true;

        let configs = self.configs.clone();
        let interval = self.interval;
        let event_tx = self.event_tx.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            while *running.read().await {
                for config in &configs {
                    if config.enabled {
                        debug!("Scanning directory: {}", config.path.display());
                        // Scan would happen here
                        // For now, just log
                    }
                }

                tokio::time::sleep(interval).await;
            }
        });
    }

    /// Stop the scheduled scans.
    pub async fn stop(&self) {
        *self.running.write().await = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_watcher_creation() {
        let watcher = DirectoryWatcher::new();
        assert!(!watcher.is_running().await);
    }

    #[tokio::test]
    async fn test_add_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = DirectoryWatcher::new();

        let config = DirectoryConfig::new(temp_dir.path());
        watcher.add(config).await.unwrap();

        let dirs = watcher.directories().await;
        assert_eq!(dirs.len(), 1);
    }

    #[tokio::test]
    async fn test_add_nonexistent_directory() {
        let mut watcher = DirectoryWatcher::new();
        let config = DirectoryConfig::new("/nonexistent/path/12345");

        let result = watcher.add(config).await;
        assert!(result.is_err());
    }
}
