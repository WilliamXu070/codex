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

pub use community::CommunityHub;
pub use community::SharedTool;
pub use community::ToolRating;
pub use error::Result;
pub use error::ToolError;
pub use executor::ExecutionContext;
pub use executor::ExecutionResult;
pub use executor::ToolExecutor;
pub use generator::GenerationRequest;
pub use generator::ToolGenerator;
pub use registry::ToolRegistry;
pub use spec::ToolInput;
pub use spec::ToolOutput;
pub use spec::ToolSpec;
pub use storage::ToolStore;
pub use tool::Tool;
pub use tool::ToolCategory;
pub use tool::ToolDefinition;
pub use tool::ToolMetadata;
