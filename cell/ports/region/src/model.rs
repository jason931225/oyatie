use cell_region::CloudRegionError;

pub const CLOUD_REGION_LIST_SURFACE: &str = "cloud.region.list";
pub const CLOUD_AZ_LIST_SURFACE: &str = "cloud.az.list";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloudRegionListApiStatus {
    Ok,
    BadRequest,
    Forbidden,
    NotFound,
}

impl CloudRegionListApiStatus {
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
pub enum CloudRegionApiErrorCode {
    RequestIdEmpty,
    TenantHeaderEmpty,
    PrincipalIdEmpty,
    PathRegionCodeEmpty,
    TenantMismatch,
    AuthorizationDecisionIdEmpty,
    AuthorizationTenantMismatch,
    AuthorizationPrincipalMismatch,
    AuthorizationDenied,
    RegionInvalidRequest,
    RegionForbidden,
    RegionNotFound,
}

impl CloudRegionApiErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RequestIdEmpty => "CLOUD_REGION_REQUEST_ID_EMPTY",
            Self::TenantHeaderEmpty => "CLOUD_REGION_TENANT_HEADER_EMPTY",
            Self::PrincipalIdEmpty => "CLOUD_REGION_PRINCIPAL_ID_EMPTY",
            Self::PathRegionCodeEmpty => "CLOUD_REGION_PATH_REGION_CODE_EMPTY",
            Self::TenantMismatch => "CLOUD_REGION_TENANT_MISMATCH",
            Self::AuthorizationDecisionIdEmpty => "CLOUD_REGION_AUTHORIZATION_DECISION_ID_EMPTY",
            Self::AuthorizationTenantMismatch => "CLOUD_REGION_AUTHORIZATION_TENANT_MISMATCH",
            Self::AuthorizationPrincipalMismatch => "CLOUD_REGION_AUTHORIZATION_PRINCIPAL_MISMATCH",
            Self::AuthorizationDenied => "CLOUD_REGION_AUTHORIZATION_DENIED",
            Self::RegionInvalidRequest => "CLOUD_REGION_INVALID_REQUEST",
            Self::RegionForbidden => "CLOUD_REGION_FORBIDDEN",
            Self::RegionNotFound => "CLOUD_REGION_NOT_FOUND",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionApiBoundaryContext {
    pub request_id: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionApiPrincipal {
    pub tenant_id: String,
    pub principal_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionApiAuthorization {
    pub tenant_id: String,
    pub principal_id: String,
    pub decision_id: String,
    pub allowed_surfaces: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionListApiRequest {
    pub boundary: CloudRegionApiBoundaryContext,
    pub principal: CloudRegionApiPrincipal,
    pub authorization: CloudRegionApiAuthorization,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudAzListApiRequest {
    pub path_region_code: String,
    pub boundary: CloudRegionApiBoundaryContext,
    pub principal: CloudRegionApiPrincipal,
    pub authorization: CloudRegionApiAuthorization,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionListSuccessResponse {
    pub data: Vec<CloudRegionRecord>,
    pub metadata: CloudRegionApiMetadata,
}

impl CloudRegionListSuccessResponse {
    pub fn ok(data: Vec<CloudRegionRecord>, request_id: impl Into<String>) -> Self {
        Self {
            data,
            metadata: CloudRegionApiMetadata {
                request_id: request_id.into(),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudAzListSuccessResponse {
    pub data: Vec<CloudAzRecord>,
    pub metadata: CloudRegionApiMetadata,
}

impl CloudAzListSuccessResponse {
    pub fn ok(data: Vec<CloudAzRecord>, request_id: impl Into<String>) -> Self {
        Self {
            data,
            metadata: CloudRegionApiMetadata {
                request_id: request_id.into(),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionApiMetadata {
    pub request_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionRecord {
    pub code: String,
    pub display_name: String,
    pub regulatory_packs: Vec<CloudRegionRegulatoryPackRef>,
    pub azs: Vec<CloudRegionAzRef>,
    pub state: String,
    pub provider_facing: bool,
    pub residency_strictness: String,
    pub created_at_epoch_seconds: u64,
    pub schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionRegulatoryPackRef {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionAzRef {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionPowerZoneRef {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionCellRef {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudAzRecord {
    pub code: String,
    pub region_code: String,
    pub power_zones: Vec<CloudRegionPowerZoneRef>,
    pub cells: Vec<CloudRegionCellRef>,
    pub cell_isolation_evidence: Vec<CloudCellIsolationEvidenceRecord>,
    pub state: String,
    pub created_at_epoch_seconds: u64,
    pub schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCellIsolationEvidenceRecord {
    pub cell_id: String,
    pub region_code: String,
    pub az_code: String,
    pub state: String,
    pub tenant_density: String,
    pub allowed_residency: Vec<String>,
    pub evidence_ref: String,
    pub schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionApiErrorResponse {
    pub error: CloudRegionApiErrorBody,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionApiErrorBody {
    pub code: String,
    pub message: String,
    pub message_localized: Option<String>,
    pub request_id: String,
    pub details: Vec<CloudRegionApiErrorDetail>,
    pub retry_after_seconds: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionApiErrorDetail {
    pub field: String,
    pub issue: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloudRegionApiError {
    EmptyRequestId,
    EmptyTenantHeader,
    EmptyPrincipalId,
    EmptyPathRegionCode,
    TenantMismatch {
        header_tenant_id: String,
        principal_tenant_id: String,
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
    Region(CloudRegionError),
}
