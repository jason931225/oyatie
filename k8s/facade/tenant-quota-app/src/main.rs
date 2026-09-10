//! Binary entry point for the managed-K8s tenant quota service.

use k8s_tenant_quota_adapter_inmemory::InMemoryQuotaStore;
use k8s_tenant_quota_app::{BootError, QuotaAuthzProvider, serve};

const ENV_BEARER_TOKEN: &str = "K8S_TENANT_QUOTA_BEARER_TOKEN";
const BREAK_GLASS_PRINCIPAL_ID: &str = "wl_k8s_tenant_quota_operator";
const BREAK_GLASS_TENANT_ID: &str = "ten_platform";
const BREAK_GLASS_SCOPE: &str = "quota:platform:write";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("k8s_tenant_quota=info".parse().expect("valid directive")),
        )
        .json()
        .init();

    let authz = match build_authz() {
        Ok(authz) => authz,
        Err(e) => {
            tracing::error!(error = %e, "managed-k8s-tenant-quota boot refused");
            std::process::exit(1);
        }
    };

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let store = InMemoryQuotaStore::new();

    if let Err(e) = serve(&addr, store, authz).await {
        tracing::error!(error = %e, "managed-k8s-tenant-quota boot failed");
        std::process::exit(1);
    }
}

fn build_authz() -> Result<QuotaAuthzProvider, BootError> {
    let bearer = std::env::var(ENV_BEARER_TOKEN).unwrap_or_default();
    let authz = QuotaAuthzProvider::from_bearer_secret(
        bearer,
        BREAK_GLASS_PRINCIPAL_ID,
        BREAK_GLASS_TENANT_ID,
        vec![BREAK_GLASS_SCOPE.to_string()],
    )?;
    Ok(authz)
}
