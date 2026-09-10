use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shared_resource_provider_contract_kernel::{
    CancellationMetadata, CompensationMetadata, IdempotencyKey, Operation, OperationLedgerEntry,
    OperationPhase, OperationState as LedgerState, ProviderError, ResourceName, RetryPolicy,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Document {
    pub(super) title: String,
    pub(super) revision: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum AppliedWrite {
    Create {
        name: String,
        payload: Document,
    },
    Put {
        name: String,
        payload: Document,
    },
    Delete {
        name: String,
        operation_name: String,
    },
}

/// A pending operation completes after this many polls, so the harness
/// exercises the pending->done path.
pub(super) const POLLS_BEFORE_TERMINAL: u32 = 1;

#[derive(Debug, Clone)]
pub(super) enum ReferenceOperationState {
    Pending {
        remaining_polls: u32,
        target: ResourceName,
        ledger: OperationLedgerEntry,
    },
    Terminal(Operation),
}

#[derive(Debug, Default)]
pub(super) struct ReferenceProvider {
    pub(super) items: BTreeMap<String, Document>,
    pub(super) dedup_log: BTreeMap<String, AppliedWrite>,
    pub(super) operations: BTreeMap<String, ReferenceOperationState>,
    pub(super) operation_seq: u64,
}

impl ReferenceProvider {
    pub(super) fn resource_orn(name: &ResourceName) -> String {
        format!(
            "orn:oya:local-test:account-test:{}:{}/{}",
            name.collection(),
            name.collection(),
            name.resource_id()
        )
    }

    pub(super) fn delete_ledger_entry(
        operation_id: &str,
        idempotency_key: &IdempotencyKey,
        name: &ResourceName,
        state: LedgerState,
        observed_generation: u64,
        transition_sequence: u64,
    ) -> OperationLedgerEntry {
        OperationLedgerEntry {
            operation_id: operation_id.to_owned(),
            idempotency_key: idempotency_key.as_str().to_owned(),
            request_hash: format!("fixture-hash:delete:{name}"),
            resource_orn: Self::resource_orn(name),
            desired_generation: 2,
            observed_generation,
            state,
            phase: OperationPhase::OperationLedger,
            tenant_account_project: "tenant-test/account-test/project-test".to_owned(),
            region_cell: "local-test/cell-0001".to_owned(),
            principal: "principal:test-harness".to_owned(),
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
            transition_sequence,
        }
    }
}

impl ReferenceProvider {
    pub(super) fn snapshot_operation(
        &self,
        operation_name: &str,
    ) -> Result<Operation, ProviderError> {
        match self.operations.get(operation_name) {
            Some(ReferenceOperationState::Terminal(operation)) => Ok(operation.clone()),
            Some(ReferenceOperationState::Pending { ledger, .. }) => {
                Operation::pending(operation_name.to_owned(), ledger.clone()).map_err(|e| {
                    ProviderError::Internal {
                        message: e.to_string(),
                    }
                })
            }
            None => Err(ProviderError::NotFound {
                name: operation_name.to_owned(),
            }),
        }
    }
}
