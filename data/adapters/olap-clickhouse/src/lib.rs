#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use shared_olap_client_kernel::{
    InsertBatch, KernelError, MaterializedViewSchema, OlapClient, Query, QuotaProfile, Row,
    TableSchema, TenantId,
};

fn ip_003_deferred(operation: &str) -> KernelError {
    KernelError::AdapterError(format!("clickhouse {operation}: IP-003 deferred"))
}

#[derive(Clone, Debug)]
pub struct ClickHouseConfig {
    pub url: String,
    pub user: String,
    /// data_class: INTERNAL_ONLY (secret at rest)
    pub password: String,
}

pub struct ClickHouseOlapClient {
    config: ClickHouseConfig,
}

impl ClickHouseOlapClient {
    #[must_use]
    pub fn new(config: ClickHouseConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.config.url
    }
}

impl OlapClient for ClickHouseOlapClient {
    fn ensure_tenant_database(&mut self, _tenant_id: &TenantId) -> Result<(), KernelError> {
        Err(ip_003_deferred("ensure_tenant_database"))
    }

    fn ensure_table(&mut self, _schema: &TableSchema) -> Result<(), KernelError> {
        Err(ip_003_deferred("ensure_table"))
    }

    fn ensure_materialized_view(
        &mut self,
        _schema: &MaterializedViewSchema,
    ) -> Result<(), KernelError> {
        Err(ip_003_deferred("ensure_materialized_view"))
    }

    fn apply_quota(&mut self, _profile: &QuotaProfile) -> Result<(), KernelError> {
        Err(ip_003_deferred("apply_quota"))
    }

    fn insert(&mut self, _batch: &InsertBatch) -> Result<u64, KernelError> {
        Err(ip_003_deferred("insert"))
    }

    fn query(&self, _caller: &TenantId, _query: &Query) -> Result<Vec<Row>, KernelError> {
        Err(ip_003_deferred("query"))
    }

    fn drop_tenant_database(&mut self, _tenant_id: &TenantId) -> Result<(), KernelError> {
        Err(ip_003_deferred("drop_tenant_database"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter() -> ClickHouseOlapClient {
        ClickHouseOlapClient::new(ClickHouseConfig {
            url: "http://clickhouse.test:8123".to_string(),
            user: "default".to_string(),
            password: "test".to_string(),
        })
    }

    #[test]
    fn unimplemented_surfaced_honestly_on_query() {
        let tid = TenantId::try_new("t1").unwrap();
        let table = shared_olap_client_kernel::QualifiedTable::new(
            tid.clone(),
            shared_olap_client_kernel::TableName::try_new("events").unwrap(),
        );
        let q = Query {
            source: table,
            columns: vec!["id".to_string()],
            aggregates: vec![],
            filter: None,
            group_by: vec![],
            order_by: vec![],
            limit: None,
        };
        let a = adapter();
        let err = a.query(&tid, &q).unwrap_err();
        match err {
            KernelError::AdapterError(msg) => assert!(msg.contains("IP-003")),
            other => panic!("expected AdapterError, got {other}"),
        }
    }

    #[test]
    fn ip_003_deferred_names_the_operation_and_the_slice() {
        assert!(matches!(
            ip_003_deferred("insert"),
            KernelError::AdapterError(msg) if msg == "clickhouse insert: IP-003 deferred"
        ));
    }

    #[test]
    fn unimplemented_surfaced_on_ensure_database() {
        let mut a = adapter();
        let tid = TenantId::try_new("t1").unwrap();
        let err = a.ensure_tenant_database(&tid).unwrap_err();
        assert!(matches!(err, KernelError::AdapterError(_)));
    }

    #[test]
    fn url_accessor() {
        let a = adapter();
        assert_eq!(a.url(), "http://clickhouse.test:8123");
    }
}
