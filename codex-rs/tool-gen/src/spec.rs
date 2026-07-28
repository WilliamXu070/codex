//! Tool specification types.
//!
//! Defines the inputs, outputs, and schema for tools.

use serde::{Deserialize, Serialize};

/// Specification for a tool's interface.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Input parameters for the tool.
    pub inputs: Vec<ToolInput>,

    /// Output parameters from the tool.
    pub outputs: Vec<ToolOutput>,

    /// JSON Schema for the tool (OpenAI function calling format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

impl ToolSpec {
    /// Create a new empty tool spec.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an input parameter.
    pub fn with_input(mut self, input: ToolInput) -> Self {
        self.inputs.push(input);
        self
    }

    /// Add an output parameter.
    pub fn with_output(mut self, output: ToolOutput) -> Self {
        self.outputs.push(output);
        self
    }

    /// Set the JSON schema.
    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Generate JSON Schema from inputs.
    pub fn generate_schema(&self) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for input in &self.inputs {
            properties.insert(input.name.clone(), input.to_schema());
            if input.required {
                required.push(serde_json::Value::String(input.name.clone()));
            }
        }

        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }

    /// Validate input values against the spec.
    pub fn validate_inputs(&self, values: &serde_json::Value) -> Result<(), String> {
        let obj = values
            .as_object()
            .ok_or_else(|| "Input must be an object".to_string())?;

        for input in &self.inputs {
            if input.required && !obj.contains_key(&input.name) {
                return Err(format!("Missing required input: {}", input.name));
            }

            if let Some(value) = obj.get(&input.name) {
                input.validate(value)?;
            }
        }

        Ok(())
    }
}

/// An input parameter for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    /// Parameter name.
    pub name: String,

    /// Data type.
    pub data_type: DataType,

    /// Description of the parameter.
    pub description: String,

    /// Whether the parameter is required.
    pub required: bool,

    /// Default value (if not required).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,

    /// Validation constraints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<InputConstraints>,
}

impl ToolInput {
    /// Create a new required input parameter.
    pub fn required(
        name: impl Into<String>,
        data_type: DataType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            description: description.into(),
            required: true,
            default: None,
            constraints: None,
        }
    }

    /// Create a new optional input parameter.
    pub fn optional(
        name: impl Into<String>,
        data_type: DataType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            description: description.into(),
            required: false,
            default: None,
            constraints: None,
        }
    }

    /// Set a default value.
    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default = Some(default);
        self.required = false;
        self
    }

    /// Add constraints.
    pub fn with_constraints(mut self, constraints: InputConstraints) -> Self {
        self.constraints = Some(constraints);
        self
    }

    /// Convert to JSON Schema.
    pub fn to_schema(&self) -> serde_json::Value {
        let mut schema = serde_json::json!({
            "type": self.data_type.to_json_type(),
            "description": self.description
        });

        if let Some(default) = &self.default {
            schema["default"] = default.clone();
        }

        if let Some(constraints) = &self.constraints {
            constraints.apply_to_schema(&mut schema);
        }

        schema
    }

    /// Validate a value against this input's constraints.
    pub fn validate(&self, value: &serde_json::Value) -> Result<(), String> {
        // Type checking
        let valid_type = match self.data_type {
            DataType::String => value.is_string(),
            DataType::Integer => value.is_i64(),
            DataType::Number => value.is_number(),
            DataType::Boolean => value.is_boolean(),
            DataType::Array => value.is_array(),
            DataType::Object => value.is_object(),
        };

        if !valid_type {
            return Err(format!(
                "Invalid type for {}: expected {:?}",
                self.name, self.data_type
            ));
        }

        // Constraint checking
        if let Some(constraints) = &self.constraints {
            constraints.validate(value, &self.name)?;
        }

        Ok(())
    }
}

/// An output parameter from a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Output name.
    pub name: String,

    /// Data type.
    pub data_type: DataType,

    /// Description of the output.
    pub description: String,
}

