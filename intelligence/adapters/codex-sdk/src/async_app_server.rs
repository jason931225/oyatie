//! Optional Tokio-backed async wrappers for the app-server API.
//!
//! The transport is blocking stdio; async methods offload it with `tokio::task::spawn_blocking`.

use serde_json::Value;

use crate::app_server::{
    AppCodex, AppLoginHandle, AppRunInput, AppServerConfig, AppThread, AppTurnHandle,
    AppTurnResult, InitializeResponse, Notification,
};
use crate::error::{CodexError, Result};

async fn blocking_result<T, F>(operation: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|err| CodexError::Protocol(format!("async app-server task failed: {err}")))?
}

/// Async counterpart to `AppCodex` using Tokio's blocking-task pool.
#[derive(Clone)]
pub struct AsyncAppCodex {
    inner: AppCodex,
}

impl AsyncAppCodex {
    /// Start and initialize `codex app-server --listen stdio://` asynchronously.
    pub async fn new(config: AppServerConfig) -> Result<Self> {
        let inner = blocking_result(move || AppCodex::new(config)).await?;
        Ok(Self { inner })
    }

    pub fn metadata(&self) -> &InitializeResponse {
        self.inner.metadata()
    }

    /// Close the app-server process on Tokio's blocking-task pool.
    pub async fn close(&self) {
        let inner = self.inner.clone();
        let _ = tokio::task::spawn_blocking(move || inner.close()).await;
    }

    pub async fn login_api_key(&self, api_key: impl Into<String>) -> Result<()> {
        let inner = self.inner.clone();
        let api_key = api_key.into();
        blocking_result(move || inner.login_api_key(api_key)).await
    }

    /// Start browser-based ChatGPT login and return a routed async login handle.
    pub async fn login_chatgpt(&self) -> Result<AsyncAppLoginHandle> {
        let inner = self.inner.clone();
        let handle = blocking_result(move || inner.login_chatgpt()).await?;
        Ok(AsyncAppLoginHandle { inner: handle })
    }

    pub async fn login_chatgpt_device_code(&self) -> Result<AsyncAppLoginHandle> {
        let inner = self.inner.clone();
        let handle = blocking_result(move || inner.login_chatgpt_device_code()).await?;
        Ok(AsyncAppLoginHandle { inner: handle })
    }

    pub async fn account(&self, refresh_token: bool) -> Result<Value> {
        let inner = self.inner.clone();
        blocking_result(move || inner.account(refresh_token)).await
    }

    pub async fn logout(&self) -> Result<()> {
        let inner = self.inner.clone();
        blocking_result(move || inner.logout()).await
    }

    pub async fn thread_start(&self, params: Option<Value>) -> Result<AsyncAppThread> {
        let inner = self.inner.clone();
        let thread = blocking_result(move || inner.thread_start(params)).await?;
        Ok(AsyncAppThread { inner: thread })
    }

    pub async fn thread_list(&self, params: Option<Value>) -> Result<Value> {
        let inner = self.inner.clone();
        blocking_result(move || inner.thread_list(params)).await
    }

    pub async fn thread_resume(
        &self,
        thread_id: impl Into<String>,
        params: Option<Value>,
    ) -> Result<AsyncAppThread> {
        let inner = self.inner.clone();
        let thread_id = thread_id.into();
        let thread = blocking_result(move || inner.thread_resume(thread_id, params)).await?;
        Ok(AsyncAppThread { inner: thread })
    }

    pub async fn thread_fork(
        &self,
        thread_id: impl Into<String>,
        params: Option<Value>,
    ) -> Result<AsyncAppThread> {
        let inner = self.inner.clone();
        let thread_id = thread_id.into();
        let thread = blocking_result(move || inner.thread_fork(thread_id, params)).await?;
        Ok(AsyncAppThread { inner: thread })
    }

    pub async fn thread_archive(&self, thread_id: impl Into<String>) -> Result<Value> {
        let inner = self.inner.clone();
        let thread_id = thread_id.into();
        blocking_result(move || inner.thread_archive(thread_id)).await
    }

    pub async fn thread_unarchive(&self, thread_id: impl Into<String>) -> Result<AsyncAppThread> {
        let inner = self.inner.clone();
        let thread_id = thread_id.into();
        let thread = blocking_result(move || inner.thread_unarchive(thread_id)).await?;
        Ok(AsyncAppThread { inner: thread })
    }

    pub async fn models(&self, include_hidden: bool) -> Result<Value> {
        let inner = self.inner.clone();
        blocking_result(move || inner.models(include_hidden)).await
    }
}

