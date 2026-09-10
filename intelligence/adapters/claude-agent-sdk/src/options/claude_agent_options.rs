use super::settings_argument::deserialize_settings_option;
use super::{
    AgentDefinition, ClaudeAgentOptionsBuilder, EffortLevel, McpServers, PermissionMode,
    SandboxSettings, SdkBeta, SdkPluginConfig, SettingSource, Skills, SystemPrompt, TaskBudget,
    ThinkingConfig, ToolConfig, Tools,
};
use crate::callbacks::CallbackRegistry;
use crate::session_store::SessionStoreFlushMode;
use crate::session_store::SharedSessionStore;
use crate::transport::SharedClaudeProcessSpawner;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

#[cfg(test)]
mod tests;

/// Main query/client options for Rust SDK sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeAgentOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Tools>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, alias = "allowedTools", skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(alias = "systemPrompt", skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<SystemPrompt>,
    #[serde(default, alias = "mcpServers")]
    pub mcp_servers: McpServers,
    #[serde(default, alias = "strictMcpConfig")]
    pub strict_mcp_config: bool,
    #[serde(alias = "permissionMode", skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<PermissionMode>,
    #[serde(default, alias = "continue", alias = "continueConversation")]
    pub continue_conversation: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
    #[serde(alias = "sessionId", skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(alias = "resumeSessionAt", skip_serializing_if = "Option::is_none")]
    pub resume_session_at: Option<String>,
    #[serde(alias = "maxTurns", skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    #[serde(alias = "maxBudgetUsd", skip_serializing_if = "Option::is_none")]
    pub max_budget_usd: Option<f64>,
    #[serde(
        default,
        alias = "disallowedTools",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub disallowed_tools: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(alias = "fallbackModel", skip_serializing_if = "Option::is_none")]
    pub fallback_model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub betas: Vec<SdkBeta>,
    #[serde(default, alias = "allowDangerouslySkipPermissions")]
    pub allow_dangerously_skip_permissions: bool,
    #[serde(
        alias = "permissionPromptToolName",
        skip_serializing_if = "Option::is_none"
    )]
    pub permission_prompt_tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    #[serde(
        alias = "pathToClaudeCodeExecutable",
        skip_serializing_if = "Option::is_none"
    )]
    pub cli_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<PathBuf>,
    #[serde(
        default,
        alias = "executableArgs",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub executable_args: Vec<String>,
    #[serde(alias = "debugFile", skip_serializing_if = "Option::is_none")]
    pub debug_file: Option<PathBuf>,
    #[serde(default)]
    pub debug: bool,
    #[serde(
        default,
        deserialize_with = "deserialize_settings_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub settings: Option<String>,
    #[serde(alias = "managedSettings", skip_serializing_if = "Option::is_none")]
    pub managed_settings: Option<Value>,
    #[serde(
        default,
        alias = "additionalDirectories",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub add_dirs: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(
        default,
        alias = "extraArgs",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub extra_args: BTreeMap<String, Option<String>>,
    #[serde(alias = "maxBufferSize", skip_serializing_if = "Option::is_none")]
    pub max_buffer_size: Option<usize>,
    #[serde(default, alias = "includePartialMessages")]
    pub include_partial_messages: bool,
    #[serde(default, alias = "includeHookEvents")]
    pub include_hook_events: bool,
    #[serde(default, alias = "forkSession")]
    pub fork_session: bool,
    #[serde(default, alias = "forwardSubagentText")]
    pub forward_subagent_text: bool,
    #[serde(default, alias = "promptSuggestions")]
    pub prompt_suggestions: bool,
    #[serde(default, alias = "agentProgressSummaries")]
    pub agent_progress_summaries: bool,
    #[serde(
        default,
        alias = "toolAliases",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub tool_aliases: BTreeMap<String, String>,
    #[serde(
        alias = "planModeInstructions",
        skip_serializing_if = "Option::is_none"
    )]
    pub plan_mode_instructions: Option<String>,
    #[serde(
        alias = "appendSubagentSystemPrompt",
        skip_serializing_if = "Option::is_none"
    )]
    pub append_subagent_system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(
        default,
        alias = "webSearchIsolationExemptMcpServers",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub web_search_isolation_exempt_mcp_servers: Vec<String>,
    #[serde(alias = "persistSession", skip_serializing_if = "Option::is_none")]
    pub persist_session: Option<bool>,
    #[serde(alias = "toolConfig", skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agents: BTreeMap<String, AgentDefinition>,
    #[serde(alias = "settingSources", skip_serializing_if = "Option::is_none")]
    pub setting_sources: Option<Vec<SettingSource>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Skills>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<SandboxSettings>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<SdkPluginConfig>,
    #[serde(alias = "maxThinkingTokens", skip_serializing_if = "Option::is_none")]
    pub max_thinking_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<EffortLevel>,
    #[serde(alias = "outputFormat", skip_serializing_if = "Option::is_none")]
    pub output_format: Option<Value>,
    #[serde(default, alias = "enableFileCheckpointing")]
    pub enable_file_checkpointing: bool,
    #[serde(skip)]
    pub session_store: Option<SharedSessionStore>,
    #[serde(default, alias = "sessionStoreFlush")]
    pub session_store_flush: SessionStoreFlushMode,
    #[serde(
        default,
        alias = "loadTimeoutMs",
        skip_serializing_if = "Option::is_none"
    )]
    pub load_timeout_ms: Option<u64>,
    #[serde(alias = "taskBudget", skip_serializing_if = "Option::is_none")]
    pub task_budget: Option<TaskBudget>,
    #[serde(skip)]
    pub callbacks: CallbackRegistry,
    #[serde(skip)]
    pub spawn_claude_code_process: Option<SharedClaudeProcessSpawner>,
}

impl ClaudeAgentOptions {
    pub fn builder() -> ClaudeAgentOptionsBuilder {
        ClaudeAgentOptionsBuilder::default()
    }
}
