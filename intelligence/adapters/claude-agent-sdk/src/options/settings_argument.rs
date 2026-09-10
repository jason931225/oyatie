use super::ClaudeAgentOptions;
use crate::error::ClaudeAgentError;
use crate::error::Result;
use serde::Deserialize;
use serde::de;
use serde::de::Deserializer;
use serde_json::Map;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

impl ClaudeAgentOptions {
    pub(super) fn settings_argument(&self) -> Result<Option<String>> {
        match (&self.settings, &self.sandbox) {
            (None, None) => Ok(None),
            (Some(settings), None) => Ok(Some(settings.clone())),
            (settings, Some(sandbox)) => {
                let mut object = if let Some(settings) = settings {
                    parse_settings_object(settings)?
                } else {
                    Map::new()
                };
                object.insert("sandbox".into(), serde_json::to_value(sandbox)?);
                Ok(Some(Value::Object(object).to_string()))
            }
        }
    }
}

fn parse_settings_object(settings: &str) -> Result<Map<String, Value>> {
    let trimmed = settings.trim();
    if trimmed.is_empty() {
        return Ok(Map::new());
    }
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        let value: Value = serde_json::from_str(trimmed)?;
        return value.as_object().cloned().ok_or_else(|| {
            ClaudeAgentError::InvalidOption("settings JSON must be an object".into())
        });
    }

    let path = PathBuf::from(settings);
    if path.exists() {
        let value: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        return value.as_object().cloned().ok_or_else(|| {
            ClaudeAgentError::InvalidOption("settings file JSON must be an object".into())
        });
    }

    Err(ClaudeAgentError::InvalidOption(format!(
        "settings file does not exist: {settings}"
    )))
}

pub(super) fn deserialize_settings_option<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<Value>::deserialize(deserializer)? {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(settings)) => Ok(Some(settings)),
        Some(Value::Object(object)) => Ok(Some(Value::Object(object).to_string())),
        Some(other) => Err(de::Error::custom(format!(
            "settings must be a string path/JSON object or an inline object, got {other}"
        ))),
    }
}