impl ToolOutput {
    /// Create a new output parameter.
    pub fn new(
        name: impl Into<String>,
        data_type: DataType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            description: description.into(),
        }
    }
}

/// Data types for tool parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
}

impl DataType {
    /// Convert to JSON Schema type string.
    pub fn to_json_type(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Array => "array",
            Self::Object => "object",
        }
    }
}

/// Constraints for input validation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputConstraints {
    /// Minimum value (for numbers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,

    /// Maximum value (for numbers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,

    /// Minimum length (for strings/arrays).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,

    /// Maximum length (for strings/arrays).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,

    /// Pattern (regex for strings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// Enumeration of allowed values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<serde_json::Value>>,
}

impl InputConstraints {
    /// Create constraints for a numeric range.
    pub fn range(min: f64, max: f64) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
            ..Default::default()
        }
    }

    /// Create constraints for string length.
    pub fn length(min: usize, max: usize) -> Self {
        Self {
            min_length: Some(min),
            max_length: Some(max),
            ..Default::default()
        }
    }

    /// Create constraints from an enum of allowed values.
    pub fn enum_of(values: Vec<serde_json::Value>) -> Self {
        Self {
            enum_values: Some(values),
            ..Default::default()
        }
    }

    /// Apply constraints to a JSON Schema object.
    pub fn apply_to_schema(&self, schema: &mut serde_json::Value) {
        if let Some(min) = self.min {
            schema["minimum"] = serde_json::json!(min);
        }
        if let Some(max) = self.max {
            schema["maximum"] = serde_json::json!(max);
        }
        if let Some(min_len) = self.min_length {
            schema["minLength"] = serde_json::json!(min_len);
        }
        if let Some(max_len) = self.max_length {
            schema["maxLength"] = serde_json::json!(max_len);
        }
        if let Some(pattern) = &self.pattern {
            schema["pattern"] = serde_json::json!(pattern);
        }
        if let Some(enum_vals) = &self.enum_values {
            schema["enum"] = serde_json::json!(enum_vals);
        }
    }

    /// Validate a value against these constraints.
    pub fn validate(&self, value: &serde_json::Value, name: &str) -> Result<(), String> {
        // Numeric constraints
        if let Some(num) = value.as_f64() {
            if let Some(min) = self.min {
                if num < min {
                    return Err(format!("{name}: value {num} is less than minimum {min}"));
                }
            }
            if let Some(max) = self.max {
                if num > max {
                    return Err(format!("{name}: value {num} is greater than maximum {max}"));
                }
            }
        }

        // String length constraints
        if let Some(s) = value.as_str() {
            if let Some(min) = self.min_length {
                if s.len() < min {
                    return Err(format!(
                        "{name}: string length {} is less than {min}",
                        s.len()
                    ));
                }
            }
            if let Some(max) = self.max_length {
                if s.len() > max {
                    return Err(format!(
                        "{name}: string length {} is greater than {max}",
                        s.len()
                    ));
                }
            }
        }

        // Enum constraints
        if let Some(enum_vals) = &self.enum_values {
            if !enum_vals.contains(value) {
                return Err(format!("{name}: value not in allowed enum"));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_tool_spec_schema_generation() {
        let spec = ToolSpec::new()
            .with_input(ToolInput::required("name", DataType::String, "User name"))
            .with_input(ToolInput::optional("age", DataType::Integer, "User age"));

        let schema = spec.generate_schema();
        assert!(schema["properties"]["name"].is_object());
        assert_eq!(schema["required"], serde_json::json!(["name"]));
    }

    #[test]
    fn test_input_validation() {
        let input = ToolInput::required("count", DataType::Integer, "Count")
            .with_constraints(InputConstraints::range(0.0, 100.0));

        assert!(input.validate(&serde_json::json!(50)).is_ok());
        assert!(input.validate(&serde_json::json!(150)).is_err());
        assert!(input.validate(&serde_json::json!("not a number")).is_err());
    }
}
