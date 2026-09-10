//! Binary entrypoint. Every startup failure is a non-zero exit: there is no
//! degraded serve.

use iam_pdp_app::{PdpConfig, observability, server};
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
            Err(err) => {
                error!(error = %err, "SIGTERM handler failed; continuing to serve");
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

    let config = match PdpConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            error!(error = %err, "configuration rejected");
            std::process::exit(1);
        }
    };

    // An absent, empty, or malformed cert mount refuses the boot here rather
    // than downgrading the listener to plain TCP.
    let mut handle = match server::boot_from_config(&config).await {
        Ok(handle) => handle,
        Err(err) => {
            error!(error = %err, "boot refused (mTLS fail-closed)");
            std::process::exit(1);
        }
    };

    let drain = tokio::select! {
        () = shutdown_signal() => true,
        () = handle.done() => false,
    };
    if drain {
        info!("shutdown signal received; draining");
        handle.shutdown().await;
    } else {
        error!("a server task exited unexpectedly");
        std::process::exit(1);
    }
}
