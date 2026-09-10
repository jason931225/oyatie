use std::collections::BTreeMap;

use shared_resource_provider_contract_kernel::conformance::{
    ConformanceFixture, check_operation_ledger_semantics,
};
use shared_resource_provider_contract_kernel::{
    CreateOutcome, IdempotencyKey, ListEntry, Operation, OperationLedgerEntry, Page, PageRequest,
    ProviderError, ProviderFuture, PutOutcome, ResourceName, ResourceProvider,
};

use super::fixture::ReferenceFixture;
use super::support::{Document, ReferenceProvider};

/// Catches providers that put operation-looking metadata in the response
/// while skipping the operation-ledger write-before-ack contract.
#[derive(Debug, Default)]
struct MissingOperationLedgerProvider(ReferenceProvider);

impl ResourceProvider for MissingOperationLedgerProvider {
    type Resource = Document;

    fn create<'a>(
        &'a mut self,
        name: &'a ResourceName,
        resource: Document,
        idempotency_key: &'a IdempotencyKey,
    ) -> ProviderFuture<'a, CreateOutcome<Document>> {
        self.0.create(name, resource, idempotency_key)
    }

    fn put<'a>(
        &'a mut self,
        name: &'a ResourceName,
        resource: Document,
        idempotency_key: &'a IdempotencyKey,
    ) -> ProviderFuture<'a, PutOutcome<Document>> {
        self.0.put(name, resource, idempotency_key)
    }

    fn get<'a>(&'a self, name: &'a ResourceName) -> ProviderFuture<'a, Document> {
        self.0.get(name)
    }

    fn list<'a>(
        &'a self,
        collection: &'a str,
        request: &'a PageRequest,
    ) -> ProviderFuture<'a, Page<ListEntry<Document>>> {
        self.0.list(collection, request)
    }

    fn delete<'a>(
        &'a mut self,
        name: &'a ResourceName,
        idempotency_key: &'a IdempotencyKey,
    ) -> ProviderFuture<'a, Operation> {
        self.0.delete(name, idempotency_key)
    }

    fn poll_operation<'a>(&'a mut self, operation_name: &'a str) -> ProviderFuture<'a, Operation> {
        self.0.poll_operation(operation_name)
    }

    fn operation_ledger_entry<'a>(
        &'a self,
        operation_name: &'a str,
    ) -> ProviderFuture<'a, OperationLedgerEntry> {
        Box::pin(async move {
            Err(ProviderError::NotFound {
                name: operation_name.to_owned(),
            })
        })
    }
}

struct MissingOperationLedgerFixture;

impl ConformanceFixture for MissingOperationLedgerFixture {
    type Provider = MissingOperationLedgerProvider;

    fn fresh_provider(&self) -> MissingOperationLedgerProvider {
        MissingOperationLedgerProvider::default()
    }

    fn collection(&self) -> &str {
        "documents"
    }

    fn resource_payload(&self, ordinal: u32) -> Document {
        ReferenceFixture.resource_payload(ordinal)
    }
}

#[tokio::test]
async fn harness_catches_missing_operation_ledger_row() {
    let violation = check_operation_ledger_semantics(&MissingOperationLedgerFixture)
        .await
        .unwrap_err();
    assert_eq!(violation.check, "operation_ledger");
    assert!(
        violation.detail.contains("ledger read after delete"),
        "{violation}"
    );
}

#[derive(Debug, Default)]
struct MismatchedReplayOperationProvider {
    inner: ReferenceProvider,
    delete_replay_counts: BTreeMap<String, u32>,
}

impl ResourceProvider for MismatchedReplayOperationProvider {
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
            let key = idempotency_key.as_str().to_owned();
            let count = self.delete_replay_counts.get(&key).copied().unwrap_or(0);
            let mut operation = self.inner.delete(name, idempotency_key).await?;
            if count > 0 {
                operation.metadata.request_hash.push_str(":mismatched");
            }
            self.delete_replay_counts.insert(key, count + 1);
            Ok(operation)
        })
    }

    fn poll_operation<'a>(&'a mut self, operation_name: &'a str) -> ProviderFuture<'a, Operation> {
        self.inner.poll_operation(operation_name)
    }

    fn operation_ledger_entry<'a>(
        &'a self,
        operation_name: &'a str,
    ) -> ProviderFuture<'a, OperationLedgerEntry> {
        self.inner.operation_ledger_entry(operation_name)
    }
}

struct MismatchedReplayOperationFixture;

impl ConformanceFixture for MismatchedReplayOperationFixture {
    type Provider = MismatchedReplayOperationProvider;

    fn fresh_provider(&self) -> MismatchedReplayOperationProvider {
        MismatchedReplayOperationProvider::default()
    }

    fn collection(&self) -> &str {
        "documents"
    }

    fn resource_payload(&self, ordinal: u32) -> Document {
        ReferenceFixture.resource_payload(ordinal)
    }
}

#[tokio::test]
async fn harness_catches_replay_operation_that_no_longer_snapshots_ledger() {
    let violation = check_operation_ledger_semantics(&MismatchedReplayOperationFixture)
        .await
        .unwrap_err();
    assert_eq!(violation.check, "operation_ledger");
    assert!(
        violation
            .detail
            .contains("operation metadata must be a snapshot"),
        "{violation}"
    );
}
