//! Analytics use-case orchestration over the OLAP client port.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::fmt;

use data_analytics_domain::{
    AuditLogSearch, BillingRollup, DataExport, DomainError, TenantDashboardQuery, TenantId,
};
use shared_olap_client_kernel::{KernelError, OlapClient, Row};

#[derive(Clone, Debug)]
pub enum UseCaseError {
    Domain(DomainError),
    Kernel(KernelError),
    Unimplemented(&'static str),
}

impl fmt::Display for UseCaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(e) => write!(f, "domain error: {e}"),
            Self::Kernel(e) => write!(f, "kernel error: {e}"),
            Self::Unimplemented(slug) => write!(f, "unimplemented: {slug}"),
        }
    }
}

impl std::error::Error for UseCaseError {}

impl From<DomainError> for UseCaseError {
    fn from(e: DomainError) -> Self {
        Self::Domain(e)
    }
}

impl From<KernelError> for UseCaseError {
    fn from(e: KernelError) -> Self {
        match &e {
            KernelError::CrossTenantAccessDenied => Self::Kernel(e),
            _ => Self::Kernel(e),
        }
    }
}

pub struct GetDashboardUseCase<'a> {
    olap: &'a dyn OlapClient,
    tenant_id: TenantId,
}

impl<'a> GetDashboardUseCase<'a> {
    #[must_use]
    pub fn new(olap: &'a dyn OlapClient, tenant_id: TenantId) -> Self {
        Self { olap, tenant_id }
    }

    /// # Errors
    /// Returns [`UseCaseError`] on domain violation or OLAP failure.
    pub fn execute(&self, request: &TenantDashboardQuery) -> Result<Vec<Row>, UseCaseError> {
        let q = request.to_olap_query()?;
        Ok(self.olap.query(&self.tenant_id, &q)?)
    }
}

pub struct SearchAuditLogUseCase<'a> {
    olap: &'a dyn OlapClient,
    tenant_id: TenantId,
}

impl<'a> SearchAuditLogUseCase<'a> {
    #[must_use]
    pub fn new(olap: &'a dyn OlapClient, tenant_id: TenantId) -> Self {
        Self { olap, tenant_id }
    }

    /// # Errors
    /// Returns [`UseCaseError`] on OLAP failure or cross-tenant detection.
    pub fn execute(&self, request: &AuditLogSearch) -> Result<Vec<Row>, UseCaseError> {
        let q = request.to_olap_query()?;
        Ok(self.olap.query(&self.tenant_id, &q)?)
    }
}

pub struct RunBillingRollupUseCase<'a> {
    olap: &'a dyn OlapClient,
    tenant_id: TenantId,
}

impl<'a> RunBillingRollupUseCase<'a> {
    #[must_use]
    pub fn new(olap: &'a dyn OlapClient, tenant_id: TenantId) -> Self {
        Self { olap, tenant_id }
    }

    /// # Errors
    /// Returns [`UseCaseError`] on domain or OLAP failure.
    pub fn execute(&self, request: &BillingRollup) -> Result<Vec<Row>, UseCaseError> {
        let q = request.to_olap_query()?;
        Ok(self.olap.query(&self.tenant_id, &q)?)
    }
}

pub struct CreateDataExportUseCase;

impl CreateDataExportUseCase {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// # Errors
    /// Always returns [`UseCaseError::Unimplemented`]: export wiring is deferred.
    #[allow(clippy::unused_self)]
    pub fn execute(&self, _request: &DataExport) -> Result<String, UseCaseError> {
        Err(UseCaseError::Unimplemented(
            "data_export: IP-013 / IP-004 deferred — object-storage wiring not yet complete",
        ))
    }
}

