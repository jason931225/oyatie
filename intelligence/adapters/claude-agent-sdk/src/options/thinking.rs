use super::ClaudeAgentOptions;
use crate::error::Result;
use serde::Deserialize;
use serde::Serialize;
use serde::de;
use serde::de::Deserializer;
use serde::ser::SerializeMap;

/// Display mode for thinking/reasoning output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingDisplay {
    Summarized,
    Omitted,
    #[serde(untagged)]
    Other(String),
}

impl ThinkingDisplay {
    pub fn as_cli_value(&self) -> &str {
        match self {
            Self::Summarized => "summarized",
            Self::Omitted => "omitted",
            Self::Other(value) => value,
        }
    }
}

impl From<&str> for ThinkingDisplay {
    fn from(value: &str) -> Self {
        match value {
            "summarized" => Self::Summarized,
            "omitted" => Self::Omitted,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl From<String> for ThinkingDisplay {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// Adaptive thinking configuration helper.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThinkingAdaptive {
    pub display: Option<ThinkingDisplay>,
}

impl Serialize for ThinkingAdaptive {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("type", "adaptive")?;
        if let Some(display) = &self.display {
            map.serialize_entry("display", display)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for ThinkingAdaptive {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(rename = "type")]
            kind: String,
            #[serde(default)]
            display: Option<ThinkingDisplay>,
        }

        let wire = Wire::deserialize(deserializer)?;
        if wire.kind != "adaptive" {
            return Err(de::Error::custom(format!(
                "expected adaptive thinking config, got {}",
                wire.kind
            )));
        }
        Ok(Self {
            display: wire.display,
        })
    }
}

/// Fixed-budget thinking configuration helper.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThinkingEnabled {
    pub budget_tokens: Option<u32>,
    pub display: Option<ThinkingDisplay>,
}

impl Serialize for ThinkingEnabled {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("type", "enabled")?;
        if let Some(budget_tokens) = self.budget_tokens {
            map.serialize_entry("budgetTokens", &budget_tokens)?;
        }
        if let Some(display) = &self.display {
            map.serialize_entry("display", display)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for ThinkingEnabled {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(rename = "type")]
            kind: String,
            #[serde(default, rename = "budgetTokens")]
            budget_tokens: Option<u32>,
            #[serde(default)]
            display: Option<ThinkingDisplay>,
        }

        let wire = Wire::deserialize(deserializer)?;
        if wire.kind != "enabled" {
            return Err(de::Error::custom(format!(
                "expected enabled thinking config, got {}",
                wire.kind
            )));
        }
        Ok(Self {
            budget_tokens: wire.budget_tokens,
            display: wire.display,
        })
    }
}

/// Disabled thinking configuration helper.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ThinkingDisabled;

impl Serialize for ThinkingDisabled {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("type", "disabled")?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for ThinkingDisabled {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(rename = "type")]
            kind: String,
        }

        let wire = Wire::deserialize(deserializer)?;
        if wire.kind != "disabled" {
            return Err(de::Error::custom(format!(
                "expected disabled thinking config, got {}",
                wire.kind
            )));
        }
        Ok(Self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ThinkingConfig {
    Adaptive {
        #[serde(skip_serializing_if = "Option::is_none")]
        display: Option<ThinkingDisplay>,
    },
    Enabled {
        #[serde(
            rename = "budgetTokens",
            alias = "budgetTokens",
            skip_serializing_if = "Option::is_none"
        )]
        budget_tokens: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        display: Option<ThinkingDisplay>,
    },
    Disabled,
}

impl From<ThinkingAdaptive> for ThinkingConfig {
    fn from(config: ThinkingAdaptive) -> Self {
        Self::Adaptive {
            display: config.display,
        }
    }
}

impl From<ThinkingEnabled> for ThinkingConfig {
    fn from(config: ThinkingEnabled) -> Self {
        Self::Enabled {
            budget_tokens: config.budget_tokens,
            display: config.display,
        }
    }
}

impl From<ThinkingDisabled> for ThinkingConfig {
    fn from(_: ThinkingDisabled) -> Self {
        Self::Disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_enabled_thinking_without_budget_as_adaptive() {
        let options: ClaudeAgentOptions = serde_json::from_value(serde_json::json!({
            "thinking": {
                "type": "enabled",
                "display": "summarized"
            }
        }))
        .unwrap();

        assert!(matches!(
            options.thinking,
            Some(ThinkingConfig::Enabled {
                budget_tokens: None,
                display: Some(ref display),
            }) if display == &ThinkingDisplay::Summarized
        ));

        let args = options.to_cli_args().unwrap();
        assert!(
            args.windows(2)
                .any(|window| window == ["--thinking", "adaptive"])
        );
        assert!(
            args.windows(2)
                .any(|window| window == ["--thinking-display", "summarized"])
        );
        assert!(!args.iter().any(|arg| arg == "--max-thinking-tokens"));
    }

    #[test]
    fn deprecated_zero_max_thinking_tokens_disables_thinking() {
        let options: ClaudeAgentOptions = serde_json::from_value(serde_json::json!({
            "maxThinkingTokens": 0
        }))
        .unwrap();

        let args = options.to_cli_args().unwrap();
        assert!(
            args.windows(2)
                .any(|window| window == ["--thinking", "disabled"])
        );
        assert!(!args.iter().any(|arg| arg == "--max-thinking-tokens"));
    }
}
