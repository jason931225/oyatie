pub const CLOUD_STORAGE_OBJECT_PUT_SURFACE: &str = "cloud.storage.object.put";
pub const CLOUD_STORAGE_OBJECT_GET_SURFACE: &str = "cloud.storage.object.get";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloudStorageObjectPutApiStatus {
    Created,
    BadRequest,
    Forbidden,
    NotFound,
    Conflict,
    UnprocessableEntity,
}

impl CloudStorageObjectPutApiStatus {
    pub const fn code(self) -> u16 {
        match self {
            Self::Created => 201,
            Self::BadRequest => 400,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::UnprocessableEntity => 422,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloudStorageObjectGetApiStatus {
    Ok,
    BadRequest,
    Forbidden,
    NotFound,
}

impl CloudStorageObjectGetApiStatus {
    pub const fn code(self) -> u16 {
        match self {
            Self::Ok => 200,
            Self::BadRequest => 400,
            Self::Forbidden => 403,
            Self::NotFound => 404,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloudStorageObjectApiErrorCode {
    RequestIdEmpty,
    TenantHeaderEmpty,
    IdempotencyKeyEmpty,
    PrincipalIdEmpty,
    BucketIdInvalid,
    BucketKindMismatch,
    ObjectKeyInvalid,
    BucketIdMismatch,
    ObjectKeyMismatch,
    TenantMismatch,
    AuthorizationDecisionIdEmpty,
    AuthorizationTenantMismatch,
    AuthorizationPrincipalMismatch,
    AuthorizationDenied,
    IdempotencyKeyReused,
    DataClassInvalid,
    KmsPurposeInvalid,
    ObjectNotFound,
    StorageInvalidRequest,
    StorageForbidden,
    StorageNotFound,
    StorageConflict,
}

impl CloudStorageObjectApiErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RequestIdEmpty => "CLOUD_STORAGE_OBJECT_REQUEST_ID_EMPTY",
            Self::TenantHeaderEmpty => "CLOUD_STORAGE_OBJECT_TENANT_HEADER_EMPTY",
            Self::IdempotencyKeyEmpty => "CLOUD_STORAGE_OBJECT_IDEMPOTENCY_KEY_EMPTY",
            Self::PrincipalIdEmpty => "CLOUD_STORAGE_OBJECT_PRINCIPAL_ID_EMPTY",
            Self::BucketIdInvalid => "CLOUD_STORAGE_OBJECT_BUCKET_ID_INVALID",
            Self::BucketKindMismatch => "CLOUD_STORAGE_OBJECT_BUCKET_KIND_MISMATCH",
            Self::ObjectKeyInvalid => "CLOUD_STORAGE_OBJECT_KEY_INVALID",
            Self::BucketIdMismatch => "CLOUD_STORAGE_OBJECT_BUCKET_ID_MISMATCH",
            Self::ObjectKeyMismatch => "CLOUD_STORAGE_OBJECT_KEY_MISMATCH",
            Self::TenantMismatch => "CLOUD_STORAGE_OBJECT_TENANT_MISMATCH",
            Self::AuthorizationDecisionIdEmpty => {
                "CLOUD_STORAGE_OBJECT_AUTHORIZATION_DECISION_ID_EMPTY"
            }
            Self::AuthorizationTenantMismatch => {
                "CLOUD_STORAGE_OBJECT_AUTHORIZATION_TENANT_MISMATCH"
            }
            Self::AuthorizationPrincipalMismatch => {
                "CLOUD_STORAGE_OBJECT_AUTHORIZATION_PRINCIPAL_MISMATCH"
            }
            Self::AuthorizationDenied => "CLOUD_STORAGE_OBJECT_AUTHORIZATION_DENIED",
            Self::IdempotencyKeyReused => "CLOUD_STORAGE_OBJECT_IDEMPOTENCY_KEY_REUSED",
            Self::DataClassInvalid => "CLOUD_STORAGE_OBJECT_DATA_CLASS_INVALID",
            Self::KmsPurposeInvalid => "CLOUD_STORAGE_OBJECT_KMS_PURPOSE_INVALID",
            Self::ObjectNotFound => "CLOUD_STORAGE_OBJECT_NOT_FOUND",
            Self::StorageInvalidRequest => "CLOUD_STORAGE_OBJECT_INVALID_REQUEST",
            Self::StorageForbidden => "CLOUD_STORAGE_OBJECT_FORBIDDEN",
            Self::StorageNotFound => "CLOUD_STORAGE_OBJECT_STORAGE_NOT_FOUND",
            Self::StorageConflict => "CLOUD_STORAGE_OBJECT_CONFLICT",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectMutationBoundaryContext {
    pub request_id: String,
    pub tenant_id: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectReadBoundaryContext {
    pub request_id: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectApiPrincipal {
    pub tenant_id: String,
    pub principal_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectApiAuthorization {
    pub tenant_id: String,
    pub principal_id: String,
    pub decision_id: String,
    pub allowed_surfaces: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectPutRequest {
    pub bucket_id: String,
    pub tenant_id: String,
    pub key: String,
    pub size_bytes: u64,
    pub etag: String,
    pub data_class: String,
    pub encryption: CloudStorageObjectEncryptionBindingRequest,
    pub stored_at_epoch_seconds: u64,
    pub last_accessed_at_epoch_seconds: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectEncryptionBindingRequest {
    pub kms_key: String,
    pub kms_key_version: u32,
    pub material_ref: String,
    pub ciphertext_ref: String,
    pub kms_encrypt_event_id: String,
    pub purpose: String,
    pub shred_proof_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectPutApiRequest {
    pub path_bucket_id: String,
    pub path_object_key: String,
    pub boundary: CloudStorageObjectMutationBoundaryContext,
    pub principal: CloudStorageObjectApiPrincipal,
    pub authorization: CloudStorageObjectApiAuthorization,
    pub body: CloudStorageObjectPutRequest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageObjectGetApiRequest {
    pub path_bucket_id: String,
    pub path_object_key: String,
    pub boundary: CloudStorageObjectReadBoundaryContext,
    pub principal: CloudStorageObjectApiPrincipal,
    pub authorization: CloudStorageObjectApiAuthorization,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloudStorageObjectReplayOutcome {
    Replayed {
        response: Box<CloudStorageObjectPutSuccessResponse>,
    },
    Conflict {
        idempotency_key: String,
    },
}
