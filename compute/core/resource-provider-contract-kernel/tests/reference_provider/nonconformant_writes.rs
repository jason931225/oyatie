use shared_resource_provider_contract_kernel::conformance::{
    ConformanceFixture, check_create_idempotency, check_idempotent_put,
};
use shared_resource_provider_contract_kernel::{
    CreateOutcome, IdempotencyKey, ListEntry, Operation, OperationLedgerEntry, Page, PageRequest,
    ProviderError, ProviderFuture, PutOutcome, ResourceName, ResourceProvider, WriteDisposition,
};

use super::fixture::ReferenceFixture;
use super::support::{Document, ReferenceProvider};

#[derive(Debug, Default)]
struct NonReplayingCreateProvider(ReferenceProvider);

impl ResourceProvider for NonReplayingCreateProvider {
    type Resource = Document;

    fn create<'a>(
        &'a mut self,
        name: &'a ResourceName,
        resource: Document,
        _idempotency_key: &'a IdempotencyKey,
    ) -> ProviderFuture<'a, CreateOutcome<Document>> {
        Box::pin(async move {
            if self.0.items.contains_key(&name.to_string()) {
                return Err(ProviderError::AlreadyExists {
                    name: name.to_string(),
                });
            }
            self.0.items.insert(name.to_string(), resource.clone());
            Ok(CreateOutcome {
                resource,
                replayed: false,
            })
        })
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
        self.0.operation_ledger_entry(operation_name)
    }
}

struct NonReplayingCreateFixture;

impl ConformanceFixture for NonReplayingCreateFixture {
    type Provider = NonReplayingCreateProvider;

    fn fresh_provider(&self) -> NonReplayingCreateProvider {
        NonReplayingCreateProvider::default()
    }

    fn collection(&self) -> &str {
        "documents"
    }

    fn resource_payload(&self, ordinal: u32) -> Document {
        ReferenceFixture.resource_payload(ordinal)
    }
}

#[tokio::test]
async fn harness_catches_create_that_does_not_replay() {
    let violation = check_create_idempotency(&NonReplayingCreateFixture)
        .await
        .unwrap_err();
    assert_eq!(violation.check, "create_idempotency");
}

/// Reports a replayed PUT as a replace: the visible-state contract holds but
/// the disposition lies, which the harness must flag.
#[derive(Debug, Default)]
struct MisreportingPutProvider(ReferenceProvider);

impl ResourceProvider for MisreportingPutProvider {
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
        Box::pin(async move {
            self.0
                .put(name, resource, idempotency_key)
                .await
                .map(|outcome| {
                    if outcome.disposition == WriteDisposition::Replayed {
                        PutOutcome {
                            resource: outcome.resource,
                            disposition: WriteDisposition::Replaced,
                        }
                    } else {
                        outcome
                    }
                })
        })
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
        self.0.operation_ledger_entry(operation_name)
    }
}

struct MisreportingPutFixture;

impl ConformanceFixture for MisreportingPutFixture {
    type Provider = MisreportingPutProvider;

    fn fresh_provider(&self) -> MisreportingPutProvider {
        MisreportingPutProvider::default()
    }

    fn collection(&self) -> &str {
        "documents"
    }

    fn resource_payload(&self, ordinal: u32) -> Document {
        ReferenceFixture.resource_payload(ordinal)
    }
}

#[tokio::test]
async fn harness_catches_misreported_put_replay() {
    let violation = check_idempotent_put(&MisreportingPutFixture)
        .await
        .unwrap_err();
    assert_eq!(violation.check, "idempotent_put");
    assert!(violation.detail.contains("replayed"), "{violation}");
}
