#![forbid(unsafe_code)]

use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use iam_identity_workload_svid_operator_app::{OperatorConfig, SystemClock};
use iam_identity_workload_svid_operator_k8s::{
    KubeSvidOperatorRuntime, TrustdEcdsaIssuanceBackend,
};
use tracing::error;

/// The cluster join token that gates SVID issuance.
const ENV_JOIN_TOKEN: &str = "OYATIE_SVID_OPERATOR_JOIN_TOKEN";
const CA_TTL_SECS: u64 = 10 * 365 * 24 * 60 * 60;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(env_or("RUST_LOG", "info"))
        .init();

    let config = match OperatorConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            error!(error = %error, "iam svid-operator config startup validation failed");
            return ExitCode::FAILURE;
        }
    };

    let join_token = match std::env::var(ENV_JOIN_TOKEN) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => {
            error!(
                error = format!("{ENV_JOIN_TOKEN} must be set to a non-empty join token"),
                "iam svid-operator join-token startup validation failed"
            );
            return ExitCode::FAILURE;
        }
    };

    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(error) => {
            error!(error = %error, "system clock before unix epoch");
            return ExitCode::FAILURE;
        }
    };

    let backend = match TrustdEcdsaIssuanceBackend::bootstrap(
        "oyatie-cloud-iam-pdp-svid-ca",
        &join_token,
        now,
        CA_TTL_SECS,
    ) {
        Ok(backend) => backend,
        Err(error) => {
            error!(error = %error, "failed to bootstrap the SVID issuance CA");
            return ExitCode::FAILURE;
        }
    };

    let client = match kube::Client::try_default().await {
        Ok(client) => client,
        Err(error) => {
            error!(error = %error, "failed to build kube client");
            return ExitCode::FAILURE;
        }
    };

    let runtime =
        KubeSvidOperatorRuntime::new(client, config.desired, backend, SystemClock, config.backoff);
    runtime.run().await;
    ExitCode::SUCCESS
}

fn env_or(key: &str, default: &str) -> String {
    match std::env::var(key) {
        Ok(value) if !value.is_empty() => value,
        _ => default.to_owned(),
    }
}
