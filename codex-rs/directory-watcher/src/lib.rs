//! # Directory Watcher
//!
//! This crate provides file system monitoring for the Codex context system.
//! It watches enabled directories and notifies the context system of changes.
//!
//! ## Features
//!
//! - **Real-time Watching**: Monitor directories for file changes
//! - **Scheduled Indexing**: Periodic full-directory scans
//! - **Exclusion Patterns**: Filter out unwanted files
//! - **Event Batching**: Efficiently handle rapid changes
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Directory Watcher                            │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  DirectoryConfig ──► Watcher ──► FileEvent                     │
//! │       │                │              │                         │
//! │       ▼                ▼              ▼                         │
//! │  ExcludePatterns   EventBatcher   EventHandler                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod config;
pub mod error;
pub mod event;
pub mod indexer;
pub mod watcher;

pub use config::DirectoryConfig;
pub use config::WatchMode;
pub use error::Result;
pub use error::WatcherError;
pub use event::FileEvent;
pub use event::FileEventKind;
pub use indexer::FileIndexer;
pub use indexer::IndexedFile;
pub use watcher::DirectoryWatcher;
