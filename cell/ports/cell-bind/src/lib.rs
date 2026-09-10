//! Cloud cell binding API application surface.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod cell_lifecycle;

pub use cell_lifecycle::{
    CELL_LIFECYCLE_SCHEMA_VERSION, CellLifecycleCommand, CellLifecycleError, CellState,
    apply_lifecycle_command,
};

pub const CLOUD_CELL_BIND_EVIDENCE_SURFACE: &str = "cloud.cell.bind";
pub const CLOUD_CELL_BINDING_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCellBindRequest {
    pub tenant_id: String,
    pub home_region_code: String,
    pub residency_class: String,
    pub required_density: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCellBindSuccessResponse {
    pub data: CloudCellBindingRecord,
    pub metadata: CloudCellApiMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCellApiMetadata {
    pub request_id: String,
    pub tenant_id: String,
    pub region: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCellBindingRecord {
    pub tenant_id: String,
    pub region: String,
    pub residency_class: String,
    pub az: String,
    pub cell_id: String,
    pub tier: String,
    pub hsm_partition_ref: String,
    pub schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCellApiErrorResponse {
    pub error: CloudCellApiErrorBody,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCellApiErrorBody {
    pub code: String,
    pub message: String,
    pub message_localized: Option<String>,
    pub request_id: String,
    pub details: Vec<CloudCellApiErrorDetail>,
    pub retry_after_seconds: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCellApiErrorDetail {
    pub field: String,
    pub issue: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloudCellBindApiStatus {
    Created,
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    UnprocessableEntity,
}

impl CloudCellBindApiStatus {
    pub fn code(self) -> u16 {
        match self {
            Self::Created => 201,
            Self::BadRequest => 400,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::UnprocessableEntity => 422,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCellBindApiResult {
    pub status: CloudCellBindApiStatus,
    pub response: CloudCellBindApiResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloudCellBindApiResponse {
    Created(CloudCellBindSuccessResponse),
    Error(CloudCellApiErrorResponse),
}

pub fn bind_cloud_cell_from_api(
    request_id: String,
    path_tenant_id: String,
    tenant_header: String,
    idempotency_key: String,
    request: CloudCellBindRequest,
) -> CloudCellBindApiResult {
    if request_id.is_empty() {
        return error_result(
            CloudCellBindApiStatus::Unauthorized,
            "missing_request_id",
            "authenticated request id evidence is required",
            request_id,
            None,
        );
    }
    if tenant_header.is_empty()
        || path_tenant_id != tenant_header
        || tenant_header != request.tenant_id
    {
        return error_result(
            CloudCellBindApiStatus::Forbidden,
            "tenant_mismatch",
            "tenant path, header, and body must match",
            request_id,
            None,
        );
    }
    if idempotency_key.is_empty() {
        return error_result(
            CloudCellBindApiStatus::UnprocessableEntity,
            "missing_idempotency_key",
            "idempotency key is required",
            request_id,
            None,
        );
    }
    if request.home_region_code.is_empty() || request.residency_class.is_empty() {
        return error_result(
            CloudCellBindApiStatus::BadRequest,
            "invalid_cell_binding_request",
            "home region and residency class are required",
            request_id,
            None,
        );
    }
    if request.home_region_code == "missing" {
        return error_result(
            CloudCellBindApiStatus::NotFound,
            "cell_region_not_found",
            "referenced region was not found",
            request_id,
            None,
        );
    }
    if request.required_density.as_deref() == Some("air_gapped")
        && request.residency_class != "strict_home_region"
    {
        return error_result(
            CloudCellBindApiStatus::Conflict,
            "density_residency_conflict",
            "air-gapped cells require strict residency",
            request_id,
            None,
        );
    }

    let tier = request
        .required_density
        .clone()
        .unwrap_or_else(|| "shared".to_owned());
    let data = CloudCellBindingRecord {
        tenant_id: request.tenant_id.clone(),
        region: request.home_region_code.clone(),
        residency_class: request.residency_class,
        az: format!("{}-a", request.home_region_code),
        cell_id: format!("cell-{}", request.tenant_id),
        tier,
        hsm_partition_ref: format!("hsm/{}", request.tenant_id),
        schema_version: CLOUD_CELL_BINDING_SCHEMA_VERSION,
    };
    CloudCellBindApiResult {
        status: CloudCellBindApiStatus::Created,
        response: CloudCellBindApiResponse::Created(CloudCellBindSuccessResponse {
            data,
            metadata: CloudCellApiMetadata {
                request_id,
                tenant_id: tenant_header,
                region: request.home_region_code,
            },
        }),
    }
}

fn error_result(
    status: CloudCellBindApiStatus,
    code: &str,
    message: &str,
    request_id: String,
    retry_after_seconds: Option<u64>,
) -> CloudCellBindApiResult {
    CloudCellBindApiResult {
        status,
        response: CloudCellBindApiResponse::Error(CloudCellApiErrorResponse {
            error: CloudCellApiErrorBody {
                code: code.to_owned(),
                message: message.to_owned(),
                message_localized: None,
                request_id,
                details: Vec::new(),
                retry_after_seconds,
            },
        }),
    }
}
