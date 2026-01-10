//! Tool generation from AI requests.
//!
//! The `ToolGenerator` creates new tools based on AI-identified needs
//! and user requests.

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::{Result, ToolError};
use crate::spec::{DataType, ToolInput, ToolOutput, ToolSpec};
use crate::tool::{Tool, ToolCategory, ToolDefinition, ToolType};

/// A request to generate a new tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    /// Name for the new tool.
    pub name: String,

    /// Description of what the tool should do.
    pub description: String,

    /// Category of tool to create.
    pub category: ToolCategory,

    /// Implementation type.
    pub tool_type: ToolType,

    /// Example inputs (used to infer schema).
    pub example_inputs: Option<serde_json::Value>,

    /// Example outputs (used to infer schema).
    pub example_outputs: Option<serde_json::Value>,

    /// Additional context for generation.
    pub context: Option<String>,

    /// Tags to apply to the tool.
    pub tags: Vec<String>,
}

impl GenerationRequest {
    /// Create a new generation request.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        category: ToolCategory,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            category,
            tool_type: ToolType::Function,
            example_inputs: None,
            example_outputs: None,
            context: None,
            tags: Vec::new(),
        }
    }

    /// Set the tool type.
    pub fn with_type(mut self, tool_type: ToolType) -> Self {
        self.tool_type = tool_type;
        self
    }

    /// Add example inputs.
    pub fn with_example_inputs(mut self, inputs: serde_json::Value) -> Self {
        self.example_inputs = Some(inputs);
        self
    }

    /// Add example outputs.
    pub fn with_example_outputs(mut self, outputs: serde_json::Value) -> Self {
        self.example_outputs = Some(outputs);
        self
    }

    /// Add context for generation.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// The result of tool generation.
#[derive(Debug, Clone)]
pub struct GenerationResult {
    /// The generated tool.
    pub tool: Tool,

    /// Warnings or suggestions from generation.
    pub warnings: Vec<String>,

    /// Whether the tool needs review before use.
    pub needs_review: bool,
}

/// Generator for creating new tools.
///
/// The generator:
/// - Infers tool specifications from examples
/// - Generates implementation scaffolds
/// - Validates tool definitions
pub struct ToolGenerator {
    /// Whether to require review for generated tools.
    require_review: bool,

    /// Default author for generated tools.
    default_author: String,
}

impl ToolGenerator {
    /// Create a new tool generator.
    pub fn new() -> Self {
        Self {
            require_review: true,
            default_author: "ai".to_string(),
        }
    }

    /// Set the default author.
    pub fn with_default_author(mut self, author: impl Into<String>) -> Self {
        self.default_author = author.into();
        self
    }

    /// Disable review requirement.
    pub fn without_review(mut self) -> Self {
        self.require_review = false;
        self
    }

    /// Generate a tool from a request.
    pub fn generate(&self, request: GenerationRequest) -> Result<GenerationResult> {
        // Validate request
        self.validate_request(&request)?;

        // Infer specification from examples
        let spec = self.infer_spec(&request)?;

        // Generate implementation
        let implementation = self.generate_implementation(&request, &spec)?;

        // Create the tool
        let mut tool = Tool::new(&request.name, &request.description, request.category)
            .with_author(&self.default_author)
            .with_definition(
                ToolDefinition::new(request.tool_type, implementation).with_spec(spec),
            );

        // Add tags
        for tag in request.tags {
            tool.add_tag(tag);
        }

        let mut warnings = Vec::new();

        // Check for potential issues
        if tool.definition.spec.inputs.is_empty() {
            warnings.push("Tool has no inputs defined".to_string());
        }

        if tool.definition.spec.outputs.is_empty() {
            warnings.push("Tool has no outputs defined".to_string());
        }

        info!("Generated tool: {}", tool.name);

        Ok(GenerationResult {
            tool,
            warnings,
            needs_review: self.require_review,
        })
    }

    /// Validate a generation request.
    fn validate_request(&self, request: &GenerationRequest) -> Result<()> {
        if request.name.is_empty() {
            return Err(ToolError::InvalidDefinition("Name cannot be empty".to_string()));
        }

        if request.description.is_empty() {
            return Err(ToolError::InvalidDefinition(
                "Description cannot be empty".to_string(),
            ));
        }

        // Validate name format
        if !request
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ToolError::InvalidDefinition(
                "Name can only contain alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            ));
        }

        Ok(())
    }

    /// Infer tool specification from examples.
    fn infer_spec(&self, request: &GenerationRequest) -> Result<ToolSpec> {
        let mut spec = ToolSpec::new();

        // Infer inputs from example
        if let Some(ref example) = request.example_inputs {
            if let Some(obj) = example.as_object() {
                for (key, value) in obj {
                    let data_type = Self::infer_type(value);
                    spec = spec.with_input(ToolInput::required(
                        key,
                        data_type,
                        format!("Input parameter: {key}"),
                    ));
                }
            }
        }

        // Infer outputs from example
        if let Some(ref example) = request.example_outputs {
            if let Some(obj) = example.as_object() {
                for (key, value) in obj {
                    let data_type = Self::infer_type(value);
                    spec = spec.with_output(ToolOutput::new(
                        key,
                        data_type,
                        format!("Output parameter: {key}"),
                    ));
                }
            }
        }

        Ok(spec)
    }