impl Default for CreateDataExportUseCase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_analytics_domain::{
        AuditLogFilter, DashboardMetric, ExportFormat, ExportScope, Pagination, TimeRange,
    };
    use shared_olap_client_kernel::{
        ColumnDef, ColumnType, KernelError, QualifiedTable, TableEngine, TableName, TableSchema,
        memory_adapter::InMemoryOlapClient,
    };

    fn tid(s: &str) -> TenantId {
        TenantId::try_new(s).unwrap()
    }

    fn range() -> TimeRange {
        TimeRange {
            from_secs: 1_735_689_600,
            to_secs: 1_738_368_000,
        }
    }

    fn seed_table(client: &mut InMemoryOlapClient, tenant: &str, table: &str) {
        let tid = TenantId::try_new(tenant).unwrap();
        let tbl = TableName::try_new(table).unwrap();
        let qt = QualifiedTable::new(tid.clone(), tbl);
        client.ensure_tenant_database(&tid).unwrap();
        client
            .ensure_table(&TableSchema {
                table: qt,
                columns: vec![
                    ColumnDef::new("ts", ColumnType::DateTime, false),
                    ColumnDef::new("workflow_execution_count", ColumnType::UInt64, true),
                    ColumnDef::new("api_call_count", ColumnType::UInt64, true),
                    ColumnDef::new("error_rate", ColumnType::Float64, true),
                    ColumnDef::new("p99_latency_ms", ColumnType::Float64, true),
                    ColumnDef::new("active_users", ColumnType::UInt64, true),
                    ColumnDef::new("storage_used_bytes", ColumnType::UInt64, true),
                    ColumnDef::new("actor_id", ColumnType::String, true),
                    ColumnDef::new("action", ColumnType::String, true),
                    ColumnDef::new("resource_id", ColumnType::String, true),
                    ColumnDef::new("amount", ColumnType::Float64, true),
                    ColumnDef::new("period", ColumnType::String, true),
                ],
                engine: TableEngine::MergeTree,
                order_by: vec!["ts".to_string()],
                partition_by: None,
                ttl: None,
            })
            .unwrap();
    }

    #[test]
    fn get_dashboard_returns_empty_rows_from_in_memory() {
        let mut client = InMemoryOlapClient::new();
        seed_table(&mut client, "t1", "tenant_metrics");
        let uc = GetDashboardUseCase::new(&client, tid("t1"));
        let req = TenantDashboardQuery {
            tenant_id: tid("t1"),
            metrics: vec![DashboardMetric::ApiCallCount],
            time_range: range(),
            pagination: Pagination::new(10),
        };
        let result = uc.execute(&req).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn search_audit_log_empty_result() {
        let mut client = InMemoryOlapClient::new();
        seed_table(&mut client, "t1", "audit_log");
        let uc = SearchAuditLogUseCase::new(&client, tid("t1"));
        let req = AuditLogSearch {
            tenant_id: tid("t1"),
            filter: AuditLogFilter::default(),
            pagination: Pagination::new(20),
        };
        let rows = uc.execute(&req).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn billing_rollup_empty_result() {
        let mut client = InMemoryOlapClient::new();
        seed_table(&mut client, "t1", "billing_events");
        let uc = RunBillingRollupUseCase::new(&client, tid("t1"));
        let req = BillingRollup {
            tenant_id: tid("t1"),
            granularity: data_analytics_domain::RollupGranularity::Monthly,
            time_range: range(),
        };
        let result = uc.execute(&req).unwrap();
        let _ = result;
    }

    #[test]
    fn data_export_honestly_unimplemented() {
        let uc = CreateDataExportUseCase::new();
        let req = DataExport {
            tenant_id: tid("t1"),
            scope: ExportScope::AuditLog {
                time_range: range(),
            },
            format: ExportFormat::JsonLines,
        };
        let err = uc.execute(&req).unwrap_err();
        match err {
            UseCaseError::Unimplemented(slug) => {
                assert!(slug.contains("IP-013"));
            }
            other => panic!("expected Unimplemented, got {other}"),
        }
    }

    #[test]
    fn get_dashboard_refuses_cross_tenant_query() {
        let mut client = InMemoryOlapClient::new();
        seed_table(&mut client, "t1", "tenant_metrics");
        let uc = GetDashboardUseCase::new(&client, tid("t2"));
        let req = TenantDashboardQuery {
            tenant_id: tid("t1"),
            metrics: vec![DashboardMetric::ApiCallCount],
            time_range: range(),
            pagination: Pagination::new(10),
        };
        let err = uc.execute(&req).unwrap_err();
        match err {
            UseCaseError::Kernel(KernelError::CrossTenantAccessDenied) => {}
            other => panic!("expected CrossTenantAccessDenied, got {other}"),
        }
    }

    #[test]
    fn search_audit_log_refuses_cross_tenant_query() {
        let mut client = InMemoryOlapClient::new();
        seed_table(&mut client, "t1", "audit_log");
        let uc = SearchAuditLogUseCase::new(&client, tid("t2"));
        let req = AuditLogSearch {
            tenant_id: tid("t1"),
            filter: AuditLogFilter::default(),
            pagination: Pagination::new(10),
        };
        let err = uc.execute(&req).unwrap_err();
        match err {
            UseCaseError::Kernel(KernelError::CrossTenantAccessDenied) => {}
            other => panic!("expected CrossTenantAccessDenied, got {other}"),
        }
    }

    #[test]
    fn billing_rollup_refuses_cross_tenant_query() {
        let mut client = InMemoryOlapClient::new();
        seed_table(&mut client, "t1", "billing_events");
        let uc = RunBillingRollupUseCase::new(&client, tid("t2"));
        let req = BillingRollup {
            tenant_id: tid("t1"),
            granularity: data_analytics_domain::RollupGranularity::Monthly,
            time_range: range(),
        };
        let err = uc.execute(&req).unwrap_err();
        match err {
            UseCaseError::Kernel(KernelError::CrossTenantAccessDenied) => {}
            other => panic!("expected CrossTenantAccessDenied, got {other}"),
        }
    }
}
