use super::PermissionMode;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

/// Subagent definition sent in the SDK initialize control request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinition {
    pub description: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disallowed_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "criticalSystemReminder_EXPERIMENTAL")]
    pub critical_system_reminder_experimental: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<PermissionMode>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::ClaudeAgentOptions;

    #[test]
    fn agent_definition_preserves_experimental_critical_reminder() {
        let options: ClaudeAgentOptions = serde_json::from_value(serde_json::json!({
            "agents": {
                "reviewer": {
                    "description": "reviews changes",
                    "prompt": "Review the diff.",
                    "criticalSystemReminder_EXPERIMENTAL": "Check tests first."
                }
            }
        }))
        .unwrap();

        let init = options.initialize_payload();
        assert_eq!(
            init["agents"]["reviewer"]["criticalSystemReminder_EXPERIMENTAL"],
            "Check tests first."
        );
    }
}
