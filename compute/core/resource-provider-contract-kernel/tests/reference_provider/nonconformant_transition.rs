use std::collections::BTreeMap;

use shared_resource_provider_contract_kernel::conformance::{
    ConformanceFixture, check_operation_ledger_semantics,
};
use shared_resource_provider_contract_kernel::{
    CreateOutcome, IdempotencyKey, ListEntry, Operation, OperationLedgerEntry,
    OperationState as LedgerState, Page, PageRequest, ProviderFuture, PutOutcome, ResourceName,
    ResourceProvider,
};

use super::fixture::ReferenceFixture;
use super::support::{Document, ReferenceProvider};

#[derive(Debug, Default)]
struct DisallowedTransitionProvider {
    inner: ReferenceProvider,
    forced_ledgers: BTreeMap<String, OperationLedgerEntry>,
}

impl ResourceProvider for DisallowedTransitionProvider {
    type Resource = Document;

    fn create<'a>(
        &'a mut self,
        name: &'a ResourceName,
        resource: Document,
        idempotency_key: &'a IdempotencyKey,
    ) -> ProviderFuture<'a, CreateOutcome<Document>> {
        self.inner.create(name, resource, idempotency_key)
    }

    fn put<'a>(
        &'a mut self,
        name: &'a ResourceName,
        resource: Document,
        idempotency_key: &'a IdempotencyKey,
    ) -> ProviderFuture<'a, PutOutcome<Document>> {
        self.inner.put(name, resource, idempotency_key)
    }

    fn get<'a>(&'a self, name: &'a ResourceName) -> ProviderFuture<'a, Document> {
        self.inner.get(name)
    }

    fn list<'a>(
        &'a self,
        collection: &'a str,
        request: &'a PageRequest,
    ) -> ProviderFuture<'a, Page<ListEntry<Document>>> {
        self.inner.list(collection, request)
    }

    fn delete<'a>(
        &'a mut self,
        name: &'a ResourceName,
        idempotency_key: &'a IdempotencyKey,
    ) -> ProviderFuture<'a, Operation> {
        Box::pin(async move {
            let mut operation = self.inner.delete(name, idempotency_key).await?;
            if let Some(ledger) = self.forced_ledgers.get(&operation.name) {
                operation.metadata = ledger.clone();
            } else {
                operation.metadata.state = LedgerState::Accepted;
                self.forced_ledgers
                    .insert(operation.name.clone(), operation.metadata.clone());
            }
            Ok(operation)
        })
    }

    fn poll_operation<'a>(&'a mut self, operation_name: &'a str) -> ProviderFuture<'a, Operation> {
        Box::pin(async move {
            let mut operation = self.inner.poll_operation(operation_name).await?;
            if let Some(previous) = self.forced_ledgers.get(operation_name)
                && !previous.state.is_terminal()
            {
                operation.metadata.state = LedgerState::Succeeded;
                operation.metadata.observed_generation = operation.metadata.desired_generation;
                operation.metadata.transition_sequence = previous.transition_sequence + 1;
                self.forced_ledgers
                    .insert(operation_name.to_owned(), operation.metadata.clone());
            }
            Ok(operation)
        })
    }

    fn operation_ledger_entry<'a>(
        &'a self,
        operation_name: &'a str,
    ) -> ProviderFuture<'a, OperationLedgerEntry> {
        Box::pin(async move {
            if let Some(ledger) = self.forced_ledgers.get(operation_name) {
                return Ok(ledger.clone());
            }
            self.inner.operation_ledger_entry(operation_name).await
        })
    }
}

struct DisallowedTransitionFixture;

impl ConformanceFixture for DisallowedTransitionFixture {
    type Provider = DisallowedTransitionProvider;

    fn fresh_provider(&self) -> DisallowedTransitionProvider {
        DisallowedTransitionProvider::default()
    }

    fn collection(&self) -> &str {
        "documents"
    }

    fn resource_payload(&self, ordinal: u32) -> Document {
        ReferenceFixture.resource_payload(ordinal)
    }
}

#[tokio::test]
async fn harness_catches_disallowed_operation_state_transition() {
    let violation = check_operation_ledger_semantics(&DisallowedTransitionFixture)
        .await
        .unwrap_err();
    assert_eq!(violation.check, "operation_ledger");
    assert!(violation.detail.contains("state transition"), "{violation}");
}
