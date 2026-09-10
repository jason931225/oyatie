use crate::error::ClaudeAgentError;
use crate::options::*;
use crate::status::McpServerPermissionPolicy;
use crate::status::McpServerToolPolicy;
use crate::tools::SdkMcpServer;
use serde_json::Map;
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::sync::Arc;

#[test]
fn sdk_mcp_server_emits_serializable_config() {
    let server = crate::tools::create_sdk_mcp_server("calculator", "1.0.0", vec![]);
    let options = ClaudeAgentOptions::builder()
        .sdk_mcp_server("calc", server)
        .build();
    let args = options.to_cli_args().unwrap();
    let config = args
        .windows(2)
        .find_map(|window| (window[0] == "--mcp-config").then(|| &window[1]))
        .expect("mcp config");
    let config: Value = serde_json::from_str(config).unwrap();
    assert_eq!(config["mcpServers"]["calc"]["type"], "sdk");
    assert_eq!(config["mcpServers"]["calc"]["name"], "calculator");
}

#[test]
fn mcp_server_config_emits_current_remote_fields() {
    let mut servers = BTreeMap::new();
    servers.insert(
        "filesystem".to_owned(),
        McpServerConfig::Stdio {
            command: "fs-mcp".to_owned(),
            args: vec!["--root".to_owned(), ".".to_owned()],
            env: BTreeMap::new(),
            timeout: Some(2500),
            always_load: Some(true),
        },
    );
    servers.insert(
        "docs".to_owned(),
        McpServerConfig::Http {
            url: "https://mcp.example/http".to_owned(),
            headers: BTreeMap::new(),
            tools: vec![crate::status::McpServerToolPolicy {
                name: "read_docs".to_owned(),
                permission_policy: crate::status::McpServerPermissionPolicy::AlwaysDeny,
            }],
            timeout: Some(5000),
            always_load: Some(false),
        },
    );

    let options = ClaudeAgentOptions::builder().mcp_servers(servers).build();
    let args = options.to_cli_args().unwrap();
    let config = args
        .windows(2)
        .find_map(|window| (window[0] == "--mcp-config").then(|| &window[1]))
        .expect("mcp config");
    let config: Value = serde_json::from_str(config).unwrap();
    assert_eq!(config["mcpServers"]["filesystem"]["timeout"], 2500);
    assert_eq!(config["mcpServers"]["filesystem"]["alwaysLoad"], true);
    assert_eq!(config["mcpServers"]["docs"]["timeout"], 5000);
    assert_eq!(config["mcpServers"]["docs"]["alwaysLoad"], false);
    assert_eq!(
        config["mcpServers"]["docs"]["tools"][0]["permission_policy"],
        "always_deny"
    );
}

#[test]
fn mcp_server_config_ignores_subsecond_timeouts_like_upstream() {
    let mut servers = BTreeMap::new();
    servers.insert(
        "filesystem".to_owned(),
        McpServerConfig::Stdio {
            command: "fs-mcp".to_owned(),
            args: Vec::new(),
            env: BTreeMap::new(),
            timeout: Some(999),
            always_load: None,
        },
    );
    servers.insert(
        "docs".to_owned(),
        McpServerConfig::Http {
            url: "https://mcp.example/http".to_owned(),
            headers: BTreeMap::new(),
            tools: Vec::new(),
            timeout: Some(500),
            always_load: None,
        },
    );
    servers.insert(
        "events".to_owned(),
        McpServerConfig::Sse {
            url: "https://mcp.example/sse".to_owned(),
            headers: BTreeMap::new(),
            tools: Vec::new(),
            timeout: Some(1000),
            always_load: None,
        },
    );

    let options = ClaudeAgentOptions::builder().mcp_servers(servers).build();
    let args = options.to_cli_args().unwrap();
    let config = args
        .windows(2)
        .find_map(|window| (window[0] == "--mcp-config").then(|| &window[1]))
        .expect("mcp config");
    let config: Value = serde_json::from_str(config).unwrap();
    assert!(config["mcpServers"]["filesystem"].get("timeout").is_none());
    assert!(config["mcpServers"]["docs"].get("timeout").is_none());
    assert_eq!(config["mcpServers"]["events"]["timeout"], 1000);
}

#[test]
fn deserializes_stdio_mcp_server_without_type_tag() {
    let options: ClaudeAgentOptions = serde_json::from_value(serde_json::json!({
        "mcpServers": {
            "filesystem": {
                "command": "fs-mcp",
                "args": ["--root", "."],
                "env": {"ROOT": "."},
                "timeout": 2500,
                "alwaysLoad": true
            }
        }
    }))
    .unwrap();

    let McpServers::Map(servers) = &options.mcp_servers else {
        panic!("expected map config");
    };
    assert!(matches!(
        servers.get("filesystem"),
        Some(McpServerConfig::Stdio {
            command,
            timeout: Some(2500),
            always_load: Some(true),
            ..
        }) if command == "fs-mcp"
    ));

    let args = options.to_cli_args().unwrap();
    let config = args
        .windows(2)
        .find_map(|window| (window[0] == "--mcp-config").then(|| &window[1]))
        .expect("mcp config");
    let config: Value = serde_json::from_str(config).unwrap();
    assert_eq!(config["mcpServers"]["filesystem"]["type"], "stdio");
    assert_eq!(config["mcpServers"]["filesystem"]["command"], "fs-mcp");
}

#[test]
fn preserves_unknown_mcp_server_configs_for_protocol_drift() {
    let options: ClaudeAgentOptions = serde_json::from_value(serde_json::json!({
        "mcpServers": {
            "workspace": {
                "type": "ws",
                "url": "wss://mcp.example/ws",
                "headersHelper": "mcp-headers",
                "role": "comms",
                "timeout": 2500,
                "alwaysLoad": true
            }
        }
    }))
    .unwrap();

    let args = options.to_cli_args().unwrap();
    let config = args
        .windows(2)
        .find_map(|window| (window[0] == "--mcp-config").then(|| &window[1]))
        .expect("mcp config");
    let config: Value = serde_json::from_str(config).unwrap();
    assert_eq!(config["mcpServers"]["workspace"]["type"], "ws");
    assert_eq!(
        config["mcpServers"]["workspace"]["headersHelper"],
        "mcp-headers"
    );
    assert_eq!(config["mcpServers"]["workspace"]["role"], "comms");
    assert_eq!(config["mcpServers"]["workspace"]["timeout"], 2500);
    assert_eq!(config["mcpServers"]["workspace"]["alwaysLoad"], true);
}

#[test]
fn sdk_mcp_callback_requires_matching_map_config() {
    let mut options = ClaudeAgentOptions {
        mcp_servers: McpServers::PathOrJson("mcp.json".into()),
        ..Default::default()
    };
    options.callbacks.sdk_mcp_servers.insert(
        "calc".into(),
        Arc::new(crate::tools::create_sdk_mcp_server(
            "calculator",
            "1.0.0",
            vec![],
        )),
    );
    assert!(matches!(
        options.to_cli_args(),
        Err(ClaudeAgentError::InvalidOption(message))
            if message.contains("map-based mcp_servers")
    ));
}
