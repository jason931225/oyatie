use crate::error::ClaudeAgentError;
use crate::options::*;
use serde_json::Value;

#[test]
fn skills_inject_allowed_tools_and_setting_sources() {
    let options = ClaudeAgentOptions::builder()
        .skills(Skills::List(vec!["review".into()]))
        .build();
    let args = options.to_cli_args().unwrap();
    assert!(
        args.windows(2)
            .any(|w| w == ["--allowedTools", "Skill(review)"])
    );
    assert!(
        args.iter()
            .any(|arg| arg == "--setting-sources=user,project")
    );
}

#[test]
fn emits_major_cli_flags() {
    let options = ClaudeAgentOptions::builder()
        .system_prompt("You are useful")
        .allowed_tools(["Read", "Bash"])
        .permission_mode(PermissionMode::AcceptEdits)
        .max_turns(3)
        .model("claude-test")
        .build();
    let args = options.to_cli_args().unwrap();
    assert_eq!(args[0..3], ["--output-format", "stream-json", "--verbose"]);
    assert!(
        args.windows(2)
            .any(|w| w == ["--allowedTools", "Read,Bash"])
    );
    assert!(
        args.windows(2)
            .any(|w| w == ["--permission-mode", "acceptEdits"])
    );
    assert!(args.windows(2).any(|w| w == ["--max-turns", "3"]));
    assert!(args.windows(2).any(|w| w == ["--model", "claude-test"]));
    assert_eq!(args[args.len() - 2..], ["--input-format", "stream-json"]);
}

#[test]
fn system_prompt_uses_current_initialize_payload_shape() {
    assert_eq!(
        ClaudeAgentOptions::default().initialize_payload()["systemPrompt"],
        serde_json::json!([""])
    );

    let text = ClaudeAgentOptions::builder()
        .system_prompt("static prompt")
        .build();
    let args = text.to_cli_args().unwrap();
    assert!(!args.iter().any(|arg| arg == "--system-prompt"));
    assert_eq!(
        text.initialize_payload()["systemPrompt"],
        serde_json::json!(["static prompt"])
    );

    let blocks = ClaudeAgentOptions::builder()
        .system_prompt(SystemPrompt::Blocks(vec![
            "static instructions".into(),
            crate::SYSTEM_PROMPT_DYNAMIC_BOUNDARY.into(),
            "session context".into(),
        ]))
        .build();
    let args = blocks.to_cli_args().unwrap();
    assert!(!args.iter().any(|arg| arg == "--system-prompt"));
    assert_eq!(
        blocks.initialize_payload()["systemPrompt"],
        serde_json::json!([
            "static instructions",
            crate::SYSTEM_PROMPT_DYNAMIC_BOUNDARY,
            "session context"
        ])
    );

    let preset = ClaudeAgentOptions::builder()
        .system_prompt(SystemPrompt::Preset {
            preset: "claude_code".into(),
            append: Some("extra instructions".into()),
            exclude_dynamic_sections: Some(true),
        })
        .build();
    let args = preset.to_cli_args().unwrap();
    assert!(!args.iter().any(|arg| arg == "--append-system-prompt"));
    let payload = preset.initialize_payload();
    assert_eq!(payload["appendSystemPrompt"], "extra instructions");
    assert_eq!(payload["excludeDynamicSections"], true);
}

#[test]
fn emits_current_upstream_option_flags_and_initialize_payload() {
    let options = ClaudeAgentOptions::builder()
        .agent("reviewer")
        .allow_dangerously_skip_permissions(true)
        .debug(true)
        .debug_file("/tmp/claude-debug.log")
        .managed_settings(serde_json::json!({"permissions": {"deny": ["Bash(rm *)"]}}))
        .persist_session(false)
        .resume_session_at("assistant-uuid")
        .plan_mode_instructions("write a plan")
        .append_subagent_system_prompt("subagent note")
        .tool_alias("Bash", "mcp__workspace__bash")
        .title("SDK parity")
        .web_search_isolation_exempt_mcp_server("docs")
        .prompt_suggestions(true)
        .agent_progress_summaries(true)
        .forward_subagent_text(true)
        .build();
    let args = options.to_cli_args().unwrap();
    assert!(args.windows(2).any(|w| w == ["--agent", "reviewer"]));
    assert!(
        args.iter()
            .any(|arg| arg == "--allow-dangerously-skip-permissions")
    );
    assert!(
        args.windows(2)
            .any(|w| w == ["--debug-file", "/tmp/claude-debug.log"])
    );
    assert!(!args.iter().any(|arg| arg == "--debug"));
    assert!(args.iter().any(|arg| arg == "--no-session-persistence"));
    assert!(
        args.windows(2)
            .any(|w| w == ["--resume-session-at", "assistant-uuid"])
    );
    let managed_settings = args
        .windows(2)
        .find_map(|window| (window[0] == "--managed-settings").then(|| &window[1]))
        .expect("managed settings arg");
    let managed_settings: Value = serde_json::from_str(managed_settings).unwrap();
    assert_eq!(
        managed_settings["permissions"]["deny"],
        serde_json::json!(["Bash(rm *)"])
    );

    let init = options.initialize_payload();
    assert_eq!(init["planModeInstructions"], "write a plan");
    assert_eq!(init["appendSubagentSystemPrompt"], "subagent note");
    assert_eq!(init["toolAliases"]["Bash"], "mcp__workspace__bash");
    assert_eq!(init["title"], "SDK parity");
    assert_eq!(
        init["webSearchIsolationExemptMcpServers"],
        serde_json::json!(["docs"])
    );
    assert_eq!(init["promptSuggestions"], true);
    assert_eq!(init["agentProgressSummaries"], true);
    assert_eq!(init["forwardSubagentText"], true);
}

#[test]
fn rejects_upstream_option_conflicts() {
    let same_model = ClaudeAgentOptions::builder()
        .model("claude-test")
        .fallback_model("claude-test")
        .build();
    assert!(matches!(
        same_model.to_cli_args(),
        Err(ClaudeAgentError::InvalidOption(message))
            if message.contains("fallback_model cannot be the same as model")
    ));

    let session_store_with_no_persistence = ClaudeAgentOptions::builder()
        .session_store(crate::session_store::InMemorySessionStore::default())
        .persist_session(false)
        .build();
    assert!(matches!(
        session_store_with_no_persistence.to_cli_args(),
        Err(ClaudeAgentError::InvalidOption(message))
            if message.contains("session_store cannot be combined with persist_session(false)")
    ));
}
