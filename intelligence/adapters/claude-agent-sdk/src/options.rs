use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{
    Deserialize, Serialize,
    de::{self, Deserializer},
    ser::SerializeMap,
};
use serde_json::{Map, Value};

use crate::{
    callbacks::{
        CallbackRegistry, ElicitationCallback, ElicitationCallbackOptions, ElicitationRequest,
        ElicitationResult, HookCallback, HookMatcher, PermissionCallback, StderrCallback,
        TokenRefreshCallback, TokenRefreshCallbackOptions, ToolPermissionRequest,
        UserDialogCallback, UserDialogCallbackOptions, UserDialogRequest,
    },
    error::{ClaudeAgentError, Result},
    session_store::{SessionStore, SessionStoreFlushMode, SharedSessionStore},
    status::{McpServerPermissionPolicy, McpServerToolPolicy},
    tools::SdkMcpServer,
    transport::{ClaudeProcessSpawner, SharedClaudeProcessSpawner},
};

mod agent_definition;
mod builder;
mod claude_agent_options;
mod cli_args;
mod cli_path;
mod initialize;
mod mcp_server;
mod output_format;
mod permission;
mod sandbox;
mod settings_argument;
mod system_prompt;
mod thinking;
mod tool_selection;

pub use agent_definition::*;
pub use builder::*;
pub use claude_agent_options::*;
pub use cli_path::*;
pub use mcp_server::*;
pub use output_format::*;
pub use permission::*;
pub use sandbox::*;
pub use system_prompt::*;
pub use thinking::*;
pub use tool_selection::*;

/// Filesystem settings sources to load for SDK sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingSource {
    User,
    Project,
    Local,
}

impl SettingSource {
    pub fn as_cli_value(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
            Self::Local => "local",
        }
    }
}

/// Model effort / adaptive thinking guidance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EffortLevel {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
    #[serde(untagged)]
    Other(String),
}

impl EffortLevel {
    pub fn as_cli_value(&self) -> &str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
            Self::Other(value) => value,
        }
    }
}

/// SDK beta feature names. Kept open so the Rust SDK does not lag upstream beta headers.
pub type SdkBeta = String;

/// API-side task budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBudget {
    pub total: u64,
}

/// Plugin configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SdkPluginConfig {
    Local { path: String },
}
