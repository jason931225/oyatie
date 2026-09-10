use super::ClaudeAgentOptions;
use crate::error::ClaudeAgentError;
use crate::error::Result;
use crate::status::McpServerPermissionPolicy;
use crate::status::McpServerToolPolicy;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use std::collections::BTreeMap;

mod wire;

#[cfg(test)]
mod tests;

/// MCP stdio/SSE/HTTP/SDK server config. Kept structurally flexible for parity.
#[derive(Debug, Clone, PartialEq)]
pub enum McpServerConfig {
    Stdio {
        command: String,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        timeout: Option<u64>,
        always_load: Option<bool>,
    },
    Sse {
        url: String,
        headers: BTreeMap<String, String>,
        tools: Vec<McpServerToolPolicy>,
        timeout: Option<u64>,
        always_load: Option<bool>,
    },
    Http {
        url: String,
        headers: BTreeMap<String, String>,
        tools: Vec<McpServerToolPolicy>,
        timeout: Option<u64>,
        always_load: Option<bool>,
    },
    /// Serializable SDK server descriptor. The in-process instance is owned by `tools`.
    Sdk { name: String },
    /// Forward-compatible raw config for current/future upstream MCP transports.
    Raw(Value),
    /// Legacy placeholder for intentionally opaque configs constructed by callers.
    Unknown,
}

impl McpServerConfig {
    pub fn stdio(command: impl Into<String>) -> Self {
        Self::Stdio {
            command: command.into(),
            args: Vec::new(),
            env: BTreeMap::new(),
            timeout: None,
            always_load: None,
        }
    }

    pub fn sse(url: impl Into<String>) -> Self {
        Self::Sse {
            url: url.into(),
            headers: BTreeMap::new(),
            tools: Vec::new(),
            timeout: None,
            always_load: None,
        }
    }

    pub fn http(url: impl Into<String>) -> Self {
        Self::Http {
            url: url.into(),
            headers: BTreeMap::new(),
            tools: Vec::new(),
            timeout: None,
            always_load: None,
        }
    }

    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        if let Self::Stdio { args, .. } = &mut self {
            args.push(arg.into());
        }
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if let Self::Stdio { env, .. } = &mut self {
            env.insert(key.into(), value.into());
        }
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        match &mut self {
            Self::Sse { headers, .. } | Self::Http { headers, .. } => {
                headers.insert(key.into(), value.into());
            }
            Self::Stdio { .. } | Self::Sdk { .. } | Self::Raw(_) | Self::Unknown => {}
        }
        self
    }

    pub fn with_tool_policy(
        mut self,
        name: impl Into<String>,
        permission_policy: McpServerPermissionPolicy,
    ) -> Self {
        match &mut self {
            Self::Sse { tools, .. } | Self::Http { tools, .. } => {
                tools.push(McpServerToolPolicy {
                    name: name.into(),
                    permission_policy,
                });
            }
            Self::Stdio { .. } | Self::Sdk { .. } | Self::Raw(_) | Self::Unknown => {}
        }
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        match &mut self {
            Self::Stdio { timeout, .. }
            | Self::Sse { timeout, .. }
            | Self::Http { timeout, .. } => *timeout = Some(timeout_ms),
            Self::Sdk { .. } | Self::Raw(_) | Self::Unknown => {}
        }
        self
    }

    pub fn with_always_load(mut self, always_load_enabled: bool) -> Self {
        match &mut self {
            Self::Stdio { always_load, .. }
            | Self::Sse { always_load, .. }
            | Self::Http { always_load, .. } => *always_load = Some(always_load_enabled),
            Self::Sdk { .. } | Self::Raw(_) | Self::Unknown => {}
        }
        self
    }
}

/// MCP config can be supplied as a map, file path, or raw JSON string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpServers {
    Map(BTreeMap<String, McpServerConfig>),
    PathOrJson(String),
}

impl Default for McpServers {
    fn default() -> Self {
        Self::Map(BTreeMap::new())
    }
}

impl ClaudeAgentOptions {
    pub(super) fn mcp_config_argument(&self) -> Result<Option<String>> {
        match &self.mcp_servers {
            McpServers::PathOrJson(path_or_json) if !path_or_json.is_empty() => {
                Ok(Some(path_or_json.clone()))
            }
            McpServers::PathOrJson(_) => Ok(None),
            McpServers::Map(map) if map.is_empty() => Ok(None),
            McpServers::Map(map) => {
                let mut outer = Map::new();
                outer.insert("mcpServers".into(), serde_json::to_value(map)?);
                Ok(Some(Value::Object(outer).to_string()))
            }
        }
    }

    pub(super) fn validate_sdk_mcp_servers(&self) -> Result<()> {
        if self.callbacks.sdk_mcp_servers.is_empty() {
            return Ok(());
        }

        let McpServers::Map(map) = &self.mcp_servers else {
            return Err(ClaudeAgentError::InvalidOption(
                "sdk_mcp_server requires map-based mcp_servers so the CLI receives matching SDK server descriptors".into(),
            ));
        };

        for alias in self.callbacks.sdk_mcp_servers.keys() {
            match map.get(alias) {
                Some(McpServerConfig::Sdk { .. }) => {}
                Some(_) => {
                    return Err(ClaudeAgentError::InvalidOption(format!(
                        "sdk_mcp_server alias '{alias}' conflicts with a non-SDK MCP server config",
                    )));
                }
                None => {
                    return Err(ClaudeAgentError::InvalidOption(format!(
                        "sdk_mcp_server alias '{alias}' is missing from mcp_servers",
                    )));
                }
            }
        }

        Ok(())
    }
}
