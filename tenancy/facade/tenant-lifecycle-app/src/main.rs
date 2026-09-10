use tenancy_tenant_lifecycle_app::serve;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(
                "tenancy_tenant_lifecycle=info"
                    .parse()
                    .expect("valid directive"),
            ),
        )
        .json()
        .init();

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    if let Err(e) = serve(&addr).await {
        tracing::error!(error = %e, "tenancy-tenant-lifecycle boot failed");
        std::process::exit(1);
    }
}
