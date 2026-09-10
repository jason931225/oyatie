#![forbid(unsafe_code)]

use data_analytics_app::{AnalyticsApp, AnalyticsConfig};

fn main() {
    tracing_subscriber::fmt().json().init();

    let config = AnalyticsConfig {
        listen_addr: std::env::var("ANALYTICS_LISTEN_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
        clickhouse_url: std::env::var("CLICKHOUSE_URL")
            .unwrap_or_else(|_| "http://clickhouse:8123".to_string()),
        clickhouse_user: std::env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".to_string()),
        clickhouse_password: std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_else(|_| String::new()),
        primary_tenant_id: std::env::var("ANALYTICS_PRIMARY_TENANT")
            .unwrap_or_else(|_| "platform".to_string()),
    };

    match AnalyticsApp::new(config) {
        Ok(app) => {
            tracing::info!(
                target: "data_analytics_app::boot",
                listen_addr = %app.listen_addr(),
                tenant = %app.primary_tenant_id(),
                "analytics service boot complete (HTTP listener deferred: IP-015)"
            );
        }
        Err(e) => {
            tracing::error!(target: "data_analytics_app::boot", error = %e, "boot failed");
            std::process::exit(1);
        }
    }
}
