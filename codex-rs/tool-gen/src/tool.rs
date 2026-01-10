//! Core tool types and definitions.
//!
//! A tool represents a reusable capability that the AI can create, use, and share.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::spec::ToolSpec;

/// A tool that can be executed by the AI.
///
/// Tools are self-contained units of functionality that can:
/// - Process inputs and produce outputs
/// - Be stored locally and shared with the community
/// - Evolve through versioning and improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Unique identifier for this tool.
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Description of what the tool does.
    pub description: String,

    /// Version string (semver).
    pub version: String,

    /// Who created this tool.
    pub author: String,

    /// Tool category.
    pub category: ToolCategory,

    /// The actual tool definition (implementation).
    pub definition: ToolDefinition,

    /// Metadata about the tool.
    pub metadata: ToolMetadata,

    /// Sharing configuration.
    pub sharing: SharingConfig,
}

impl Tool {
    /// Create a new tool with the given name and description.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        category: ToolCategory,
    ) -> Self {
        let name = name.into();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.clone(),
            description: description.into(),
            version: "0.1.0".to_string(),
            author: "user".to_string(),
            category,
            definition: ToolDefinition::default(),
            metadata: ToolMetadata::new(),
            sharing: SharingConfig::default(),
        }
    }

    /// Set the tool author.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    /// Set the tool version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the tool definition.
    pub fn with_definition(mut self, definition: ToolDefinition) -> Self {
        self.definition = definition;
        self
    }

    /// Mark the tool as public for sharing.
    pub fn make_public(mut self) -> Self {
        self.sharing.is_public = true;
        self
    }

    /// Add a tag to the tool.
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.metadata.tags.push(tag.into());
    }

    /// Record a usage of this tool.
    pub fn record_usage(&mut self) {
        self.metadata.usage_count += 1;
        self.metadata.last_used = Some(Utc::now());
    }

    /// Compute a signature hash for this tool.
    pub fn compute_signature(&self) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(self.name.as_bytes());
        hasher.update(self.version.as_bytes());
        hasher.update(format!("{:?}", self.definition).as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Category of tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    /// MCP server integration.
    McpServer,

    /// File type handler.
    FileHandler,

    /// Application integrator.
    AppIntegrator,

    /// Workflow automation.
    Workflow,

    /// General utility.
    Utility,

    /// Custom category.
    Custom,
}

impl std::fmt::Display for ToolCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::McpServer => write!(f, "MCP Server"),
            Self::FileHandler => write!(f, "File Handler"),
            Self::AppIntegrator => write!(f, "App Integrator"),
            Self::Workflow => write!(f, "Workflow"),
            Self::Utility => write!(f, "Utility"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

/// The actual implementation of a tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Type of tool implementation.
    pub tool_type: ToolType,

    /// The implementation code or configuration.
    pub implementation: String,

    /// Dependencies required by this tool.
    pub dependencies: Vec<String>,

    /// Tool specification (inputs/outputs).
    pub spec: ToolSpec,
}

impl ToolDefinition {
    /// Create a new tool definition with the given type and implementation.
    pub fn new(tool_type: ToolType, implementation: impl Into<String>) -> Self {
        Self {
            tool_type,
            implementation: implementation.into(),
            dependencies: Vec::new(),
            spec: ToolSpec::default(),
        }
    }

    /// Add a dependency.
    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    /// Set the tool specification.
    pub fn with_spec(mut self, spec: ToolSpec) -> Self {
        self.spec = spec;
        self
    }
}

/// Type of tool implementation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    /// A function definition (JSON Schema based).
    #[default]
    Function,

    /// An MCP server configuration.
    McpServer,

    /// A script (shell, Python, etc.).
    Script,

    /// A workflow definition (multiple steps).
    Workflow,
}

/// Metadata about a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// When the tool was created.
    pub created_at: DateTime<Utc>,

    /// When the tool was last updated.
    pub last_updated: DateTime<Utc>,

    /// When the tool was last used.
    pub last_used: Option<DateTime<Utc>>,

    /// Number of times the tool has been used.
    pub usage_count: u64,

    /// Community rating (if shared).
    pub rating: Option<f32>,

    /// Tags for categorization.
    pub tags: Vec<String>,

    /// Related tools that work well with this one.
    pub related_tools: Vec<String>,
}

impl ToolMetadata {
    /// Create new metadata with current timestamps.
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            last_updated: now,
            last_used: None,
            usage_count: 0,
            rating: None,
            tags: Vec::new(),
            related_tools: Vec::new(),
        }
    }
}

impl Default for ToolMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for tool sharing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SharingConfig {
    /// Whether the tool is shared publicly.
    pub is_public: bool,

    /// Unique share ID (assigned when published).
    pub share_id: Option<String>,

    /// Number of downloads (if shared).
    pub download_count: Option<u64>,

    /// Number of forks (if shared).
    pub forks: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_tool_creation() {
        let tool = Tool::new("test-tool", "A test tool", ToolCategory::Utility);
        assert_eq!(tool.name, "test-tool");
        assert_eq!(tool.version, "0.1.0");
        assert!(!tool.sharing.is_public);
    }

    #[test]
    fn test_tool_builder() {
        let tool = Tool::new("my-tool", "Description", ToolCategory::Workflow)
            .with_author("alice")
            .with_version("1.0.0")
            .make_public();

        assert_eq!(tool.author, "alice");
        assert_eq!(tool.version, "1.0.0");
        assert!(tool.sharing.is_public);
    }

    #[test]
    fn test_usage_recording() {
        let mut tool = Tool::new("counter", "Test", ToolCategory::Utility);
        assert_eq!(tool.metadata.usage_count, 0);

        tool.record_usage();
        tool.record_usage();
        assert_eq!(tool.metadata.usage_count, 2);
    }
}
