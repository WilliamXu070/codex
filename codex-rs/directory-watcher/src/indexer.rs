//! File indexing for directory scanning.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

use crate::config::DirectoryConfig;
use crate::error::Result;
use crate::event::FileAttributes;

/// An indexed file with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFile {
    /// Full path to the file.
    pub path: PathBuf,

    /// File attributes.
    pub attributes: FileAttributes,

    /// When the file was last modified.
    pub modified: Option<DateTime<Utc>>,

    /// When the file was indexed.
    pub indexed_at: DateTime<Utc>,

    /// Content hash (if computed).
    pub content_hash: Option<String>,
}

impl IndexedFile {
    /// Create an indexed file from a path.
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let metadata = path.metadata().ok();

        let modified = metadata
            .as_ref()
            .and_then(|m| m.modified().ok().map(|t| DateTime::<Utc>::from(t)));

        Self {
            attributes: FileAttributes::from_path(&path).with_mime_type(),
            path,
            modified,
            indexed_at: Utc::now(),
            content_hash: None,
        }
    }

    /// Compute content hash.
    pub async fn compute_hash(&mut self) -> Result<()> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        if self.attributes.is_file {
            let content = fs::read(&self.path).await?;
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            self.content_hash = Some(format!("{:x}", hasher.finish()));
        }

        Ok(())
    }

    /// Check if the file has been modified since indexing.
    pub fn is_stale(&self) -> bool {
        if let Some(modified) = self.modified {
            modified > self.indexed_at
        } else {
            false
        }
    }
}

/// Indexes files in a directory.
pub struct FileIndexer {
    /// Index of files by path.
    files: HashMap<PathBuf, IndexedFile>,

    /// Configuration.
    config: DirectoryConfig,
}

impl FileIndexer {
    /// Create a new file indexer.
    pub fn new(config: DirectoryConfig) -> Self {
        Self {
            files: HashMap::new(),
            config,
        }
    }

    /// Scan the directory and index all files.
    pub fn scan(&mut self) -> Result<IndexResult> {
        let start = std::time::Instant::now();
        let mut new_files = 0;
        let mut updated_files = 0;
        let mut removed_files = 0;

        let mut current_paths: std::collections::HashSet<PathBuf> =
            std::collections::HashSet::new();

        // Walk the directory
        let walker = WalkDir::new(&self.config.path)
            .follow_links(self.config.follow_symlinks)
            .max_depth(self.config.max_depth.unwrap_or(usize::MAX));

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path().to_path_buf();

            // Skip excluded paths
            if self.config.should_exclude(&path) {
                continue;
            }

            // Skip directories (we only index files)
            if path.is_dir() {
                continue;
            }

            current_paths.insert(path.clone());

            if let Some(existing) = self.files.get(&path) {
                // Check if modified
                let indexed = IndexedFile::from_path(&path);
                if indexed.modified != existing.modified {
                    self.files.insert(path, indexed);
                    updated_files += 1;
                }
            } else {
                // New file
                self.files
                    .insert(path.clone(), IndexedFile::from_path(&path));
                new_files += 1;
            }
        }

        // Find removed files
        let old_paths: Vec<PathBuf> = self.files.keys().cloned().collect();
        for path in old_paths {
            if !current_paths.contains(&path) {
                self.files.remove(&path);
                removed_files += 1;
            }
        }

        let duration = start.elapsed();
        info!(
            "Indexed {} files in {:?} (new: {}, updated: {}, removed: {})",
            self.files.len(),
            duration,
            new_files,
            updated_files,
            removed_files
        );

        Ok(IndexResult {
            total_files: self.files.len(),
            new_files,
            updated_files,
            removed_files,
            duration_ms: duration.as_millis() as u64,
        })
    }

    /// Get an indexed file by path.
    pub fn get(&self, path: &Path) -> Option<&IndexedFile> {
        self.files.get(path)
    }

    /// Get all indexed files.
    pub fn files(&self) -> impl Iterator<Item = &IndexedFile> {
        self.files.values()
    }

    /// Get files by extension.
    pub fn by_extension(&self, ext: &str) -> Vec<&IndexedFile> {
        self.files
            .values()
            .filter(|f| f.attributes.extension.as_ref().map_or(false, |e| e == ext))
            .collect()
    }

    /// Get files by MIME type.
    pub fn by_mime_type(&self, mime: &str) -> Vec<&IndexedFile> {
        self.files
            .values()
            .filter(|f| {
                f.attributes
                    .mime_type
                    .as_ref()
                    .map_or(false, |m| m.starts_with(mime))
            })
            .collect()
    }

    /// Search files by path pattern.
    pub fn search(&self, pattern: &str) -> Vec<&IndexedFile> {
        let pattern_lower = pattern.to_lowercase();
        self.files
            .values()
            .filter(|f| {
                f.path
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&pattern_lower)
            })
            .collect()
    }

    /// Get stale files that need re-indexing.
    pub fn stale_files(&self) -> Vec<&IndexedFile> {
        self.files.values().filter(|f| f.is_stale()).collect()
    }

    /// Get statistics about the index.
    pub fn stats(&self) -> IndexStats {
        let mut by_extension: HashMap<String, usize> = HashMap::new();
        let mut total_size: u64 = 0;

        for file in self.files.values() {
            if let Some(ref ext) = file.attributes.extension {
                *by_extension.entry(ext.clone()).or_insert(0) += 1;
            }
            if let Some(size) = file.attributes.size {
                total_size += size;
            }
        }

        IndexStats {
            total_files: self.files.len(),
            total_size_bytes: total_size,
            by_extension,
        }
    }

    /// Clear the index.
    pub fn clear(&mut self) {
        self.files.clear();
        debug!("Cleared file index");
    }
}

/// Result of an indexing operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    /// Total files in the index.
    pub total_files: usize,

    /// Number of new files.
    pub new_files: usize,

    /// Number of updated files.
    pub updated_files: usize,

    /// Number of removed files.
    pub removed_files: usize,

    /// Time taken in milliseconds.
    pub duration_ms: u64,
}

/// Statistics about the file index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Total number of files.
    pub total_files: usize,

    /// Total size of all files.
    pub total_size_bytes: u64,

    /// Files by extension.
    pub by_extension: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_indexer_scan() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        let mut f1 = File::create(temp_dir.path().join("test.txt")).unwrap();
        writeln!(f1, "Hello").unwrap();

        let mut f2 = File::create(temp_dir.path().join("code.rs")).unwrap();
        writeln!(f2, "fn main() {{}}").unwrap();

        let config = DirectoryConfig::new(temp_dir.path());
        let mut indexer = FileIndexer::new(config);

        let result = indexer.scan().unwrap();
        assert_eq!(result.total_files, 2);
        assert_eq!(result.new_files, 2);
    }

    #[test]
    fn test_indexer_by_extension() {
        let temp_dir = TempDir::new().unwrap();

        File::create(temp_dir.path().join("a.txt")).unwrap();
        File::create(temp_dir.path().join("b.txt")).unwrap();
        File::create(temp_dir.path().join("c.rs")).unwrap();

        let config = DirectoryConfig::new(temp_dir.path());
        let mut indexer = FileIndexer::new(config);
        indexer.scan().unwrap();

        let txt_files = indexer.by_extension("txt");
        assert_eq!(txt_files.len(), 2);

        let rs_files = indexer.by_extension("rs");
        assert_eq!(rs_files.len(), 1);
    }
}
