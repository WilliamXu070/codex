//! # Tool Generation Framework
//!
//! This crate implements the tool generation and community sharing system for Codex.
//! It enables the AI to:
//!
//! - **Create Tools**: Generate custom tools during task execution
//! - **Store Tools**: Persist tools locally for reuse
//! - **Share Tools**: Publish tools to the community repository
//! - **Discover Tools**: Find and install tools from the community
//!
//! ## Tool Categories
//!
//! 1. **MCP Servers**: Connect to external applications
//! 2. **File Handlers**: Read/write specialized file formats
//! 3. **App Integrators**: Automate workflows across apps
//! 4. **Workflows**: Reusable automation scripts
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Tool Generation System                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ToolGenerator ──► Tool ──► ToolStore                          │
//! │       │              │           │                              │
//! │       ▼              ▼           ▼                              │
//! │  ToolExecutor    ToolSpec   ToolRegistry ◄── CommunityHub      │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod community;
pub mod error;
pub mod executor;
pub mod generator;
pub mod registry;
pub mod spec;
pub mod storage;
pub mod tool;

pub use community::{CommunityHub, SharedTool, ToolRating};
pub use error::{Result, ToolError};
pub use executor::{ExecutionContext, ExecutionResult, ToolExecutor};
pub use generator::{GenerationRequest, ToolGenerator};
pub use registry::ToolRegistry;
pub use spec::{ToolInput, ToolOutput, ToolSpec};
pub use storage::ToolStore;
pub use tool::{Tool, ToolCategory, ToolDefinition, ToolMetadata};
