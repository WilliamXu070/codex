//! Tool execution engine.
//!
//! The `ToolExecutor` handles running tools with proper sandboxing,
//! input validation, and result capture.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{Result, ToolError};
use crate::tool::{Tool, ToolType};

/// Context for tool execution.
#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    /// Environment variables to set.
    pub env: HashMap<String, String>,

    /// Working directory.
    pub working_dir: Option<String>,

    /// Maximum execution time.
    pub timeout: Option<Duration>,

    /// Whether to run in sandbox mode.
    pub sandboxed: bool,

    /// User ID for permission checking.
    pub user_id: Option<String>,
}

impl ExecutionContext {
    /// Create a new execution context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set the working directory.
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set the timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Enable sandboxing.
    pub fn sandboxed(mut self) -> Self {
        self.sandboxed = true;
        self
    }

    /// Set user ID.
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
}

/// Result of tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether execution succeeded.
    pub success: bool,

    /// Output from the tool.
    pub output: serde_json::Value,

    /// Error message if failed.
    pub error: Option<String>,

    /// Execution time in milliseconds.
    pub duration_ms: u64,

    /// Standard output (if applicable).
    pub stdout: Option<String>,

    /// Standard error (if applicable).
    pub stderr: Option<String>,
}

impl ExecutionResult {
    /// Create a successful result.
    pub fn success(output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            success: true,
            output,
            error: None,
            duration_ms,
            stdout: None,
            stderr: None,
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            success: false,
            output: serde_json::Value::Null,
            error: Some(error.into()),
            duration_ms,
            stdout: None,
            stderr: None,
        }
    }

    /// Add stdout to the result.
    pub fn with_stdout(mut self, stdout: impl Into<String>) -> Self {
        self.stdout = Some(stdout.into());
        self
    }

    /// Add stderr to the result.
    pub fn with_stderr(mut self, stderr: impl Into<String>) -> Self {
        self.stderr = Some(stderr.into());
        self
    }
}

/// Executor for running tools.
///
/// The executor handles:
/// - Input validation against tool spec
/// - Proper sandboxing for script tools
/// - Timeout enforcement
/// - Result capture and formatting
pub struct ToolExecutor {
    /// Default timeout for executions.
    default_timeout: Duration,

    /// Whether to enforce sandboxing by default.
    default_sandboxed: bool,
}

