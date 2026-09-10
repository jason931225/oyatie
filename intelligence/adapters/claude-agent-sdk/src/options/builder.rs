use super::McpServers;
use super::{
    AskUserQuestionToolConfig, ClaudeAgentOptions, McpServerConfig, PermissionMode,
    QuestionPreviewFormat, SettingSource, Skills, SystemPrompt, TaskBudget, ToolConfig,
};
use serde_json::Map;
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

mod callbacks;

#[derive(Debug, Default, Clone)]
pub struct ClaudeAgentOptionsBuilder {
    options: ClaudeAgentOptions,
}

impl ClaudeAgentOptionsBuilder {
    pub fn build(self) -> ClaudeAgentOptions {
        self.options
    }

    pub fn cli_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.options.cli_path = Some(path.into());
        self
    }

    pub fn executable(mut self, executable: impl Into<PathBuf>) -> Self {
        self.options.executable = Some(executable.into());
        self
    }

    pub fn executable_arg(mut self, arg: impl Into<String>) -> Self {
        self.options.executable_args.push(arg.into());
        self
    }

    pub fn executable_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.options.executable_args = args.into_iter().map(Into::into).collect();
        self
    }

    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.options.cwd = Some(path.into());
        self
    }

    pub fn agent(mut self, agent: impl Into<String>) -> Self {
        self.options.agent = Some(agent.into());
        self
    }

    pub fn allowed_tool(mut self, tool: impl Into<String>) -> Self {
        self.options.allowed_tools.push(tool.into());
        self
    }

    pub fn allowed_tools(mut self, tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.options.allowed_tools = tools.into_iter().map(Into::into).collect();
        self
    }

    pub fn disallowed_tool(mut self, tool: impl Into<String>) -> Self {
        self.options.disallowed_tools.push(tool.into());
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<SystemPrompt>) -> Self {
        self.options.system_prompt = Some(prompt.into());
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.options.model = Some(model.into());
        self
    }

    pub fn fallback_model(mut self, model: impl Into<String>) -> Self {
        self.options.fallback_model = Some(model.into());
        self
    }

    pub fn max_turns(mut self, max_turns: u32) -> Self {
        self.options.max_turns = Some(max_turns);
        self
    }

    pub fn max_budget_usd(mut self, max_budget_usd: f64) -> Self {
        self.options.max_budget_usd = Some(max_budget_usd);
        self
    }

    pub fn task_budget(mut self, total: u64) -> Self {
        self.options.task_budget = Some(TaskBudget { total });
        self
    }

    pub fn resume(mut self, session_id: impl Into<String>) -> Self {
        self.options.resume = Some(session_id.into());
        self
    }

    pub fn resume_session_at(mut self, message_uuid: impl Into<String>) -> Self {
        self.options.resume_session_at = Some(message_uuid.into());
        self
    }

    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.options.session_id = Some(session_id.into());
        self
    }

    pub fn continue_conversation(mut self, enabled: bool) -> Self {
        self.options.continue_conversation = enabled;
        self
    }

    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.options.permission_mode = Some(mode);
        self
    }

    pub fn allow_dangerously_skip_permissions(mut self, enabled: bool) -> Self {
        self.options.allow_dangerously_skip_permissions = enabled;
        self
    }

    pub fn mcp_servers(mut self, servers: BTreeMap<String, McpServerConfig>) -> Self {
        self.options.mcp_servers = McpServers::Map(servers);
        self
    }

    pub fn setting_sources(mut self, sources: Vec<SettingSource>) -> Self {
        self.options.setting_sources = Some(sources);
        self
    }

    pub fn skills(mut self, skills: Skills) -> Self {
        self.options.skills = Some(skills);
        self
    }

    pub fn debug(mut self, enabled: bool) -> Self {
        self.options.debug = enabled;
        self
    }

    pub fn debug_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.options.debug_file = Some(path.into());
        self
    }

    pub fn settings(mut self, settings: impl Into<String>) -> Self {
        self.options.settings = Some(settings.into());
        self
    }

    pub fn managed_settings(mut self, settings: impl Into<Value>) -> Self {
        self.options.managed_settings = Some(settings.into());
        self
    }

    pub fn add_dir(mut self, directory: impl Into<PathBuf>) -> Self {
        self.options.add_dirs.push(directory.into());
        self
    }

    pub fn add_dirs(mut self, directories: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        self.options.add_dirs = directories.into_iter().map(Into::into).collect();
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.env.insert(key.into(), value.into());
        self
    }

    pub fn extra_arg(mut self, flag: impl Into<String>, value: Option<impl Into<String>>) -> Self {
        self.options
            .extra_args
            .insert(flag.into(), value.map(Into::into));
        self
    }

    pub fn output_format(mut self, output_format: impl Into<Value>) -> Self {
        self.options.output_format = Some(output_format.into());
        self
    }

    pub fn output_format_json_schema(mut self, schema: impl Into<Value>) -> Self {
        self.options.output_format = Some(serde_json::json!({
            "type": "json_schema",
            "schema": schema.into(),
        }));
        self
    }

    pub fn include_partial_messages(mut self, enabled: bool) -> Self {
        self.options.include_partial_messages = enabled;
        self
    }

    pub fn include_hook_events(mut self, enabled: bool) -> Self {
        self.options.include_hook_events = enabled;
        self
    }

    pub fn fork_session(mut self, enabled: bool) -> Self {
        self.options.fork_session = enabled;
        self
    }

    pub fn forward_subagent_text(mut self, enabled: bool) -> Self {
        self.options.forward_subagent_text = enabled;
        self
    }

    pub fn prompt_suggestions(mut self, enabled: bool) -> Self {
        self.options.prompt_suggestions = enabled;
        self
    }

    pub fn agent_progress_summaries(mut self, enabled: bool) -> Self {
        self.options.agent_progress_summaries = enabled;
        self
    }

    pub fn plan_mode_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.options.plan_mode_instructions = Some(instructions.into());
        self
    }

    pub fn append_subagent_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.options.append_subagent_system_prompt = Some(prompt.into());
        self
    }

    pub fn tool_alias(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.options.tool_aliases.insert(from.into(), to.into());
        self
    }

    pub fn tool_aliases(mut self, aliases: BTreeMap<String, String>) -> Self {
        self.options.tool_aliases = aliases;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.options.title = Some(title.into());
        self
    }

    pub fn web_search_isolation_exempt_mcp_server(mut self, server: impl Into<String>) -> Self {
        self.options
            .web_search_isolation_exempt_mcp_servers
            .push(server.into());
        self
    }

    pub fn persist_session(mut self, persist: bool) -> Self {
        self.options.persist_session = Some(persist);
        self
    }

    pub fn ask_user_question_preview_format(mut self, format: QuestionPreviewFormat) -> Self {
        let config = self
            .options
            .tool_config
            .get_or_insert_with(ToolConfig::default);
        let ask_user_question = config
            .ask_user_question
            .get_or_insert_with(AskUserQuestionToolConfig::default);
        ask_user_question.preview_format = Some(format);
        self
    }

    pub fn tool_config(mut self, config: ToolConfig) -> Self {
        self.options.tool_config = Some(config);
        self
    }
}
