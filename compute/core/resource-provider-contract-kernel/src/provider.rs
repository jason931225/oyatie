use core::future::Future;
use core::pin::Pin;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    IdempotencyKey, ListEntry, Operation, OperationLedgerEntry, Page, PageRequest, ResourceName,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderError {
    AlreadyExists { name: String },
    NotFound { name: String },
    IdempotencyKeyReuse { key: String },
    InvalidArgument { message: String },
    FailedPrecondition { message: String },
    Internal { message: String },
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists { name } => write!(f, "already exists: {name}"),
            Self::NotFound { name } => write!(f, "not found: {name}"),
            Self::IdempotencyKeyReuse { key } => {
                write!(f, "idempotency key {key} reused with different parameters")
            }
            Self::InvalidArgument { message } => write!(f, "invalid argument: {message}"),
            Self::FailedPrecondition { message } => write!(f, "failed precondition: {message}"),
            Self::Internal { message } => write!(f, "internal: {message}"),
        }
    }
}

impl std::error::Error for ProviderError {}

pub type ProviderFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ProviderError>> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteDisposition {
    Created,
    Replaced,
    Replayed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PutOutcome<R> {
    pub resource: R,                   // data_class: TENANT_SCOPED
    pub disposition: WriteDisposition, // data_class: INTERNAL_ONLY
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateOutcome<R> {
    pub resource: R,    // data_class: TENANT_SCOPED
    pub replayed: bool, // data_class: INTERNAL_ONLY
}

pub trait ResourceProvider {
    type Resource: Clone + PartialEq + fmt::Debug;

    /// Create `name`. Replays under the same idempotency key MUST return the
    /// original resource with `replayed = true`; the same key with different
    /// parameters MUST fail with [`ProviderError::IdempotencyKeyReuse`]; an
    /// existing name under a new key MUST fail with
    /// [`ProviderError::AlreadyExists`].
    fn create<'a>(
        &'a mut self,
        name: &'a ResourceName,
        resource: Self::Resource,
        idempotency_key: &'a IdempotencyKey,
    ) -> ProviderFuture<'a, CreateOutcome<Self::Resource>>;

    /// Replays under the same idempotency key MUST be no-ops returning the
    /// original outcome with [`WriteDisposition::Replayed`].
    fn put<'a>(
        &'a mut self,
        name: &'a ResourceName,
        resource: Self::Resource,
        idempotency_key: &'a IdempotencyKey,
    ) -> ProviderFuture<'a, PutOutcome<Self::Resource>>;

    fn get<'a>(&'a self, name: &'a ResourceName) -> ProviderFuture<'a, Self::Resource>;

    fn list<'a>(
        &'a self,
        collection: &'a str,
        request: &'a PageRequest,
    ) -> ProviderFuture<'a, Page<ListEntry<Self::Resource>>>;

    /// Async delete of `name`: returns an AIP-151 operation. Replays under
    /// the same idempotency key MUST return the SAME operation resource.
    fn delete<'a>(
        &'a mut self,
        name: &'a ResourceName,
        idempotency_key: &'a IdempotencyKey,
    ) -> ProviderFuture<'a, Operation>;

    fn poll_operation<'a>(&'a mut self, operation_name: &'a str) -> ProviderFuture<'a, Operation>;

    fn operation_ledger_entry<'a>(
        &'a self,
        operation_name: &'a str,
    ) -> ProviderFuture<'a, OperationLedgerEntry>;
}
