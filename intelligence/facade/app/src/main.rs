//! intelligence-app binary entry point (ADR-0384 Path B).

use intelligence_app::{AppConfig, build_app};
use intelligence_rest::build_router;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = match AppConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "failed to read AppConfig from environment");
            std::process::exit(1);
        }
    };

    let listen_addr = config.listen_addr.clone();

    let state = match build_app(config) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "failed to build AppState");
            std::process::exit(1);
        }
    };

    let router = build_router(state);

    let listener = match tokio::net::TcpListener::bind(&listen_addr).await {
        Ok(l) => l,
        Err(e) => {
            error!(addr = %listen_addr, error = %e, "failed to bind TCP listener");
            std::process::exit(1);
        }
    };

    info!(addr = %listen_addr, "intelligence-app listening");

    if let Err(e) = axum::serve(listener, router).await {
        error!(error = %e, "axum serve error");
        std::process::exit(1);
    }
}