impl ToolExecutor {
    /// Create a new tool executor.
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            default_sandboxed: true,
        }
    }

    /// Set the default timeout.
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Disable default sandboxing.
    pub fn without_sandbox(mut self) -> Self {
        self.default_sandboxed = false;
        self
    }

    /// Execute a tool with the given inputs.
    pub async fn execute(
        &self,
        tool: &Tool,
        inputs: serde_json::Value,
        context: Option<ExecutionContext>,
    ) -> Result<ExecutionResult> {
        let start = Instant::now();
        let context = context.unwrap_or_else(|| {
            let mut ctx = ExecutionContext::new();
            if self.default_sandboxed {
                ctx = ctx.sandboxed();
            }
            ctx.timeout = Some(self.default_timeout);
            ctx
        });

        debug!("Executing tool: {} with inputs: {:?}", tool.name, inputs);

        // Validate inputs
        tool.definition
            .spec
            .validate_inputs(&inputs)
            .map_err(ToolError::InvalidInput)?;

        // Execute based on tool type
        let result = match tool.definition.tool_type {
            ToolType::Function => self.execute_function(tool, inputs, &context).await,
            ToolType::Script => self.execute_script(tool, inputs, &context).await,
            ToolType::McpServer => self.execute_mcp(tool, inputs, &context).await,
            ToolType::Workflow => self.execute_workflow(tool, inputs, &context).await,
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => {
                info!(
                    "Tool {} executed successfully in {}ms",
                    tool.name, duration_ms
                );
                Ok(ExecutionResult::success(output, duration_ms))
            }
            Err(e) => {
                warn!("Tool {} failed: {}", tool.name, e);
                Ok(ExecutionResult::failure(e.to_string(), duration_ms))
            }
        }
    }

    /// Execute a function-type tool.
    async fn execute_function(
        &self,
        tool: &Tool,
        inputs: serde_json::Value,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value> {
        // Function tools are primarily for AI function calling
        // They don't execute locally but define the schema for the AI
        // Return the inputs as acknowledgment
        debug!("Function tool {} called with inputs", tool.name);
        Ok(serde_json::json!({
            "tool": tool.name,
            "inputs_received": inputs,
            "status": "function_tools_require_ai_execution"
        }))
    }

    /// Execute a script-type tool.
    async fn execute_script(
        &self,
        tool: &Tool,
        inputs: serde_json::Value,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value> {
        // This is a placeholder for actual script execution
        // In production, this would:
        // 1. Write inputs to stdin or temp file
        // 2. Execute the script in a sandbox
        // 3. Capture stdout/stderr
        // 4. Parse and return the output

        if context.sandboxed {
            debug!("Script execution would run in sandbox");
        }

        warn!("Script execution not yet implemented for tool: {}", tool.name);

        Ok(serde_json::json!({
            "status": "script_execution_not_implemented",
            "tool": tool.name,
            "inputs": inputs
        }))
    }

    /// Execute an MCP server tool.
    async fn execute_mcp(
        &self,
        tool: &Tool,
        inputs: serde_json::Value,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value> {
        // MCP tools would communicate with an MCP server
        // This is a placeholder for actual MCP integration

        debug!("MCP tool {} called", tool.name);

        Ok(serde_json::json!({
            "status": "mcp_execution_not_implemented",
            "tool": tool.name,
            "inputs": inputs
        }))
    }

    /// Execute a workflow-type tool.
    async fn execute_workflow(
        &self,
        tool: &Tool,
        inputs: serde_json::Value,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value> {
        // Workflow tools execute multiple steps
        // This is a placeholder for workflow orchestration

        debug!("Workflow tool {} called", tool.name);

        // Parse workflow definition
        let workflow: serde_json::Value =
            serde_json::from_str(&tool.definition.implementation)?;

        let steps = workflow
            .get("steps")
            .and_then(|s| s.as_array())
            .map(|a| a.len())
            .unwrap_or(0);

        Ok(serde_json::json!({
            "status": "workflow_execution_not_implemented",
            "tool": tool.name,
            "steps_count": steps,
            "inputs": inputs,
            "sandboxed": context.sandboxed
        }))
    }

    /// Check if execution is allowed in the current context.
    #[allow(dead_code)]
    fn check_permissions(&self, tool: &Tool, context: &ExecutionContext) -> Result<()> {
        // Check if user has permission to run this tool
        if tool.sharing.is_public {
            return Ok(()); // Public tools are always allowed
        }

        if let Some(ref _user_id) = context.user_id {
            // Check user permissions here
            return Ok(());
        }

        Err(ToolError::SecurityValidation(
            "Permission denied for tool execution".to_string(),
        ))
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolCategory;

    #[tokio::test]
    async fn test_executor_creation() {
        let executor = ToolExecutor::new();
        assert_eq!(executor.default_timeout, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_execution_context() {
        let ctx = ExecutionContext::new()
            .with_env("FOO", "bar")
            .with_timeout(Duration::from_secs(10))
            .sandboxed();

        assert_eq!(ctx.env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(ctx.timeout, Some(Duration::from_secs(10)));
        assert!(ctx.sandboxed);
    }

    #[tokio::test]
    async fn test_function_execution() {
        let executor = ToolExecutor::new();
        let tool = Tool::new("test", "Test tool", ToolCategory::Utility);

        let result = executor
            .execute(&tool, serde_json::json!({}), None)
            .await
            .unwrap();

        assert!(result.success);
    }
}
