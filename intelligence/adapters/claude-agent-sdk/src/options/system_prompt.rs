use serde::Deserialize;
use serde::Serialize;
use serde::de;
use serde::de::Deserializer;
use serde::ser::SerializeMap;
use serde_json::Value;
use std::path::PathBuf;

/// System prompt configuration.
#[derive(Debug, Clone, PartialEq)]
pub enum SystemPrompt {
    Text(String),
    Blocks(Vec<String>),
    Preset {
        preset: String,
        append: Option<String>,
        exclude_dynamic_sections: Option<bool>,
    },
    File {
        path: PathBuf,
    },
}

impl Serialize for SystemPrompt {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Text(text) => serializer.serialize_str(text),
            Self::Blocks(blocks) => blocks.serialize(serializer),
            Self::Preset {
                preset,
                append,
                exclude_dynamic_sections,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "preset")?;
                map.serialize_entry("preset", preset)?;
                if let Some(append) = append {
                    map.serialize_entry("append", append)?;
                }
                if let Some(exclude_dynamic_sections) = exclude_dynamic_sections {
                    map.serialize_entry("excludeDynamicSections", exclude_dynamic_sections)?;
                }
                map.end()
            }
            Self::File { path } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "file")?;
                map.serialize_entry("path", path)?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for SystemPrompt {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::String(text) => Ok(Self::Text(text)),
            Value::Array(values) => values
                .into_iter()
                .map(|value| match value {
                    Value::String(block) => Ok(block),
                    other => Err(de::Error::custom(format!(
                        "system prompt blocks must be strings, got {other}"
                    ))),
                })
                .collect::<std::result::Result<Vec<_>, _>>()
                .map(Self::Blocks),
            Value::Object(mut object) => {
                let prompt_type = object
                    .remove("type")
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .ok_or_else(|| de::Error::custom("system prompt object requires type"))?;
                match prompt_type.as_str() {
                    "preset" => {
                        let preset = object
                            .remove("preset")
                            .and_then(|value| value.as_str().map(str::to_owned))
                            .ok_or_else(|| {
                                de::Error::custom("preset system prompt requires preset")
                            })?;
                        let append = object
                            .remove("append")
                            .and_then(|value| value.as_str().map(str::to_owned));
                        let exclude_dynamic_sections = object
                            .remove("excludeDynamicSections")
                            .or_else(|| object.remove("exclude_dynamic_sections"))
                            .and_then(|value| value.as_bool());
                        Ok(Self::Preset {
                            preset,
                            append,
                            exclude_dynamic_sections,
                        })
                    }
                    "file" => {
                        let path = object
                            .remove("path")
                            .and_then(|value| value.as_str().map(PathBuf::from))
                            .ok_or_else(|| de::Error::custom("file system prompt requires path"))?;
                        Ok(Self::File { path })
                    }
                    other => Err(de::Error::custom(format!(
                        "unsupported system prompt type: {other}"
                    ))),
                }
            }
            other => Err(de::Error::custom(format!(
                "system prompt must be a string, string array, or object, got {other}"
            ))),
        }
    }
}

impl From<String> for SystemPrompt {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for SystemPrompt {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<Vec<String>> for SystemPrompt {
    fn from(value: Vec<String>) -> Self {
        Self::Blocks(value)
    }
}

impl From<Vec<&str>> for SystemPrompt {
    fn from(value: Vec<&str>) -> Self {
        Self::Blocks(value.into_iter().map(str::to_owned).collect())
    }
}
