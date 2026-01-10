//! Configuration types for directory watching.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration for a watched directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryConfig {
    /// Path to the directory.
    pub path: PathBuf,

    /// Whether watching is enabled.
    pub enabled: bool,

    /// How to watch the directory.
    pub watch_mode: WatchMode,

    /// Patterns to exclude (glob patterns).
    pub exclude_patterns: Vec<String>,

    /// Priority for indexing (higher = more frequent).
    pub priority: u32,

    /// Maximum depth to recurse (None = unlimited).
    pub max_depth: Option<usize>,

    /// Whether to follow symbolic links.
    pub follow_symlinks: bool,
}

impl DirectoryConfig {
    /// Create a new directory config.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            enabled: true,
            watch_mode: WatchMode::Realtime,
            exclude_patterns: Self::default_excludes(),
            priority: 5,
            max_depth: None,
            follow_symlinks: false,
        }
    }

    /// Set the watch mode.
    pub fn with_mode(mut self, mode: WatchMode) -> Self {
        self.watch_mode = mode;
        self
    }

    /// Add an exclude pattern.
    pub fn exclude(mut self, pattern: impl Into<String>) -> Self {
        self.exclude_patterns.push(pattern.into());
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set the maximum depth.
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Enable following symbolic links.
    pub fn follow_symlinks(mut self) -> Self {
        self.follow_symlinks = true;
        self
    }

    /// Disable the directory.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Get default exclude patterns.
    fn default_excludes() -> Vec<String> {
        vec![
            // Version control
            "**/.git/**".to_string(),
            "**/.svn/**".to_string(),
            "**/.hg/**".to_string(),
            // Dependencies
            "**/node_modules/**".to_string(),
            "**/target/**".to_string(),
            "**/vendor/**".to_string(),
            "**/.venv/**".to_string(),
            "**/venv/**".to_string(),
            // Build artifacts
            "**/build/**".to_string(),
            "**/dist/**".to_string(),
            "**/__pycache__/**".to_string(),
            "**/*.pyc".to_string(),
            // IDE/Editor
            "**/.idea/**".to_string(),
            "**/.vscode/**".to_string(),
            "**/*.swp".to_string(),
            "**/*~".to_string(),
            // System files
            "**/.DS_Store".to_string(),
            "**/Thumbs.db".to_string(),
            // Temporary files
            "**/tmp/**".to_string(),
            "**/temp/**".to_string(),
            "**/*.tmp".to_string(),
            "**/*.temp".to_string(),
        ]
    }

    /// Check if a path should be excluded.
    pub fn should_exclude(&self, path: &std::path::Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.exclude_patterns {
            if let Ok(glob) = glob::Pattern::new(pattern) {
                if glob.matches(&path_str) {
                    return true;
                }
            }
        }

        false
    }
}

/// How to watch a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchMode {
    /// Watch for changes in real-time.
    Realtime,

    /// Periodic scheduled scans.
    Scheduled,

    /// Manual indexing only.
    Manual,
}

impl Default for WatchMode {
    fn default() -> Self {
        Self::Realtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::Path;

    #[test]
    fn test_directory_config_creation() {
        let config = DirectoryConfig::new("/home/user/documents")
            .with_mode(WatchMode::Scheduled)
            .with_priority(10);

        assert_eq!(config.path, Path::new("/home/user/documents"));
        assert_eq!(config.watch_mode, WatchMode::Scheduled);
        assert_eq!(config.priority, 10);
    }

    #[test]
    fn test_exclude_patterns() {
        let config = DirectoryConfig::new("/test");

        assert!(config.should_exclude(Path::new("/test/.git/config")));
        assert!(config.should_exclude(Path::new("/test/node_modules/package/index.js")));
        assert!(!config.should_exclude(Path::new("/test/src/main.rs")));
    }
}
