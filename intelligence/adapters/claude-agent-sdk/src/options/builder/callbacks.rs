use super::super::{McpServerConfig, McpServers};
use super::ClaudeAgentOptionsBuilder;
use crate::callbacks::ElicitationCallback;
use crate::callbacks::ElicitationCallbackOptions;
use crate::callbacks::ElicitationRequest;
use crate::callbacks::ElicitationResult;
use crate::callbacks::HookCallback;
use crate::callbacks::HookMatcher;
use crate::callbacks::PermissionCallback;
use crate::callbacks::StderrCallback;
use crate::callbacks::TokenRefreshCallback;
use crate::callbacks::TokenRefreshCallbackOptions;
use crate::callbacks::ToolPermissionRequest;
use crate::callbacks::UserDialogCallback;
use crate::callbacks::UserDialogCallbackOptions;
use crate::callbacks::UserDialogRequest;
use crate::error::Result;
use crate::session_store::SessionStore;
use crate::session_store::SessionStoreFlushMode;
use crate::session_store::SharedSessionStore;
use crate::tools::SdkMcpServer;
use crate::transport::ClaudeProcessSpawner;
use crate::transport::SharedClaudeProcessSpawner;
use std::collections::BTreeMap;
use std::sync::Arc;

impl ClaudeAgentOptionsBuilder {
    pub fn can_use_tool<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(ToolPermissionRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<crate::callbacks::PermissionResult>>
            + Send
            + 'static,
    {
        let callback: PermissionCallback =
            std::sync::Arc::new(move |request| Box::pin(callback(request)));
        self.options.callbacks.can_use_tool = Some(callback);
        self
    }

    pub fn on_elicitation<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(ElicitationRequest, ElicitationCallbackOptions) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<ElicitationResult>> + Send + 'static,
    {
        let callback: ElicitationCallback =
            std::sync::Arc::new(move |request, options| Box::pin(callback(request, options)));
        self.options.callbacks.on_elicitation = Some(callback);
        self
    }

    pub fn get_oauth_token<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(TokenRefreshCallbackOptions) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Option<String>>> + Send + 'static,
    {
        let callback: TokenRefreshCallback =
            std::sync::Arc::new(move |options| Box::pin(callback(options)));
        self.options.callbacks.get_oauth_token = Some(callback);
        self
    }

    pub fn get_host_auth_token<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(TokenRefreshCallbackOptions) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Option<String>>> + Send + 'static,
    {
        let callback: TokenRefreshCallback =
            std::sync::Arc::new(move |options| Box::pin(callback(options)));
        self.options.callbacks.get_host_auth_token = Some(callback);
        self
    }

    pub fn on_user_dialog<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(UserDialogRequest, UserDialogCallbackOptions) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value>> + Send + 'static,
    {
        let callback: UserDialogCallback =
            std::sync::Arc::new(move |request, options| Box::pin(callback(request, options)));
        self.options.callbacks.on_user_dialog = Some(callback);
        self
    }

    pub fn stderr<F>(mut self, callback: F) -> Self
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let callback: StderrCallback = std::sync::Arc::new(callback);
        self.options.callbacks.stderr = Some(callback);
        self
    }

    pub fn hook<F, Fut>(
        mut self,
        event: impl Into<String>,
        matcher: Option<impl Into<String>>,
        timeout: Option<f64>,
        callback: F,
    ) -> Self
    where
        F: Fn(crate::callbacks::HookCallbackRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value>> + Send + 'static,
    {
        let callback: HookCallback =
            std::sync::Arc::new(move |request| Box::pin(callback(request)));
        self.options
            .callbacks
            .hooks
            .entry(event.into())
            .or_default()
            .push(HookMatcher {
                matcher: matcher.map(Into::into),
                hooks: vec![callback],
                timeout,
            });
        self
    }

    pub fn sdk_mcp_server(mut self, alias: impl Into<String>, server: SdkMcpServer) -> Self {
        let alias = alias.into();
        let server_name = server.name.clone();
        self.options
            .callbacks
            .sdk_mcp_servers
            .insert(alias.clone(), Arc::new(server));

        let mut servers = match std::mem::take(&mut self.options.mcp_servers) {
            McpServers::Map(map) => map,
            McpServers::PathOrJson(value) if value.is_empty() => BTreeMap::new(),
            McpServers::PathOrJson(value) => {
                self.options.mcp_servers = McpServers::PathOrJson(value);
                return self;
            }
        };
        servers.insert(alias, McpServerConfig::Sdk { name: server_name });
        self.options.mcp_servers = McpServers::Map(servers);
        self
    }

    pub fn session_store<S>(mut self, store: S) -> Self
    where
        S: SessionStore + 'static,
    {
        self.options.session_store = Some(SharedSessionStore::new(store));
        self
    }

    pub fn shared_session_store(mut self, store: SharedSessionStore) -> Self {
        self.options.session_store = Some(store);
        self
    }

    pub fn session_store_flush(mut self, flush: SessionStoreFlushMode) -> Self {
        self.options.session_store_flush = flush;
        self
    }

    pub fn load_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.options.load_timeout_ms = Some(timeout_ms);
        self
    }

    pub fn spawn_claude_code_process<S>(mut self, spawner: S) -> Self
    where
        S: ClaudeProcessSpawner + 'static,
    {
        self.options.spawn_claude_code_process = Some(SharedClaudeProcessSpawner::new(spawner));
        self
    }
}