/// Async routed interactive-login handle.
#[derive(Clone)]
pub struct AsyncAppLoginHandle {
    inner: AppLoginHandle,
}

impl AsyncAppLoginHandle {
    pub fn login_id(&self) -> &str {
        self.inner.login_id()
    }

    pub fn auth_url(&self) -> Option<&str> {
        self.inner.auth_url()
    }

    pub fn verification_url(&self) -> Option<&str> {
        self.inner.verification_url()
    }

    pub fn user_code(&self) -> Option<&str> {
        self.inner.user_code()
    }

    /// Wait for the completion notification for this login attempt.
    pub async fn wait(&self) -> Result<Notification> {
        let inner = self.inner.clone();
        blocking_result(move || inner.wait()).await
    }

    pub async fn cancel(&self) -> Result<Value> {
        let inner = self.inner.clone();
        blocking_result(move || inner.cancel()).await
    }
}

/// Async high-level thread handle backed by app-server JSON-RPC.
#[derive(Clone)]
pub struct AsyncAppThread {
    inner: AppThread,
}

impl AsyncAppThread {
    pub fn id(&self) -> &str {
        self.inner.id()
    }

    /// Start a turn, consume routed notifications until completion, and return a result summary.
    pub async fn run<I>(&self, input: I, params: Option<Value>) -> Result<AppTurnResult>
    where
        I: Into<AppRunInput> + Send + 'static,
    {
        let inner = self.inner.clone();
        let input = input.into();
        blocking_result(move || inner.run(input, params)).await
    }

    /// Start a turn and return a low-level async turn handle for streaming/control.
    pub async fn turn<I>(&self, input: I, params: Option<Value>) -> Result<AsyncAppTurnHandle>
    where
        I: Into<AppRunInput> + Send + 'static,
    {
        let inner = self.inner.clone();
        let input = input.into();
        let handle = blocking_result(move || inner.turn(input, params)).await?;
        Ok(AsyncAppTurnHandle { inner: handle })
    }

    pub async fn read(&self, include_turns: bool) -> Result<Value> {
        let inner = self.inner.clone();
        blocking_result(move || inner.read(include_turns)).await
    }

    pub async fn set_name(&self, name: impl Into<String>) -> Result<Value> {
        let inner = self.inner.clone();
        let name = name.into();
        blocking_result(move || inner.set_name(name)).await
    }

    pub async fn compact(&self) -> Result<Value> {
        let inner = self.inner.clone();
        blocking_result(move || inner.compact()).await
    }
}

/// Async low-level turn handle for routed streaming, steering, interruption, and collection.
#[derive(Clone)]
pub struct AsyncAppTurnHandle {
    inner: AppTurnHandle,
}

impl AsyncAppTurnHandle {
    pub fn id(&self) -> &str {
        self.inner.id()
    }

    pub fn thread_id(&self) -> &str {
        self.inner.thread_id()
    }

    pub async fn steer<I>(&self, input: I) -> Result<Value>
    where
        I: Into<AppRunInput> + Send + 'static,
    {
        let inner = self.inner.clone();
        let input = input.into();
        blocking_result(move || inner.steer(input)).await
    }

    pub async fn interrupt(&self) -> Result<Value> {
        let inner = self.inner.clone();
        blocking_result(move || inner.interrupt()).await
    }

    pub fn stream(&self) -> AsyncAppTurnStream {
        AsyncAppTurnStream {
            inner: self.inner.clone(),
            done: false,
        }
    }

    /// Consume this turn's stream and summarize the completed turn.
    pub async fn run(&self) -> Result<AppTurnResult> {
        let inner = self.inner.clone();
        blocking_result(move || inner.run()).await
    }
}

/// Async helper with a `next().await` method over routed turn notifications.
#[derive(Clone)]
pub struct AsyncAppTurnStream {
    inner: AppTurnHandle,
    done: bool,
}

impl AsyncAppTurnStream {
    pub async fn next(&mut self) -> Option<Result<Notification>> {
        if self.done {
            return None;
        }
        let inner = self.inner.clone();
        let next = tokio::task::spawn_blocking(move || inner.stream().next())
            .await
            .map_err(|err| CodexError::Protocol(format!("async app-server task failed: {err}")))
            .unwrap_or_else(|err| Some(Err(err)));

        match next {
            Some(Ok(notification)) => {
                if notification.method == "turn/completed" {
                    self.done = true;
                }
                Some(Ok(notification))
            }
            Some(Err(err)) => {
                self.done = true;
                Some(Err(err))
            }
            None => {
                self.done = true;
                None
            }
        }
    }
}
