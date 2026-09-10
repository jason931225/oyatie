use super::ClaudeAgentOptions;
use crate::error::Result;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

/// Output format discriminator exported by the upstream SDK.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormatType {
    JsonSchema,
    #[serde(untagged)]
    Other(String),
}

/// Base output-format shape with a type discriminator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaseOutputFormat {
    #[serde(rename = "type")]
    pub output_type: OutputFormatType,
}

/// Structured-response JSON Schema output format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonSchemaOutputFormat {
    #[serde(rename = "type")]
    pub output_type: OutputFormatType,
    pub schema: Value,
}

impl JsonSchemaOutputFormat {
    pub fn new(schema: impl Into<Value>) -> Self {
        Self {
            output_type: OutputFormatType::JsonSchema,
            schema: schema.into(),
        }
    }
}

/// Output format configuration accepted by Claude Code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutputFormat {
    JsonSchema(JsonSchemaOutputFormat),
    Raw(Value),
}

impl OutputFormat {
    pub fn json_schema(schema: impl Into<Value>) -> Self {
        Self::JsonSchema(JsonSchemaOutputFormat::new(schema))
    }
}

impl From<JsonSchemaOutputFormat> for OutputFormat {
    fn from(format: JsonSchemaOutputFormat) -> Self {
        Self::JsonSchema(format)
    }
}

impl From<JsonSchemaOutputFormat> for Value {
    fn from(format: JsonSchemaOutputFormat) -> Self {
        serde_json::to_value(format)
            .expect("json schema output format serialization should be infallible")
    }
}

impl From<OutputFormat> for Value {
    fn from(format: OutputFormat) -> Self {
        serde_json::to_value(format).expect("output format serialization should be infallible")
    }
}

impl ClaudeAgentOptions {
    pub(super) fn output_format_json_schema(&self) -> Result<Option<Value>> {
        let Some(output_format) = &self.output_format else {
            return Ok(None);
        };
        if output_format.get("type").and_then(Value::as_str) == Some("json_schema") {
            Ok(output_format.get("schema").cloned())
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_json_schema_builder_emits_cli_schema() {
        let options = ClaudeAgentOptions::builder()
            .output_format_json_schema(
                crate::tools::JsonSchema::object()
                    .required_property("company_name", crate::tools::JsonSchema::string())
                    .optional_property("founded_year", crate::tools::JsonSchema::integer())
                    .build(),
            )
            .build();
        let args = options.to_cli_args().unwrap();
        let schema = args
            .windows(2)
            .find_map(|window| (window[0] == "--json-schema").then(|| &window[1]))
            .expect("json schema arg");
        let schema: Value = serde_json::from_str(schema).unwrap();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["company_name"]["type"], "string");
        assert_eq!(schema["properties"]["founded_year"]["type"], "integer");
        assert_eq!(schema["required"], serde_json::json!(["company_name"]));
    }
}
