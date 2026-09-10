#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use core::future::Future;
use core::pin::Pin;
use std::fmt;

use serde::{Deserialize, Serialize};
use shared_platform_contracts_kernel::tenancy::{Tenant, TenantLifecycleOperation};
use shared_resource_provider_contract_kernel::Operation;

/// AIP-155 request ids / AWS client tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppliedWriteRecord {
    Create {
        name: String,
        tenant: Tenant,
    },
    Put {
        name: String,
        tenant: Tenant,
    },
    Lifecycle {
        name: String,
        operation: TenantLifecycleOperation,
        operation_name: String,
    },
}

/// One entry in the AIP-151 operation ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationRecord {
    pub operation: Operation,
    pub kind: TenantLifecycleOperation,
    pub target: String, // data_class: TENANT_SCOPED
}

/// Storage-port failures. The port is infallible on semantics (absence is
/// `Ok(None)`); errors are availability/integrity only, so the usecase layer
/// can map them to `internal` without guessing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Unavailable { detail: String },
    Corrupt { detail: String },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable { detail } => write!(f, "store unavailable: {detail}"),
            Self::Corrupt { detail } => write!(f, "store record corrupt: {detail}"),
        }
    }
}

impl std::error::Error for StoreError {}

pub type TenantScanFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<(String, Tenant)>, StoreError>> + Send + 'a>>;

/// The lifecycle control plane's storage port: ordered keyed records with
/// point get/put/remove and an ordered range scan (the owned data
/// shape). Async (the durable backend performs real I/O) but IO-free at this
/// layer; adapters own transport. Async is modelled with a return-position
/// boxed future — `core::future::Future` + `core::pin::Pin` + `Box::pin` — so
/// the kernel takes no `async-trait` / `futures` dependency
/// (kernel-purity gate, ADR-0547; ADR-0376 rejects async-trait for ports).
pub trait TenantLifecycleStore {
    fn get_tenant<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Tenant>, StoreError>> + Send + 'a>>;

    fn put_tenant<'a>(
        &'a mut self,
        name: &'a str,
        tenant: &'a Tenant,
    ) -> Pin<Box<dyn Future<Output = Result<(), StoreError>> + Send + 'a>>;

    /// No-op when `name` is absent.
    fn remove_tenant<'a>(
        &'a mut self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), StoreError>> + Send + 'a>>;

    /// Ordered scan of tenant records whose key starts with `prefix`,
    /// beginning at `start_at` (inclusive) when given, yielding at most
    /// `limit` entries in ascending key order. The order MUST be a stable
    /// total order over keys (AIP-158 pagination is built on it).
    fn scan_tenants<'a>(
        &'a self,
        prefix: &'a str,
        start_at: Option<&'a str>,
        limit: u32,
    ) -> TenantScanFuture<'a>;

    // S-B: tenant_id (RLS scope) is threaded through the idempotency + operation
    // ledger methods below so the durable sqlx adapter can set the per-transaction
    // tenant GUC (`oyatie.tenant_id`) before any tenant-scoped statement. The
    // applied/operation tables are tenant-scoped (RLS RESTRICTIVE per tenant_id);
    // the in-memory adapter ignores the scope (its maps are process-local) but
    // accepts it so both adapters share one port.

    fn get_applied<'a>(
        &'a self,
        tenant_id: &'a str,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<AppliedWriteRecord>, StoreError>> + Send + 'a>>;

    fn put_applied<'a>(
        &'a mut self,
        tenant_id: &'a str,
        key: &'a str,
        record: &'a AppliedWriteRecord,
    ) -> Pin<Box<dyn Future<Output = Result<(), StoreError>> + Send + 'a>>;

    fn get_operation<'a>(
        &'a self,
        tenant_id: &'a str,
        operation_name: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<OperationRecord>, StoreError>> + Send + 'a>>;

    /// Write a ledger entry within `tenant_id`. Callers MUST never overwrite a
    /// terminal entry (the usecase layer enforces immutability before calling
    /// this).
    fn put_operation<'a>(
        &'a mut self,
        tenant_id: &'a str,
        operation_name: &'a str,
        record: &'a OperationRecord,
    ) -> Pin<Box<dyn Future<Output = Result<(), StoreError>> + Send + 'a>>;

    /// Next monotonic ledger ordinal for `tenant_id`, used to mint unique
    /// operation names. The ordinal need only be unique within the tenant.
    fn next_operation_seq<'a>(
        &'a mut self,
        tenant_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<u64, StoreError>> + Send + 'a>>;
}

#[cfg(test)]
mod tests {
    use shared_platform_contracts_kernel::tenancy::{IsolationPosture, TenantLifecycleState};
    use shared_resource_provider_contract_kernel::{
        CancellationMetadata, CompensationMetadata, OperationLedgerEntry, OperationPhase,
        OperationState, RetryPolicy,
    };

    use super::*;
    fn operation_ledger_entry(operation_id: &str) -> OperationLedgerEntry {
        OperationLedgerEntry {
            operation_id: operation_id.to_owned(),
            idempotency_key: "00000000-0000-4000-8000-000000000001".to_owned(),
            request_hash: format!("fixture-hash:{operation_id}"),
            resource_orn: "orn:oya:tenancy:acme:tenants/acme".to_owned(),
            desired_generation: 1,
            observed_generation: 0,
            state: OperationState::Accepted,
            phase: OperationPhase::OperationLedger,
            tenant_account_project: "tenant/acme".to_owned(),
            region_cell: "control-plane/default".to_owned(),
            principal: "principal:test".to_owned(),
            audit_chain_id: format!("audit-chain/{operation_id}"),
            retry_policy: RetryPolicy {
                backoff: "bounded-exponential-jitter".to_owned(),
                max_attempts: 3,
                retry_classification: "transient".to_owned(),
            },
            cancellation: CancellationMetadata {
                cancel_safe: true,
                audit_required: true,
            },
            compensation: CompensationMetadata {
                required: false,
                strategy: "none".to_owned(),
            },
            transition_sequence: 1,
        }
    }

    #[test]
    fn applied_write_record_round_trips() {
        let record = AppliedWriteRecord::Create {
            name: "tenants/acme".to_owned(),
            tenant: Tenant {
                tenant_id: "acme".to_owned(),
                display_name: "Acme Corp".to_owned(),
                state: TenantLifecycleState::initial(),
                isolation_posture: IsolationPosture::Pooled,
                cell_id: "cell-001".to_owned(),
                residency_zone: None,
            },
        };
        let json = serde_json::to_string(&record).unwrap();
        assert_eq!(
            serde_json::from_str::<AppliedWriteRecord>(&json).unwrap(),
            record
        );
    }

    #[test]
    fn operation_record_round_trips_and_rejects_unknown_fields() {
        let record = OperationRecord {
            operation: Operation::pending(
                "operations/acme-lifecycle-000001",
                operation_ledger_entry("acme-lifecycle-000001"),
            )
            .unwrap(),
            kind: TenantLifecycleOperation::Activate,
            target: "tenants/acme".to_owned(),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert_eq!(
            serde_json::from_str::<OperationRecord>(&json).unwrap(),
            record
        );
        let mut value = serde_json::to_value(&record).unwrap();
        value["surprise"] = serde_json::json!(true);
        assert!(serde_json::from_value::<OperationRecord>(value).is_err());
    }
}
