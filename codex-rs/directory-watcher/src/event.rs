//! File events from directory watching.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A file system event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    /// The kind of event.
    pub kind: FileEventKind,

    /// Path to the affected file or directory.
    pub path: PathBuf,

    /// When the event occurred.
    pub timestamp: DateTime<Utc>,

    /// Additional attributes.
    pub attributes: FileAttributes,
}

impl FileEvent {
    /// Create a new file event.
    pub fn new(kind: FileEventKind, path: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            path: path.into(),
            timestamp: Utc::now(),
            attributes: FileAttributes::default(),
        }
    }

    /// Add attributes to the event.
    pub fn with_attributes(mut self, attributes: FileAttributes) -> Self {
        self.attributes = attributes;
        self
    }

    /// Check if this is a file event (not directory).
    pub fn is_file(&self) -> bool {
        self.attributes.is_file
    }

    /// Check if this is a directory event.
    pub fn is_directory(&self) -> bool {
        self.attributes.is_directory
    }
}

/// Kind of file event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileEventKind {
    /// File was created.
    Created,

    /// File was modified.
    Modified,

    /// File was deleted.
    Deleted,

    /// File was renamed (old path).
    RenamedFrom,

    /// File was renamed (new path).
    RenamedTo,

    /// File metadata changed.
    MetadataChanged,

    /// Access time changed.
    Accessed,

    /// Unknown event type.
    Unknown,
}

impl From<notify::EventKind> for FileEventKind {
    fn from(kind: notify::EventKind) -> Self {
        match kind {
            notify::EventKind::Create(_) => Self::Created,
            notify::EventKind::Modify(modify_kind) => match modify_kind {
                notify::event::ModifyKind::Name(rename) => match rename {
                    notify::event::RenameMode::From => Self::RenamedFrom,
                    notify::event::RenameMode::To => Self::RenamedTo,
                    _ => Self::Modified,
                },
                notify::event::ModifyKind::Metadata(_) => Self::MetadataChanged,
                _ => Self::Modified,
            },
            notify::EventKind::Remove(_) => Self::Deleted,
            notify::EventKind::Access(_) => Self::Accessed,
            _ => Self::Unknown,
        }
    }
}

/// Additional file attributes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileAttributes {
    /// Whether the path is a file.
    pub is_file: bool,

    /// Whether the path is a directory.
    pub is_directory: bool,

    /// File size in bytes (if known).
    pub size: Option<u64>,

    /// File extension (if any).
    pub extension: Option<String>,

    /// MIME type (if known).
    pub mime_type: Option<String>,
}

impl FileAttributes {
    /// Create attributes from a path.
    pub fn from_path(path: &std::path::Path) -> Self {
        let metadata = path.metadata().ok();

        Self {
            is_file: path.is_file(),
            is_directory: path.is_dir(),
            size: metadata.as_ref().map(|m| m.len()),
            extension: path
                .extension()
                .and_then(|e| e.to_str())
                .map(String::from),
            mime_type: None,
        }
    }

    /// Infer MIME type from extension.
    pub fn with_mime_type(mut self) -> Self {
        if let Some(ref ext) = self.extension {
            self.mime_type = Some(mime_from_extension(ext));
        }
        self
    }
}

/// Get MIME type from file extension.
fn mime_from_extension(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        // Text
        "txt" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "csv" => "text/csv",
        "xml" => "text/xml",
        // Code
        "rs" => "text/x-rust",
        "js" | "mjs" => "text/javascript",
        "ts" | "tsx" => "text/typescript",
        "py" => "text/x-python",
        "rb" => "text/x-ruby",
        "go" => "text/x-go",
        "java" => "text/x-java",
        "c" | "h" => "text/x-c",
        "cpp" | "hpp" | "cc" => "text/x-c++",
        "cs" => "text/x-csharp",
        "swift" => "text/x-swift",
        "kt" | "kts" => "text/x-kotlin",
        // Data
        "json" => "application/json",
        "yaml" | "yml" => "application/x-yaml",
        "toml" => "application/toml",
        // Documents
        "pdf" => "application/pdf",
        "doc" | "docx" => "application/msword",
        "xls" | "xlsx" => "application/vnd.ms-excel",
        "ppt" | "pptx" => "application/vnd.ms-powerpoint",
        // Images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        // Audio/Video
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        // Archives
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        // Default
        _ => "application/octet-stream",
    }
    .to_string()
}

/// A batch of file events.
#[derive(Debug, Clone, Default)]
pub struct EventBatch {
    /// Events in this batch.
    pub events: Vec<FileEvent>,

    /// When the batch was created.
    pub created_at: Option<DateTime<Utc>>,
}

impl EventBatch {
    /// Create a new empty batch.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            created_at: Some(Utc::now()),
        }
    }

    /// Add an event to the batch.
    pub fn push(&mut self, event: FileEvent) {
        self.events.push(event);
    }

    /// Check if the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get the number of events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Deduplicate events (keep latest for each path).
    pub fn deduplicate(&mut self) {
        use std::collections::HashMap;

        let mut latest: HashMap<PathBuf, FileEvent> = HashMap::new();

        for event in self.events.drain(..) {
            latest.insert(event.path.clone(), event);
        }

        self.events = latest.into_values().collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_file_event_creation() {
        let event = FileEvent::new(FileEventKind::Created, "/test/file.txt");
        assert_eq!(event.kind, FileEventKind::Created);
        assert_eq!(event.path, Path::new("/test/file.txt"));
    }

    #[test]
    fn test_mime_type_inference() {
        let mut attrs = FileAttributes {
            extension: Some("rs".to_string()),
            ..Default::default()
        };
        attrs = attrs.with_mime_type();
        assert_eq!(attrs.mime_type, Some("text/x-rust".to_string()));
    }

    #[test]
    fn test_event_batch_dedup() {
        let mut batch = EventBatch::new();
        batch.push(FileEvent::new(FileEventKind::Modified, "/test/a.txt"));
        batch.push(FileEvent::new(FileEventKind::Modified, "/test/b.txt"));
        batch.push(FileEvent::new(FileEventKind::Deleted, "/test/a.txt"));

        batch.deduplicate();
        assert_eq!(batch.len(), 2);
    }
}
