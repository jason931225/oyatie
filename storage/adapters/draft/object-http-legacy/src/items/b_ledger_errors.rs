#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectPutIdempotencyEntry {
    pub idempotency_key: String,
    pub outcome: CloudStorageObjectReplayOutcome,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CloudStorageObjectPutIdempotencyLedger {
    entries: BTreeMap<CloudStorageObjectIdempotencyLedgerKey, CloudStorageObjectPutLedgerEntry>,
}

impl CloudStorageObjectPutIdempotencyLedger {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Does not mutate the ledger. Does not drive the catalog.
    pub fn peek(
        &self,
        tenant_id: &str,
        principal_id: &str,
        surface: &str,
        idempotency_key: &str,
    ) -> Option<CloudStorageObjectPutIdempotencyEntry> {
        let key = CloudStorageObjectIdempotencyLedgerKey {
            tenant_id: tenant_id.to_string(),
            principal_id: principal_id.to_string(),
            surface: surface.to_string(),
            idempotency_key: idempotency_key.to_string(),
        };
        let entry = self.entries.get(&key)?;
        let outcome = match &entry.result {
            Ok(response) => CloudStorageObjectReplayOutcome::Replayed {
                response: Box::new(response.clone()),
            },
            Err(_) => CloudStorageObjectReplayOutcome::Conflict {
                idempotency_key: idempotency_key.to_string(),
            },
        };
        Some(CloudStorageObjectPutIdempotencyEntry {
            idempotency_key: idempotency_key.to_string(),
            outcome,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct CloudStorageObjectIdempotencyLedgerKey {
    tenant_id: String,
    principal_id: String,
    surface: String,
    idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CloudStorageObjectPutLedgerEntry {
    fingerprint: CloudStorageObjectRequestFingerprint,
    result: CloudStorageObjectPutApiResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CloudStorageObjectRequestFingerprint {
    canonical: String,
}

type CloudStorageObjectPutApiResult =
    Result<CloudStorageObjectPutSuccessResponse, CloudStorageObjectApiError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectPutSuccessResponse {
    pub data: CloudStorageObjectRecord,
    pub metadata: CloudStorageObjectMetadata,
}

impl CloudStorageObjectPutSuccessResponse {
    pub fn created(data: CloudStorageObjectRecord, request_id: impl Into<String>) -> Self {
        Self {
            data,
            metadata: CloudStorageObjectMetadata {
                request_id: request_id.into(),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectGetSuccessResponse {
    pub data: CloudStorageObjectRecord,
    pub metadata: CloudStorageObjectMetadata,
}

impl CloudStorageObjectGetSuccessResponse {
    pub fn ok(data: CloudStorageObjectRecord, request_id: impl Into<String>) -> Self {
        Self {
            data,
            metadata: CloudStorageObjectMetadata {
                request_id: request_id.into(),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectMetadata {
    pub request_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectRecord {
    pub bucket_id: String,
    pub tenant_id: String,
    pub key: String,
    pub size_bytes: u64,
    pub etag: String,
    pub data_class: String,
    pub encryption: CloudStorageObjectEncryptionBindingRecord,
    pub stored_at_epoch_seconds: u64,
    pub last_accessed_at_epoch_seconds: Option<u64>,
    pub schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectEncryptionBindingRecord {
    pub kms_key: String,
    pub kms_key_version: u32,
    pub material_ref: String,
    pub ciphertext_ref: String,
    pub kms_encrypt_event_id: String,
    pub purpose: String,
    pub shred_proof_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectApiErrorResponse {
    pub error: CloudStorageObjectApiErrorBody,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectApiErrorBody {
    pub code: String,
    pub message: String,
    pub message_localized: Option<String>,
    pub request_id: String,
    pub details: Vec<CloudStorageObjectApiErrorDetail>,
    pub retry_after_seconds: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectApiErrorDetail {
    pub field: String,
    pub issue: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloudStorageObjectApiError {
    EmptyRequestId,
    EmptyTenantHeader,
    EmptyIdempotencyKey,
    EmptyPrincipalId,
    InvalidBucketId {
        bucket_id: String,
    },
    BucketKindMismatch {
        bucket_id: String,
        kind_label: String,
    },
    InvalidObjectKey {
        object_key: String,
    },
    BucketIdMismatch {
        path_bucket_id: String,
        body_bucket_id: String,
    },
    ObjectKeyMismatch {
        path_object_key: String,
        body_key: String,
    },
    TenantMismatch {
        header_tenant_id: String,
        principal_tenant_id: String,
        resource_tenant_id: String,
        body_tenant_id: Option<String>,
    },
    EmptyAuthorizationDecisionId,
    AuthorizationTenantMismatch {
        authorization_tenant_id: String,
        principal_tenant_id: String,
    },
    AuthorizationPrincipalMismatch {
        authorization_principal_id: String,
        principal_id: String,
    },
    AuthorizationDenied {
        surface: String,
    },
    IdempotencyKeyReused {
        idempotency_key: String,
    },
    InvalidDataClassLabel {
        data_class: String,
    },
    InvalidKmsPurposeLabel {
        purpose: String,
    },
    ObjectNotFound {
        bucket_id: String,
        key: String,
    },
    Storage(CloudStorageError),
}
