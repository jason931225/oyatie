use crate::options::*;
use crate::session_store::SessionStoreFlushMode;
use serde_json::Map;
use serde_json::Value;
use std::path::PathBuf;

#[test]
fn deserializes_typescript_option_aliases() {
    let options: ClaudeAgentOptions = serde_json::from_value(serde_json::json!({
        "allowedTools": ["Read"],
        "disallowedTools": ["Bash"],
        "pathToClaudeCodeExecutable": "/tmp/claude",
        "executableArgs": ["--launcher-flag"],
        "additionalDirectories": ["/workspace/extra"],
        "continue": true,
        "maxTurns": 7,
        "maxBudgetUsd": 1.5,
        "fallbackModel": "claude-fallback",
        "enableFileCheckpointing": true,
        "forkSession": true,
        "includePartialMessages": true,
        "includeHookEvents": true,
        "forwardSubagentText": true,
        "promptSuggestions": true,
        "agentProgressSummaries": true,
        "toolAliases": {"Bash": "mcp__sandbox__bash"},
        "planModeInstructions": "plan first",
        "permissionMode": "plan",
        "allowDangerouslySkipPermissions": true,
        "permissionPromptToolName": "stdio",
        "sessionId": "00000000-0000-0000-0000-000000000001",
        "resumeSessionAt": "assistant-uuid",
        "strictMcpConfig": true,
        "debugFile": "/tmp/claude-debug.log",
        "managedSettings": {"permissions": {"deny": ["Bash(rm *)"]}},
        "settingSources": ["user"],
        "systemPrompt": {
            "type": "preset",
            "preset": "claude_code",
            "append": "extra instructions",
            "excludeDynamicSections": true
        },
        "thinking": {"type": "enabled", "budgetTokens": 1234},
        "outputFormat": {"type": "json_schema", "schema": {"type": "object"}},
        "taskBudget": {"total": 1000},
        "loadTimeoutMs": 42,
        "sessionStoreFlush": "eager",
        "persistSession": false,
        "mcpServers": {
            "docs": {
                "type": "http",
                "url": "https://mcp.example/http",
                "timeout": 5000,
                "alwaysLoad": true
            }
        }
    }))
    .unwrap();

    assert_eq!(options.allowed_tools, ["Read"]);
    assert_eq!(options.disallowed_tools, ["Bash"]);
    assert_eq!(options.cli_path, Some(PathBuf::from("/tmp/claude")));
    assert_eq!(options.executable_args, ["--launcher-flag"]);
    assert_eq!(options.add_dirs, [PathBuf::from("/workspace/extra")]);
    assert!(options.continue_conversation);
    assert_eq!(options.max_turns, Some(7));
    assert_eq!(options.max_budget_usd, Some(1.5));
    assert_eq!(options.fallback_model.as_deref(), Some("claude-fallback"));
    assert!(options.enable_file_checkpointing);
    assert!(options.fork_session);
    assert!(options.include_partial_messages);
    assert!(options.include_hook_events);
    assert!(options.forward_subagent_text);
    assert!(options.prompt_suggestions);
    assert!(options.agent_progress_summaries);
    assert_eq!(
        options.tool_aliases.get("Bash").map(String::as_str),
        Some("mcp__sandbox__bash")
    );
    assert_eq!(
        options.plan_mode_instructions.as_deref(),
        Some("plan first")
    );
    assert_eq!(options.permission_mode, Some(PermissionMode::Plan));
    assert!(options.allow_dangerously_skip_permissions);
    assert_eq!(
        options.permission_prompt_tool_name.as_deref(),
        Some("stdio")
    );
    assert_eq!(
        options.session_id.as_deref(),
        Some("00000000-0000-0000-0000-000000000001")
    );
    assert_eq!(options.resume_session_at.as_deref(), Some("assistant-uuid"));
    assert!(options.strict_mcp_config);
    assert_eq!(
        options.debug_file,
        Some(PathBuf::from("/tmp/claude-debug.log"))
    );
    assert_eq!(
        options.managed_settings.as_ref().unwrap()["permissions"]["deny"],
        serde_json::json!(["Bash(rm *)"])
    );
    assert_eq!(options.setting_sources, Some(vec![SettingSource::User]));
    assert!(matches!(
        options.system_prompt,
        Some(SystemPrompt::Preset {
            exclude_dynamic_sections: Some(true),
            ..
        })
    ));
    assert!(matches!(
        options.thinking,
        Some(ThinkingConfig::Enabled {
            budget_tokens: Some(1234),
            ..
        })
    ));
    assert_eq!(
        options.output_format.as_ref().unwrap()["type"],
        "json_schema"
    );
    assert_eq!(
        options.task_budget.as_ref().map(|budget| budget.total),
        Some(1000)
    );
    assert_eq!(options.load_timeout_ms, Some(42));
    assert_eq!(options.session_store_flush, SessionStoreFlushMode::Eager);
    assert_eq!(options.persist_session, Some(false));
    assert!(matches!(
        &options.mcp_servers,
        McpServers::Map(servers)
            if matches!(servers.get("docs"), Some(McpServerConfig::Http { timeout: Some(5000), always_load: Some(true), .. }))
    ));
}

#[test]
fn deserializes_typescript_executable_option() {
    let options: ClaudeAgentOptions = serde_json::from_value(serde_json::json!({
        "executable": "node",
        "executableArgs": ["--runtime-flag"],
        "pathToClaudeCodeExecutable": "/tmp/claude.mjs"
    }))
    .unwrap();

    assert_eq!(options.executable, Some(PathBuf::from("node")));
    assert_eq!(options.executable_args, ["--runtime-flag"]);
    assert_eq!(options.cli_path, Some(PathBuf::from("/tmp/claude.mjs")));
}

#[test]
fn deserializes_typescript_inline_settings_object() {
    let options: ClaudeAgentOptions = serde_json::from_value(serde_json::json!({
        "settings": {
            "model": "claude-test",
            "permissions": {"allow": ["Read"]}
        }
    }))
    .unwrap();

    let args = options.to_cli_args().unwrap();
    let settings = args
        .windows(2)
        .find_map(|window| (window[0] == "--settings").then(|| &window[1]))
        .expect("settings arg");
    let settings: Value = serde_json::from_str(settings).unwrap();
    assert_eq!(settings["model"], "claude-test");
    assert_eq!(
        settings["permissions"]["allow"],
        serde_json::json!(["Read"])
    );
}

#[test]
fn deserializes_typescript_system_prompt_string_and_blocks() {
    let text: ClaudeAgentOptions = serde_json::from_value(serde_json::json!({
        "systemPrompt": "static prompt"
    }))
    .unwrap();
    assert_eq!(
        text.initialize_payload()["systemPrompt"],
        serde_json::json!(["static prompt"])
    );

    let blocks: ClaudeAgentOptions = serde_json::from_value(serde_json::json!({
        "systemPrompt": [
            "static instructions",
            crate::SYSTEM_PROMPT_DYNAMIC_BOUNDARY,
            "session context"
        ]
    }))
    .unwrap();
    assert_eq!(
        blocks.initialize_payload()["systemPrompt"],
        serde_json::json!([
            "static instructions",
            crate::SYSTEM_PROMPT_DYNAMIC_BOUNDARY,
            "session context"
        ])
    );
}
