use super::{ClaudeAgentOptions, Skills, SystemPrompt};
use serde_json::Map;
use serde_json::Value;

impl ClaudeAgentOptions {
    pub(crate) fn initialize_payload(&self) -> Value {
        let mut request = Map::new();
        request.insert("subtype".into(), Value::String("initialize".into()));
        request.insert("hooks".into(), Value::Null);
        if let Some(plan_mode_instructions) = &self.plan_mode_instructions {
            request.insert(
                "planModeInstructions".into(),
                Value::String(plan_mode_instructions.clone()),
            );
        }
        if let Some(append_subagent_system_prompt) = &self.append_subagent_system_prompt {
            request.insert(
                "appendSubagentSystemPrompt".into(),
                Value::String(append_subagent_system_prompt.clone()),
            );
        }
        match &self.system_prompt {
            None => {
                request.insert(
                    "systemPrompt".into(),
                    Value::Array(vec![Value::String(String::new())]),
                );
            }
            Some(SystemPrompt::Text(text)) => {
                request.insert(
                    "systemPrompt".into(),
                    Value::Array(vec![Value::String(text.clone())]),
                );
            }
            Some(SystemPrompt::Blocks(blocks)) => {
                request.insert(
                    "systemPrompt".into(),
                    Value::Array(blocks.iter().cloned().map(Value::String).collect()),
                );
            }
            Some(SystemPrompt::Preset {
                append: Some(append),
                ..
            }) => {
                request.insert("appendSystemPrompt".into(), Value::String(append.clone()));
            }
            Some(SystemPrompt::Preset { append: None, .. }) | Some(SystemPrompt::File { .. }) => {}
        }
        if !self.tool_aliases.is_empty() {
            request.insert(
                "toolAliases".into(),
                serde_json::to_value(&self.tool_aliases).unwrap_or(Value::Null),
            );
        }
        if let Some(title) = &self.title {
            request.insert("title".into(), Value::String(title.clone()));
        }
        if !self.web_search_isolation_exempt_mcp_servers.is_empty() {
            request.insert(
                "webSearchIsolationExemptMcpServers".into(),
                Value::Array(
                    self.web_search_isolation_exempt_mcp_servers
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect(),
                ),
            );
        }
        if self.prompt_suggestions {
            request.insert("promptSuggestions".into(), Value::Bool(true));
        }
        if self.agent_progress_summaries {
            request.insert("agentProgressSummaries".into(), Value::Bool(true));
        }
        if self.forward_subagent_text {
            request.insert("forwardSubagentText".into(), Value::Bool(true));
        }
        if !self.agents.is_empty() {
            request.insert(
                "agents".into(),
                serde_json::to_value(&self.agents).unwrap_or(Value::Null),
            );
        }
        if let Some(SystemPrompt::Preset {
            exclude_dynamic_sections: Some(value),
            ..
        }) = &self.system_prompt
        {
            request.insert("excludeDynamicSections".into(), Value::Bool(*value));
        }
        if let Some(Skills::List(skills)) = &self.skills {
            request.insert(
                "skills".into(),
                Value::Array(skills.iter().cloned().map(Value::String).collect()),
            );
        }
        Value::Object(request)
    }
}