    /// Infer data type from a JSON value.
    fn infer_type(value: &serde_json::Value) -> DataType {
        match value {
            serde_json::Value::String(_) => DataType::String,
            serde_json::Value::Number(n) if n.is_i64() => DataType::Integer,
            serde_json::Value::Number(_) => DataType::Number,
            serde_json::Value::Bool(_) => DataType::Boolean,
            serde_json::Value::Array(_) => DataType::Array,
            serde_json::Value::Object(_) => DataType::Object,
            serde_json::Value::Null => DataType::String, // Default to string for null
        }
    }

    /// Generate implementation code for a tool.
    fn generate_implementation(&self, request: &GenerationRequest, spec: &ToolSpec) -> Result<String> {
        match request.tool_type {
            ToolType::Function => self.generate_function_impl(request, spec),
            ToolType::Script => self.generate_script_impl(request, spec),
            ToolType::McpServer => self.generate_mcp_impl(request),
            ToolType::Workflow => self.generate_workflow_impl(request, spec),
        }
    }

    /// Generate a function implementation.
    fn generate_function_impl(&self, request: &GenerationRequest, spec: &ToolSpec) -> Result<String> {
        let schema = spec.generate_schema();
        let impl_json = serde_json::json!({
            "type": "function",
            "name": request.name,
            "description": request.description,
            "parameters": schema
        });

        Ok(serde_json::to_string_pretty(&impl_json)?)
    }

    /// Generate a script implementation.
    fn generate_script_impl(&self, request: &GenerationRequest, spec: &ToolSpec) -> Result<String> {
        // Generate a placeholder script
        let mut script = format!(
            "#!/bin/bash\n# Tool: {}\n# Description: {}\n\n",
            request.name, request.description
        );

        // Add input parsing
        script.push_str("# Parse inputs from JSON stdin\n");
        for input in &spec.inputs {
            script.push_str(&format!(
                "# {}: {} ({:?})\n",
                input.name, input.description, input.data_type
            ));
        }

        script.push_str("\n# TODO: Implement tool logic here\n");
        script.push_str("echo '{\"status\": \"not_implemented\"}'\n");

        Ok(script)
    }

    /// Generate an MCP server implementation.
    fn generate_mcp_impl(&self, request: &GenerationRequest) -> Result<String> {
        let config = serde_json::json!({
            "name": request.name,
            "description": request.description,
            "transport": {
                "type": "stdio"
            },
            "tools": [{
                "name": request.name,
                "description": request.description,
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }]
        });

        Ok(serde_json::to_string_pretty(&config)?)
    }

    /// Generate a workflow implementation.
    fn generate_workflow_impl(&self, request: &GenerationRequest, spec: &ToolSpec) -> Result<String> {
        let workflow = serde_json::json!({
            "name": request.name,
            "description": request.description,
            "version": "1.0",
            "steps": [
                {
                    "id": "step1",
                    "name": "Initial step",
                    "type": "action",
                    "action": "TODO",
                    "inputs": spec.inputs.iter().map(|i| &i.name).collect::<Vec<_>>()
                }
            ],
            "outputs": spec.outputs.iter().map(|o| &o.name).collect::<Vec<_>>()
        });

        Ok(serde_json::to_string_pretty(&workflow)?)
    }
}

impl Default for ToolGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_generation_request() {
        let request = GenerationRequest::new("my-tool", "A test tool", ToolCategory::Utility)
            .with_type(ToolType::Function)
            .with_tags(vec!["test".to_string()]);

        assert_eq!(request.name, "my-tool");
        assert_eq!(request.tool_type, ToolType::Function);
    }

    #[test]
    fn test_tool_generation() {
        let generator = ToolGenerator::new().without_review();
        let request = GenerationRequest::new("test-tool", "A test tool", ToolCategory::Utility)
            .with_example_inputs(serde_json::json!({"name": "test", "count": 5}));

        let result = generator.generate(request).unwrap();
        assert_eq!(result.tool.name, "test-tool");
        assert!(!result.needs_review);
    }

    #[test]
    fn test_type_inference() {
        assert_eq!(
            ToolGenerator::infer_type(&serde_json::json!("hello")),
            DataType::String
        );
        assert_eq!(
            ToolGenerator::infer_type(&serde_json::json!(42)),
            DataType::Integer
        );
        assert_eq!(
            ToolGenerator::infer_type(&serde_json::json!(3.14)),
            DataType::Number
        );
        assert_eq!(
            ToolGenerator::infer_type(&serde_json::json!(true)),
            DataType::Boolean
        );
    }
}
