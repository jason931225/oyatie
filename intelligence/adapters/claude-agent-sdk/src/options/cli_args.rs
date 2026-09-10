use super::{
    ClaudeAgentOptions, SdkPluginConfig, SettingSource, Skills, SystemPrompt, ThinkingConfig, Tools,
};
use crate::error::ClaudeAgentError;
use crate::error::Result;

#[cfg(test)]
mod tests;

impl ClaudeAgentOptions {
    /// Build the Claude CLI arguments after the executable path.
    ///
    /// Mirrors the upstream Python subprocess transport and TypeScript reference:
    /// stream-json output, verbose logs, and stream-json stdin are always enabled.
    pub fn to_cli_args(&self) -> Result<Vec<String>> {
        self.validate_sdk_mcp_servers()?;
        if self.session_store.is_some() && self.enable_file_checkpointing {
            return Err(ClaudeAgentError::InvalidOption(
                "session_store cannot be combined with enable_file_checkpointing".into(),
            ));
        }
        if self.session_store.is_some() && self.persist_session == Some(false) {
            return Err(ClaudeAgentError::InvalidOption(
                "session_store cannot be combined with persist_session(false)".into(),
            ));
        }
        if self
            .fallback_model
            .as_ref()
            .zip(self.model.as_ref())
            .is_some_and(|(fallback, model)| fallback == model)
        {
            return Err(ClaudeAgentError::InvalidOption(
                "fallback_model cannot be the same as model".into(),
            ));
        }

        let mut args = vec![
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
        ];

        match &self.system_prompt {
            None | Some(SystemPrompt::Text(_)) | Some(SystemPrompt::Blocks(_)) => {}
            Some(SystemPrompt::File { path }) => args.extend([
                "--system-prompt-file".into(),
                path.to_string_lossy().into_owned(),
            ]),
            Some(SystemPrompt::Preset { .. }) => {}
        }

        if let Some(tools) = &self.tools {
            match tools {
                Tools::List(list) if list.is_empty() => args.extend(["--tools".into(), "".into()]),
                Tools::List(list) => args.extend(["--tools".into(), list.join(",")]),
                Tools::Preset { .. } => args.extend(["--tools".into(), "default".into()]),
            }
        }

        let (allowed_tools, setting_sources) = self.effective_allowed_tools_and_setting_sources();
        if !allowed_tools.is_empty() {
            args.extend(["--allowedTools".into(), allowed_tools.join(",")]);
        }
        if let Some(max_turns) = self.max_turns {
            args.extend(["--max-turns".into(), max_turns.to_string()]);
        }
        if let Some(max_budget_usd) = self.max_budget_usd {
            args.extend(["--max-budget-usd".into(), max_budget_usd.to_string()]);
        }
        if !self.disallowed_tools.is_empty() {
            args.extend(["--disallowedTools".into(), self.disallowed_tools.join(",")]);
        }
        if let Some(task_budget) = &self.task_budget {
            args.extend(["--task-budget".into(), task_budget.total.to_string()]);
        }
        if let Some(model) = &self.model {
            args.extend(["--model".into(), model.clone()]);
        }
        if let Some(agent) = &self.agent {
            args.extend(["--agent".into(), agent.clone()]);
        }
        if let Some(fallback_model) = &self.fallback_model {
            args.extend(["--fallback-model".into(), fallback_model.clone()]);
        }
        if !self.betas.is_empty() {
            args.extend(["--betas".into(), self.betas.join(",")]);
        }
        if let Some(debug_file) = &self.debug_file {
            args.extend([
                "--debug-file".into(),
                debug_file.to_string_lossy().into_owned(),
            ]);
        } else if self.debug {
            args.push("--debug".into());
        }
        if self.callbacks.can_use_tool.is_some() && self.permission_prompt_tool_name.is_some() {
            return Err(ClaudeAgentError::InvalidOption(
                "can_use_tool callback cannot be used with permission_prompt_tool_name".into(),
            ));
        }
        if let Some(tool_name) = self
            .permission_prompt_tool_name
            .as_deref()
            .or_else(|| self.callbacks.can_use_tool.as_ref().map(|_| "stdio"))
        {
            args.extend(["--permission-prompt-tool".into(), tool_name.to_owned()]);
        }
        if let Some(mode) = self.permission_mode {
            args.extend(["--permission-mode".into(), mode.as_cli_value().into()]);
        }
        if self.allow_dangerously_skip_permissions {
            args.push("--allow-dangerously-skip-permissions".into());
        }
        if self.continue_conversation {
            args.push("--continue".into());
        }
        if let Some(resume) = &self.resume {
            args.extend(["--resume".into(), resume.clone()]);
        }
        if let Some(resume_session_at) = &self.resume_session_at {
            args.extend(["--resume-session-at".into(), resume_session_at.clone()]);
        }
        if let Some(session_id) = &self.session_id {
            args.extend(["--session-id".into(), session_id.clone()]);
        }
        if self.persist_session == Some(false) {
            args.push("--no-session-persistence".into());
        }
        if let Some(managed_settings) = &self.managed_settings {
            args.extend([
                "--managed-settings".into(),
                serde_json::to_string(managed_settings)?,
            ]);
        }
        if let Some(settings) = self.settings_argument()? {
            args.extend(["--settings".into(), settings]);
        }
        for directory in &self.add_dirs {
            args.extend(["--add-dir".into(), directory.to_string_lossy().into_owned()]);
        }
        if let Some(mcp_config) = self.mcp_config_argument()? {
            args.extend(["--mcp-config".into(), mcp_config]);
        }
        if self.include_partial_messages {
            args.push("--include-partial-messages".into());
        }
        if self.include_hook_events {
            args.push("--include-hook-events".into());
        }
        if self.strict_mcp_config {
            args.push("--strict-mcp-config".into());
        }
        if self.fork_session {
            args.push("--fork-session".into());
        }
        if self.session_store.is_some() {
            args.push("--session-mirror".into());
        }
        if let Some(sources) = setting_sources {
            args.push(format!(
                "--setting-sources={}",
                sources
                    .iter()
                    .map(|s| s.as_cli_value())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        for plugin in &self.plugins {
            match plugin {
                SdkPluginConfig::Local { path } => {
                    args.extend(["--plugin-dir".into(), path.clone()])
                }
            }
        }
        for (flag, value) in &self.extra_args {
            args.push(format!("--{flag}"));
            if let Some(value) = value {
                args.push(value.clone());
            }
        }
        if let Some(thinking) = &self.thinking {
            match thinking {
                ThinkingConfig::Adaptive { display } => {
                    args.extend(["--thinking".into(), "adaptive".into()]);
                    if let Some(display) = display {
                        args.extend(["--thinking-display".into(), display.as_cli_value().into()]);
                    }
                }
                ThinkingConfig::Enabled {
                    budget_tokens,
                    display,
                } => {
                    if let Some(budget_tokens) = budget_tokens {
                        args.extend(["--max-thinking-tokens".into(), budget_tokens.to_string()]);
                    } else {
                        args.extend(["--thinking".into(), "adaptive".into()]);
                    }
                    if let Some(display) = display {
                        args.extend(["--thinking-display".into(), display.as_cli_value().into()]);
                    }
                }
                ThinkingConfig::Disabled => args.extend(["--thinking".into(), "disabled".into()]),
            }
        } else if let Some(tokens) = self.max_thinking_tokens {
            if tokens == 0 {
                args.extend(["--thinking".into(), "disabled".into()]);
            } else {
                args.extend(["--max-thinking-tokens".into(), tokens.to_string()]);
            }
        }
        if let Some(effort) = &self.effort {
            args.extend(["--effort".into(), effort.as_cli_value().into()]);
        }
        if let Some(schema) = self.output_format_json_schema()? {
            args.extend(["--json-schema".into(), serde_json::to_string(&schema)?]);
        }

        args.extend(["--input-format".into(), "stream-json".into()]);
        Ok(args)
    }

    fn effective_allowed_tools_and_setting_sources(
        &self,
    ) -> (Vec<String>, Option<Vec<SettingSource>>) {
        let mut allowed_tools = self.allowed_tools.clone();
        let mut setting_sources = self.setting_sources.clone();

        match &self.skills {
            None => {}
            Some(Skills::All(_)) => {
                if !allowed_tools.iter().any(|tool| tool == "Skill") {
                    allowed_tools.push("Skill".into());
                }
                if setting_sources.is_none() {
                    setting_sources = Some(vec![SettingSource::User, SettingSource::Project]);
                }
            }
            Some(Skills::List(skills)) => {
                for skill in skills {
                    let pattern = format!("Skill({skill})");
                    if !allowed_tools.iter().any(|tool| tool == &pattern) {
                        allowed_tools.push(pattern);
                    }
                }
                if setting_sources.is_none() {
                    setting_sources = Some(vec![SettingSource::User, SettingSource::Project]);
                }
            }
        }

        (allowed_tools, setting_sources)
    }
}
