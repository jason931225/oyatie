use shared_resource_provider_contract_kernel::{
    IdempotencyKey, Operation, OperationLedgerEntry, OperationState as LedgerState, ProviderError,
    ProviderFuture, ResourceName,
};

use super::support::{
    AppliedWrite, POLLS_BEFORE_TERMINAL, ReferenceOperationState, ReferenceProvider,
};

pub(super) fn delete<'a>(
    provider: &'a mut ReferenceProvider,
    name: &'a ResourceName,
    idempotency_key: &'a IdempotencyKey,
) -> ProviderFuture<'a, Operation> {
    Box::pin(async move {
        let key = idempotency_key.as_str().to_owned();
        if let Some(applied) = provider.dedup_log.get(&key) {
            return match applied {
                AppliedWrite::Delete {
                    name: n,
                    operation_name,
                } if *n == name.to_string() => provider.snapshot_operation(operation_name),
                _ => Err(ProviderError::IdempotencyKeyReuse { key }),
            };
        }
        if !provider.items.contains_key(&name.to_string()) {
            return Err(ProviderError::NotFound {
                name: name.to_string(),
            });
        }
        provider.operation_seq += 1;
        let operation_id = format!("delete-{:06}", provider.operation_seq);
        let operation_name = format!("operations/{operation_id}");
        let ledger = ReferenceProvider::delete_ledger_entry(
            &operation_id,
            idempotency_key,
            name,
            LedgerState::Running,
            1,
            1,
        );
        provider.operations.insert(
            operation_name.clone(),
            ReferenceOperationState::Pending {
                remaining_polls: POLLS_BEFORE_TERMINAL,
                target: name.clone(),
                ledger: ledger.clone(),
            },
        );
        provider.dedup_log.insert(
            key,
            AppliedWrite::Delete {
                name: name.to_string(),
                operation_name: operation_name.clone(),
            },
        );
        Operation::pending(operation_name, ledger).map_err(|e| ProviderError::Internal {
            message: e.to_string(),
        })
    })
}

pub(super) fn poll_operation<'a>(
    provider: &'a mut ReferenceProvider,
    operation_name: &'a str,
) -> ProviderFuture<'a, Operation> {
    Box::pin(async move {
        let state = provider
            .operations
            .get(operation_name)
            .cloned()
            .ok_or_else(|| ProviderError::NotFound {
                name: operation_name.to_owned(),
            })?;
        match state {
            ReferenceOperationState::Pending {
                remaining_polls,
                target,
                ledger,
            } => {
                if remaining_polls > 1 {
                    provider.operations.insert(
                        operation_name.to_owned(),
                        ReferenceOperationState::Pending {
                            remaining_polls: remaining_polls - 1,
                            target,
                            ledger: ledger.clone(),
                        },
                    );
                    return Operation::pending(operation_name.to_owned(), ledger).map_err(|e| {
                        ProviderError::Internal {
                            message: e.to_string(),
                        }
                    });
                }
                provider.items.remove(&target.to_string());
                let terminal_ledger = OperationLedgerEntry {
                    observed_generation: ledger.desired_generation,
                    state: LedgerState::Succeeded,
                    transition_sequence: ledger.transition_sequence + 1,
                    ..ledger
                };
                let terminal = Operation::succeeded(
                    operation_name.to_owned(),
                    terminal_ledger,
                    serde_json::json!({ "deleted": target.to_string() }),
                )
                .map_err(|e| ProviderError::Internal {
                    message: e.to_string(),
                })?;
                provider.operations.insert(
                    operation_name.to_owned(),
                    ReferenceOperationState::Terminal(terminal.clone()),
                );
                Ok(terminal)
            }
            ReferenceOperationState::Terminal(operation) => Ok(operation),
        }
    })
}

pub(super) fn operation_ledger_entry<'a>(
    provider: &'a ReferenceProvider,
    operation_name: &'a str,
) -> ProviderFuture<'a, OperationLedgerEntry> {
    Box::pin(async move {
        match provider.operations.get(operation_name) {
            Some(ReferenceOperationState::Terminal(operation)) => Ok(operation.metadata.clone()),
            Some(ReferenceOperationState::Pending { ledger, .. }) => Ok(ledger.clone()),
            None => Err(ProviderError::NotFound {
                name: operation_name.to_owned(),
            }),
        }
    })
}
