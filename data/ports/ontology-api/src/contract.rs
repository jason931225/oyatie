//! Request/response DTOs, the in-memory directory, and the idempotency
//! ledger of the upsert surface.

use std::collections::BTreeMap;

use data_ontology_domain::ObjectEntity;

use crate::codes::ObjectGraphEntityUpsertApiStatus;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectGraphApiBoundaryContext {
    pub request_id: String,
    pub tenant_id: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectGraphApiPrincipal {
    pub tenant_id: String,
    pub principal_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectGraphApiAuthorization {
    pub tenant_id: String,
    pub principal_id: String,
    pub decision_id: String,
    pub allowed_surfaces: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectGraphEntityPropertyRef {
    pub name: String,
    pub value: String,
    pub tier: String,
    pub data_class: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectGraphEntityUpsertRequest {
    pub tenant_id: String,
    pub entity_id: String,
    pub entity_type: String,
    pub property_refs: Vec<ObjectGraphEntityPropertyRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectGraphEntityUpsertApiRequest {
    pub path_tenant_id: String,
    pub path_entity_id: String,
    pub boundary: ObjectGraphApiBoundaryContext,
    pub principal: ObjectGraphApiPrincipal,
    pub authorization: ObjectGraphApiAuthorization,
    pub body: ObjectGraphEntityUpsertRequest,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ObjectGraphEntityDirectory {
    pub(crate) entities: BTreeMap<ObjectGraphEntityKey, ObjectEntity>,
    pub(crate) events: Vec<ObjectGraphEntityMutationEvent>,
}

impl ObjectGraphEntityDirectory {
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct ObjectGraphEntityKey {
    pub(crate) tenant_id: String,
    pub(crate) entity_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ObjectGraphEntityMutationEvent {
    pub(crate) event_id: String,
    pub(crate) tenant_id: String,
    pub(crate) entity_id: String,
    pub(crate) request_id: String,
    pub(crate) result: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ObjectGraphEntityUpsertIdempotencyLedger {
    pub(crate) entries: BTreeMap<
        ObjectGraphEntityUpsertIdempotencyLedgerKey,
        ObjectGraphEntityUpsertIdempotencyLedgerEntry,
    >,
}

impl ObjectGraphEntityUpsertIdempotencyLedger {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct ObjectGraphEntityUpsertIdempotencyLedgerKey {
    pub(crate) tenant_id: String,
    pub(crate) principal_id: String,
    pub(crate) surface: String,
    pub(crate) idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ObjectGraphEntityUpsertIdempotencyLedgerEntry {
    pub(crate) fingerprint: ObjectGraphEntityUpsertRequestFingerprint,
    pub(crate) result: ObjectGraphEntityUpsertSuccessResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ObjectGraphEntityUpsertRequestFingerprint {
    pub(crate) canonical: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectGraphEntityUpsertSuccessResponse {
    pub data: ObjectGraphEntityRecord,
    pub metadata: ObjectGraphEntityUpsertMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectGraphEntityUpsertMetadata {
    pub request_id: String,
    pub tenant_id: String,
    pub principal_id: String,
    pub result: String,
    pub event_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectGraphEntityRecord {
    pub tenant_id: String,
    pub entity_id: String,
    pub entity_type: String,
    pub property_refs: Vec<ObjectGraphEntityPropertyRef>,
    pub schema_version: u32, // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectGraphEntityUpsertApiErrorResponse {
    pub error: ObjectGraphEntityUpsertApiErrorBody,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectGraphEntityUpsertApiErrorBody {
    pub code: String,
    pub message: String,
    pub message_localized: Option<String>,
    pub request_id: String,
    pub details: Vec<ObjectGraphEntityUpsertApiErrorDetail>,
    pub retry_after_seconds: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectGraphEntityUpsertApiErrorDetail {
    pub field: String,
    pub issue: String,
}
