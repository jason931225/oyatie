use super::*;

pub async fn serve(
    addr: SocketAddr,
    router: Arc<Router<SyncHandler>>,
    chain: Arc<MiddlewareChain<HttpRequest, HttpResponse>>,
    config: ServerConfig,
) -> Result<(), HyperRuntimeError> {
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|error| HyperRuntimeError::Bind(error.to_string()))?;
    serve_listener(listener, router, chain, config).await
}

/// The daemon-facing seam used by application composition roots that
/// own the bind decision outside this adapter. It keeps all Tokio/Hyper types
/// inside this crate while still allowing callers to perform pre-bind policy
/// checks with `std::net::TcpListener`.
pub async fn serve_listener(
    listener: TcpListener,
    router: Arc<Router<SyncHandler>>,
    chain: Arc<MiddlewareChain<HttpRequest, HttpResponse>>,
    config: ServerConfig,
) -> Result<(), HyperRuntimeError> {
    supervisor::run(
        listener,
        router,
        chain,
        config,
        ServingControl::new(ServingLimits::default()),
        None,
        None,
    )
    .await?
    .into_result()
}

/// A deterministic loopback harness for service-level integration
/// tests and local evidence capture. It does not replace `serve` for daemon
/// operation and does not create deployment, readiness, or production endpoint
/// evidence by itself.
pub async fn serve_one_connection(
    listener: TcpListener,
    router: Arc<Router<SyncHandler>>,
    chain: Arc<MiddlewareChain<HttpRequest, HttpResponse>>,
    config: ServerConfig,
) -> Result<(), HyperRuntimeError> {
    serve_n_connections(listener, router, chain, config, 1).await
}

/// For deterministic integration evidence only. It avoids unbounded
/// daemon lifetime in tests while exercising the same Hyper request parsing,
/// response serialization, router, middleware, and handler seam as `serve`.
pub async fn serve_n_connections(
    listener: TcpListener,
    router: Arc<Router<SyncHandler>>,
    chain: Arc<MiddlewareChain<HttpRequest, HttpResponse>>,
    config: ServerConfig,
    max_connections: usize,
) -> Result<(), HyperRuntimeError> {
    if max_connections == 0 {
        return Err(HyperRuntimeError::Config(
            "max_connections must be greater than zero".to_string(),
        ));
    }
    supervisor::run(
        listener,
        router,
        chain,
        config,
        ServingControl::new(ServingLimits::default()),
        Some(max_connections),
        None,
    )
    .await?
    .into_result()
}

/// Blocking wrapper for deterministic tests that need to bind a std listener
/// outside this crate without depending directly on tokio.
///
/// The std listener must already be bound (commonly to `127.0.0.1:0` in a
/// test). This helper converts it to a Tokio listener and serves one
/// connection on a private single-thread runtime.
pub fn serve_one_connection_on_std_listener(
    listener: StdTcpListener,
    router: Arc<Router<SyncHandler>>,
    chain: Arc<MiddlewareChain<HttpRequest, HttpResponse>>,
    config: ServerConfig,
) -> Result<(), HyperRuntimeError> {
    serve_n_connections_on_std_listener(listener, router, chain, config, 1)
}

/// Blocking wrapper for bounded local integration evidence without leaking
/// Tokio into downstream crates.
pub fn serve_n_connections_on_std_listener(
    listener: StdTcpListener,
    router: Arc<Router<SyncHandler>>,
    chain: Arc<MiddlewareChain<HttpRequest, HttpResponse>>,
    config: ServerConfig,
    max_connections: usize,
) -> Result<(), HyperRuntimeError> {
    supervisor::run_std(
        listener,
        router,
        chain,
        config,
        ServingControl::new(ServingLimits::default()),
        Some(max_connections),
        false,
    )?
    .into_result()
}

/// Blocking wrapper for daemon entrypoints that pre-bind a std listener while
/// keeping the async runtime dependency adapter-local.
pub fn serve_on_std_listener(
    listener: StdTcpListener,
    router: Arc<Router<SyncHandler>>,
    chain: Arc<MiddlewareChain<HttpRequest, HttpResponse>>,
    config: ServerConfig,
) -> Result<(), HyperRuntimeError> {
    serve_controlled_on_std_listener(
        listener,
        router,
        chain,
        config,
        ServingControl::new(ServingLimits::default()),
    )?
    .into_result()
}

pub fn serve_controlled_on_std_listener(
    listener: StdTcpListener,
    router: Arc<Router<SyncHandler>>,
    chain: Arc<MiddlewareChain<HttpRequest, HttpResponse>>,
    config: ServerConfig,
    control: ServingControl,
) -> Result<ServingReport, HyperRuntimeError> {
    supervisor::run_std(listener, router, chain, config, control, None, false)
}

/// Explicit executable opt-in; ordinary library serving installs no signal handlers.
pub fn serve_with_signals_on_std_listener(
    listener: StdTcpListener,
    router: Arc<Router<SyncHandler>>,
    chain: Arc<MiddlewareChain<HttpRequest, HttpResponse>>,
    config: ServerConfig,
    control: ServingControl,
) -> Result<ServingReport, HyperRuntimeError> {
    supervisor::run_std(listener, router, chain, config, control, None, true)
}
