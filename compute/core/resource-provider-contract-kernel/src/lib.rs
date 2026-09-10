#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

pub mod conformance;

mod error;
mod identity;
mod operation;
mod pagination;
mod provider;

pub use error::ContractShapeError;
pub use identity::{IdempotencyKey, ResourceName};
pub use operation::{
    ALLOWED_RETRY_CLASSIFICATIONS, CancellationMetadata, CompensationMetadata,
    OPERATION_NAME_PREFIX, Operation, OperationError, OperationLedgerEntry, OperationPhase,
    OperationResult, OperationState, RetryPolicy,
};
pub use pagination::{ListEntry, MAX_PAGE_SIZE, Page, PageRequest, PageToken};
pub use provider::{
    CreateOutcome, ProviderError, ProviderFuture, PutOutcome, ResourceProvider, WriteDisposition,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn operation_ledger(operation_id: &str, state: OperationState) -> OperationLedgerEntry {
        OperationLedgerEntry {
            operation_id: operation_id.to_owned(),
            idempotency_key: "00000000-0000-4000-8000-000000000001".to_owned(),
            request_hash: format!("fixture-hash:{operation_id}"),
            resource_orn: "orn:oya:local-test:account-test:documents/documents/doc-1".to_owned(),
            desired_generation: 2,
            observed_generation: if state.is_terminal() { 2 } else { 1 },
            state,
            phase: OperationPhase::OperationLedger,
            tenant_account_project: "tenant-test/account-test/project-test".to_owned(),
            region_cell: "local-test/cell-0001".to_owned(),
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
    fn idempotency_key_accepts_and_normalizes_canonical_uuids() {
        let key = IdempotencyKey::new("00000000-0000-4000-8000-00000000002A").unwrap();
        assert_eq!(key.as_str(), "00000000-0000-4000-8000-00000000002a");
    }

    #[test]
    fn idempotency_key_rejects_non_uuid_shapes() {
        for (bad, rejected_because) in [
            ("", "empty"),
            ("not-a-uuid", "not uuid shaped"),
            ("00000000-0000-4000-8000-00000000002", "too short"),
            ("00000000-0000-4000-8000-0000000000021", "too long"),
            ("00000000000040008000000000000020abcd", "no dashes"),
            ("zzzzzzzz-0000-4000-8000-00000000002a", "non-hex"),
        ] {
            assert!(
                IdempotencyKey::new(bad).is_err(),
                "{rejected_because}: {bad:?}"
            );
        }
    }

    #[test]
    fn resource_name_round_trips_through_aip_122_string_form() {
        let name = ResourceName::new("documents", "doc-1").unwrap();
        assert_eq!(name.to_string(), "documents/doc-1");
        let parsed: ResourceName = serde_json::from_str("\"documents/doc-1\"").unwrap();
        assert_eq!(parsed, name);
        assert_eq!(serde_json::to_string(&name).unwrap(), "\"documents/doc-1\"");
    }

    #[test]
    fn resource_name_rejects_malformed_forms() {
        assert!(ResourceName::new("Docs", "doc-1").is_err());
        assert!(ResourceName::new("documents", "").is_err());
        for bad in ["documents", "documents/a/b", "/doc-1", "documents/"] {
            assert!(
                serde_json::from_str::<ResourceName>(&format!("{bad:?}")).is_err(),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn page_request_bounds_are_enforced() {
        assert!(PageRequest::first(0).is_err());
        assert!(PageRequest::first(MAX_PAGE_SIZE + 1).is_err());
        let first = PageRequest::first(3).unwrap();
        let token = PageToken::new("cursor-1").unwrap();
        let next = first.after(token.clone());
        assert_eq!(next.page_size, 3);
        assert_eq!(next.page_token, Some(token));
        assert!(PageToken::new("").is_err());
    }

    #[test]
    fn operation_constructors_enforce_done_result_coupling() {
        let pending_ledger = operation_ledger("op-1", OperationState::Running);
        let pending = Operation::pending("operations/op-1", pending_ledger.clone()).unwrap();
        assert!(!pending.done);
        assert!(pending.result.is_none());
        assert_eq!(pending.metadata, pending_ledger);
        pending.validate().unwrap();

        let ok_ledger = operation_ledger("op-1", OperationState::Succeeded);
        let ok = Operation::succeeded("operations/op-1", ok_ledger, serde_json::json!({})).unwrap();
        assert!(ok.done);
        ok.validate().unwrap();

        let failed_ledger = operation_ledger("op-2", OperationState::Failed);
        let failed = Operation::failed(
            "operations/op-2",
            failed_ledger,
            OperationError {
                code: "failed_precondition".to_owned(),
                message: "resource busy".to_owned(),
            },
        )
        .unwrap();
        assert!(matches!(failed.result, Some(OperationResult::Error(_))));

        assert!(
            Operation::pending(
                "op-without-prefix",
                operation_ledger("op-without-prefix", OperationState::Running)
            )
            .is_err()
        );
        assert!(
            Operation::pending("operations/", operation_ledger("", OperationState::Running))
                .is_err()
        );

        let forged = Operation {
            name: "operations/op-3".to_owned(),
            done: true,
            metadata: operation_ledger("op-3", OperationState::Running),
            result: None,
        };
        assert!(forged.validate().is_err());
    }

    #[test]
    fn operation_ledger_rejects_unknown_retry_classification() {
        let mut ledger = operation_ledger("op-1", OperationState::Running);
        ledger.retry_policy.retry_classification = "eventually".to_owned();
        let error = ledger.validate().unwrap_err();
        assert!(
            error
                .to_string()
                .contains("retry_policy.retry_classification"),
            "{error}"
        );
    }
}
