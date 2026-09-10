#![forbid(unsafe_code)]

use data_analytics_tenant_bootstrap_app::TenantBootstrapController;
use shared_olap_clickhouse_adapter::{ClickHouseConfig, ClickHouseOlapClient};

fn main() {
    tracing_subscriber::fmt().json().init();

    let clickhouse_url =
        std::env::var("CLICKHOUSE_URL").unwrap_or_else(|_| "http://clickhouse:8123".to_string());
    let clickhouse_user =
        std::env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".to_string());
    let clickhouse_password =
        std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_else(|_| String::new());

    let mut adapter = ClickHouseOlapClient::new(ClickHouseConfig {
        url: clickhouse_url,
        user: clickhouse_user,
        password: clickhouse_password,
    });

    let _ctrl = TenantBootstrapController::new(&mut adapter);

    tracing::info!(
        target: "data_analytics_tenant_bootstrap_app::boot",
        "tenant bootstrap controller boot complete (Kafka consumer deferred: IP-002)"
    );
}
