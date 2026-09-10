pub const STORAGE_BLOCK_CREATE_SURFACE: &str = "storage.block.create";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloudStorageBlockCreateApiStatus {
    Created,
    BadRequest,
    Forbidden,
    NotFound,
    Conflict,
    UnprocessableEntity,
}

impl CloudStorageBlockCreateApiStatus {
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
pub enum CloudStorageBlockApiErrorCode {
    RequestIdEmpty,
    TenantHeaderEmpty,
    IdempotencyKeyEmpty,
    PrincipalIdEmpty,
    PathVolumeIdEmpty,
    VolumeIdMismatch,
    TenantMismatch,
    AuthorizationDecisionIdEmpty,
    AuthorizationTenantMismatch,
    AuthorizationPrincipalMismatch,
    AuthorizationDenied,
    IdempotencyKeyReused,
    ResidencyInvalid,
    VolumeTierInvalid,
    EncryptionInvalid,
    DataClassInvalid,
    StorageInvalidRequest,
    StorageForbidden,
    StorageNotFound,
    StorageConflict,
}

impl CloudStorageBlockApiErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RequestIdEmpty => "STORAGE_BLOCK_REQUEST_ID_EMPTY",
            Self::TenantHeaderEmpty => "STORAGE_BLOCK_TENANT_HEADER_EMPTY",
            Self::IdempotencyKeyEmpty => "STORAGE_BLOCK_IDEMPOTENCY_KEY_EMPTY",
            Self::PrincipalIdEmpty => "STORAGE_BLOCK_PRINCIPAL_ID_EMPTY",
            Self::PathVolumeIdEmpty => "STORAGE_BLOCK_PATH_VOLUME_ID_EMPTY",
            Self::VolumeIdMismatch => "STORAGE_BLOCK_VOLUME_ID_MISMATCH",
            Self::TenantMismatch => "STORAGE_BLOCK_TENANT_MISMATCH",
            Self::AuthorizationDecisionIdEmpty => "STORAGE_BLOCK_AUTHORIZATION_DECISION_ID_EMPTY",
            Self::AuthorizationTenantMismatch => "STORAGE_BLOCK_AUTHORIZATION_TENANT_MISMATCH",
            Self::AuthorizationPrincipalMismatch => {
                "STORAGE_BLOCK_AUTHORIZATION_PRINCIPAL_MISMATCH"
            }
            Self::AuthorizationDenied => "STORAGE_BLOCK_AUTHORIZATION_DENIED",
            Self::IdempotencyKeyReused => "STORAGE_BLOCK_IDEMPOTENCY_KEY_REUSED",
            Self::ResidencyInvalid => "STORAGE_BLOCK_RESIDENCY_INVALID",
            Self::VolumeTierInvalid => "STORAGE_BLOCK_VOLUME_TIER_INVALID",
            Self::EncryptionInvalid => "STORAGE_BLOCK_ENCRYPTION_INVALID",
            Self::DataClassInvalid => "STORAGE_BLOCK_DATA_CLASS_INVALID",
            Self::StorageInvalidRequest => "STORAGE_BLOCK_INVALID_REQUEST",
            Self::StorageForbidden => "STORAGE_BLOCK_FORBIDDEN",
            Self::StorageNotFound => "STORAGE_BLOCK_NOT_FOUND",
            Self::StorageConflict => "STORAGE_BLOCK_CONFLICT",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageBlockApiBoundaryContext {
    pub request_id: String,
    pub tenant_id: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageBlockApiPrincipal {
    pub tenant_id: String,
    pub principal_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageBlockApiAuthorization {
    pub tenant_id: String,
    pub principal_id: String,
    pub decision_id: String,
    pub allowed_surfaces: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageBlockVolumeCreateRequest {
    pub resource_id: String,
    pub tenant_id: String,
    pub name: String,
    pub region: String,
    pub az: String,
    pub cell_id: String,
    pub residency: String,
    pub tier: String,
    pub size_gib: u64,
    pub performance: CloudStorageBlockVolumePerformance,
    pub encryption: String,
    pub kms_key: Option<String>,
    pub data_class: String,
    pub created_at_epoch_seconds: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CloudStorageBlockVolumePerformance {
    pub iops: u64,
    pub throughput_mbps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageBlockVolumeCreateApiRequest {
    pub path_volume_id: String,
    pub boundary: CloudStorageBlockApiBoundaryContext,
    pub principal: CloudStorageBlockApiPrincipal,
    pub authorization: CloudStorageBlockApiAuthorization,
    pub body: CloudStorageBlockVolumeCreateRequest,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CloudStorageBlockCreateIdempotencyLedger {
    entries: BTreeMap<
        CloudStorageBlockIdempotencyLedgerKey,
        CloudStorageBlockCreateIdempotencyLedgerEntry,
    >,
}

impl CloudStorageBlockCreateIdempotencyLedger {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct CloudStorageBlockIdempotencyLedgerKey {
    tenant_id: String,
    principal_id: String,
    surface: String,
    idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CloudStorageBlockCreateIdempotencyLedgerEntry {
    fingerprint: CloudStorageBlockRequestFingerprint,
    result: CloudStorageBlockCreateApiResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CloudStorageBlockRequestFingerprint {
    canonical: String,
}

type CloudStorageBlockCreateApiResult =
    Result<CloudStorageBlockVolumeCreateSuccessResponse, CloudStorageBlockApiError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageBlockVolumeCreateSuccessResponse {
    pub data: CloudStorageBlockVolumeRecord,
    pub metadata: CloudStorageBlockApiMetadata,
}

impl CloudStorageBlockVolumeCreateSuccessResponse {
    pub fn created(data: CloudStorageBlockVolumeRecord, request_id: impl Into<String>) -> Self {
        Self {
            data,
            metadata: CloudStorageBlockApiMetadata {
                request_id: request_id.into(),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageBlockApiMetadata {
    pub request_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageBlockVolumeRecord {
    pub resource_id: String,
    pub tenant_id: String,
    pub name: String,
    pub region: String,
    pub az: String,
    pub cell_id: String,
    pub residency: String,
    pub tier: String,
    pub size_gib: u64,
    pub performance: CloudStorageBlockVolumePerformance,
    pub encryption: String,
    pub kms_key: Option<String>,
    pub data_class: String,
    pub state: String,
    pub created_at_epoch_seconds: u64,
    pub schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageBlockApiErrorResponse {
    pub error: CloudStorageBlockApiErrorBody,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageBlockApiErrorBody {
    pub code: String,
    pub message: String,
    pub message_localized: Option<String>,
    pub request_id: String,
    pub details: Vec<CloudStorageBlockApiErrorDetail>,
    pub retry_after_seconds: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudStorageBlockApiErrorDetail {
    pub field: String,
    pub issue: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloudStorageBlockApiError {
    EmptyRequestId,
    EmptyTenantHeader,
    EmptyIdempotencyKey,
    EmptyPrincipalId,
    EmptyPathVolumeId,
    VolumeIdMismatch {
        path_volume_id: String,
        body_resource_id: String,
    },
    TenantMismatch {
        header_tenant_id: String,
        principal_tenant_id: String,
        body_tenant_id: String,
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
    InvalidResidencyLabel {
        residency: String,
    },
    InvalidVolumeTierLabel {
        tier: String,
    },
    InvalidEncryptionLabel {
        encryption: String,
    },
    InvalidDataClassLabel {
        data_class: String,
    },
    Storage(CloudStorageError),
}
