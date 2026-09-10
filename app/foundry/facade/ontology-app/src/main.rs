use foundry_ontology_app::{Config, compose, observability, router};
use tracing::{error, info};

async fn shutdown_signal() {
    let ctrl_c = async {
        if tokio::signal::ctrl_c().await.is_err() {
            error!("ctrl-c handler failed; continuing to serve");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(error) => {
                error!(error = %error, "SIGTERM handler failed; continuing to serve");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}

#[tokio::main]
async fn main() {
    observability::init();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            error!(error = %error, "configuration rejected");
            std::process::exit(1);
        }
    };
    let listen_addr = config.listen_addr.clone();

    let state = match compose(&config) {
        Ok(state) => state,
        Err(error) => {
            error!(error = %error, "boot refused");
            std::process::exit(1);
        }
    };
    let seen = foundry_ontology_app::observation::observe(&state);
    info!(
        tenants = state.tenant_count(),
        poisoned = seen.poisoned,
        // Reported beside the count so a zero is never read as "none" when
        // it is really "not sampled": at boot nothing is contended, so a
        // non-zero here is itself a finding.
        unsampled_tenants = seen.contended + seen.unreadable,
        "composed"
    );

    let listener = match tokio::net::TcpListener::bind(&listen_addr).await {
        Ok(listener) => listener,
        Err(error) => {
            error!(error = %error, address = %listen_addr, "the listener could not bind");
            std::process::exit(1);
        }
    };
    info!(address = %listen_addr, "serving");

    if let Err(error) = axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        error!(error = %error, "the server exited unexpectedly");
        std::process::exit(1);
    }
    info!("drained");
}
