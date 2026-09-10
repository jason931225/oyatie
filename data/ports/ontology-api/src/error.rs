//! The upsert surface's API error, mapped onto codes and details.

use crate::codes::{ObjectGraphEntityUpsertApiErrorCode, ObjectGraphEntityUpsertApiStatus};
use crate::contract::{
    ObjectGraphEntityUpsertApiErrorBody, ObjectGraphEntityUpsertApiErrorDetail,
    ObjectGraphEntityUpsertApiErrorResponse,
};
use crate::mapping::{
    detail, object_graph_kernel_error_code, object_graph_kernel_error_message,
    object_graph_kernel_issue,
};
use data_ontology_domain::ObjectGraphError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectGraphEntityUpsertApiError {
    EmptyRequestId,
    EmptyTenantHeader,
    EmptyIdempotencyKey,
    EmptyPrincipalId,
    EmptyPathTenantId,
    EmptyPathEntityId,
    TenantPathBodyMismatch {
        path_tenant_id: String,
        body_tenant_id: String,
    },
    EntityPathBodyMismatch {
        path_entity_id: String,
        body_entity_id: String,
    },
    PrincipalTenantMismatch {
        principal_tenant_id: String,
        boundary_tenant_id: String,
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
    InvalidPropertyTier {
        tier: String,
    },
    InvalidPropertyDataClass {
        data_class: String,
    },
    IdempotencyKeyReused {
        idempotency_key: String,
    },
    Kernel(ObjectGraphError),
    /// The stored property carries a typed (non-String) value, which this
    /// legacy string-wire surface cannot project. Fail-closed: the typed
    /// read path is the ontology surface's own; this port never lies by
    /// stringifying.
    NonStringPropertyValue {
        name: String,
    },
}

impl ObjectGraphEntityUpsertApiError {
    pub fn object_graph_entity_upsert_status(&self) -> ObjectGraphEntityUpsertApiStatus {
        match self.status_kind() {
            ObjectGraphEntityUpsertApiStatusKind::BadRequest => {
                ObjectGraphEntityUpsertApiStatus::BadRequest
            }
            ObjectGraphEntityUpsertApiStatusKind::Unauthorized => {
                ObjectGraphEntityUpsertApiStatus::Unauthorized
            }
            ObjectGraphEntityUpsertApiStatusKind::Forbidden => {
                ObjectGraphEntityUpsertApiStatus::Forbidden
            }
            ObjectGraphEntityUpsertApiStatusKind::UnprocessableEntity => {
                ObjectGraphEntityUpsertApiStatus::UnprocessableEntity
            }
        }
    }

    pub fn object_graph_entity_upsert_status_code(&self) -> u16 {
        self.object_graph_entity_upsert_status().code()
    }

    pub fn code(&self) -> ObjectGraphEntityUpsertApiErrorCode {
        match self {
            Self::EmptyRequestId => ObjectGraphEntityUpsertApiErrorCode::RequestIdEmpty,
            Self::EmptyTenantHeader => ObjectGraphEntityUpsertApiErrorCode::TenantHeaderEmpty,
            Self::EmptyIdempotencyKey => ObjectGraphEntityUpsertApiErrorCode::IdempotencyKeyEmpty,
            Self::EmptyPrincipalId => ObjectGraphEntityUpsertApiErrorCode::PrincipalIdEmpty,
            Self::EmptyPathTenantId => ObjectGraphEntityUpsertApiErrorCode::PathTenantIdEmpty,
            Self::EmptyPathEntityId => ObjectGraphEntityUpsertApiErrorCode::PathEntityIdEmpty,
            Self::TenantPathBodyMismatch { .. } => {
                ObjectGraphEntityUpsertApiErrorCode::TenantPathBodyMismatch
            }
            Self::EntityPathBodyMismatch { .. } => {
                ObjectGraphEntityUpsertApiErrorCode::EntityPathBodyMismatch
            }
            Self::PrincipalTenantMismatch { .. } => {
                ObjectGraphEntityUpsertApiErrorCode::PrincipalTenantMismatch
            }
            Self::EmptyAuthorizationDecisionId => {
                ObjectGraphEntityUpsertApiErrorCode::AuthorizationDecisionIdEmpty
            }
            Self::AuthorizationTenantMismatch { .. } => {
                ObjectGraphEntityUpsertApiErrorCode::AuthorizationTenantMismatch
            }
            Self::AuthorizationPrincipalMismatch { .. } => {
                ObjectGraphEntityUpsertApiErrorCode::AuthorizationPrincipalMismatch
            }
            Self::AuthorizationDenied { .. } => {
                ObjectGraphEntityUpsertApiErrorCode::AuthorizationDenied
            }
            Self::InvalidPropertyTier { .. } => {
                ObjectGraphEntityUpsertApiErrorCode::PropertyTierInvalid
            }
            Self::InvalidPropertyDataClass { .. } => {
                ObjectGraphEntityUpsertApiErrorCode::PropertyDataClassInvalid
            }
            Self::IdempotencyKeyReused { .. } => {
                ObjectGraphEntityUpsertApiErrorCode::IdempotencyKeyReused
            }
            Self::Kernel(error) => object_graph_kernel_error_code(error),
            Self::NonStringPropertyValue { .. } => {
                ObjectGraphEntityUpsertApiErrorCode::PropertyValueNotString
            }
        }
    }

    pub fn error_response(
        &self,
        request_id: impl Into<String>,
    ) -> ObjectGraphEntityUpsertApiErrorResponse {
        ObjectGraphEntityUpsertApiErrorResponse {
            error: ObjectGraphEntityUpsertApiErrorBody {
                code: self.code().as_str().to_string(),
                message: self.message().to_string(),
                message_localized: None,
                request_id: request_id.into(),
                details: self.details(),
                retry_after_seconds: None,
            },
        }
    }

    fn status_kind(&self) -> ObjectGraphEntityUpsertApiStatusKind {
        match self {
            Self::EmptyPrincipalId => ObjectGraphEntityUpsertApiStatusKind::Unauthorized,
            Self::PrincipalTenantMismatch { .. }
            | Self::EmptyAuthorizationDecisionId
            | Self::AuthorizationTenantMismatch { .. }
            | Self::AuthorizationPrincipalMismatch { .. }
            | Self::AuthorizationDenied { .. } => ObjectGraphEntityUpsertApiStatusKind::Forbidden,
            Self::IdempotencyKeyReused { .. } => {
                ObjectGraphEntityUpsertApiStatusKind::UnprocessableEntity
            }
            Self::EmptyRequestId
            | Self::EmptyTenantHeader
            | Self::EmptyIdempotencyKey
            | Self::EmptyPathTenantId
            | Self::EmptyPathEntityId
            | Self::TenantPathBodyMismatch { .. }
            | Self::EntityPathBodyMismatch { .. }
            | Self::InvalidPropertyTier { .. }
            | Self::InvalidPropertyDataClass { .. }
            | Self::Kernel(_) => ObjectGraphEntityUpsertApiStatusKind::BadRequest,
            Self::NonStringPropertyValue { .. } => {
                ObjectGraphEntityUpsertApiStatusKind::UnprocessableEntity
            }
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::EmptyRequestId => "X-Request-Id header is required",
            Self::EmptyTenantHeader => "X-Tenant-Id header is required",
            Self::EmptyIdempotencyKey => "Idempotency-Key header is required",
            Self::EmptyPrincipalId => "Authenticated principal id is required",
            Self::EmptyPathTenantId => "Path tenant id is required",
            Self::EmptyPathEntityId => "Path entity id is required",
            Self::TenantPathBodyMismatch { .. } => {
                "Path tenant id must match request body tenant_id"
            }
            Self::EntityPathBodyMismatch { .. } => {
                "Path entity id must match request body entity_id"
            }
            Self::PrincipalTenantMismatch { .. } => {
                "Authenticated principal tenant must match the tenant header"
            }
            Self::EmptyAuthorizationDecisionId => "Authorization decision id is required",
            Self::AuthorizationTenantMismatch { .. } => {
                "Authorization decision tenant must match the authenticated principal"
            }
            Self::AuthorizationPrincipalMismatch { .. } => {
                "Authorization decision principal must match the authenticated principal id"
            }
            Self::AuthorizationDenied { .. } => {
                "Authorization decision does not allow the requested Object Graph entity upsert surface"
            }
            Self::InvalidPropertyTier { .. } => {
                "Object Graph property tier must be scalar, vector, timeseries, geo, ciphertext, or struct"
            }
            Self::InvalidPropertyDataClass { .. } => {
                "Object Graph property data_class must be a privacy-program data class"
            }
            Self::IdempotencyKeyReused { .. } => {
                "Idempotency key was already used with a different request"
            }
            Self::Kernel(error) => object_graph_kernel_error_message(error),
            Self::NonStringPropertyValue { .. } => {
                "Stored property value is typed; this string-wire surface cannot project it"
            }
        }
    }

    fn details(&self) -> Vec<ObjectGraphEntityUpsertApiErrorDetail> {
        match self {
            Self::NonStringPropertyValue { name } => {
                vec![detail(name, "typed value has no string projection")]
            }
            Self::EmptyRequestId => vec![detail("header.X-Request-Id", "must be non-empty")],
            Self::EmptyTenantHeader => vec![detail("header.X-Tenant-Id", "must be non-empty")],
            Self::EmptyIdempotencyKey => {
                vec![detail("header.Idempotency-Key", "must be non-empty")]
            }
            Self::EmptyPrincipalId => vec![detail("principal.principal_id", "must be non-empty")],
            Self::EmptyPathTenantId => vec![detail("path.tenant_id", "must be non-empty")],
            Self::EmptyPathEntityId => vec![detail("path.entity_id", "must be non-empty")],
            Self::TenantPathBodyMismatch { .. } => vec![detail(
                "body.tenant_id",
                "must match the tenant_id path parameter",
            )],
            Self::EntityPathBodyMismatch { .. } => vec![detail(
                "body.entity_id",
                "must match the entity_id path parameter",
            )],
            Self::PrincipalTenantMismatch { .. } => vec![detail(
                "principal.tenant_id",
                "must match the authenticated tenant header",
            )],
            Self::EmptyAuthorizationDecisionId => vec![detail(
                "authorization.decision_id",
                "must be non-empty authorization evidence",
            )],
            Self::AuthorizationTenantMismatch { .. } => vec![detail(
                "authorization.tenant_id",
                "must match the authenticated principal tenant",
            )],
            Self::AuthorizationPrincipalMismatch { .. } => vec![detail(
                "authorization.principal_id",
                "must match the authenticated principal id",
            )],
            Self::AuthorizationDenied { .. } => vec![detail(
                "authorization.allowed_surfaces",
                "must include the requested object-graph.entity.upsert surface",
            )],
            Self::InvalidPropertyTier { .. } => vec![detail(
                "body.property_refs[].tier",
                "must be a closed Object Graph property tier label",
            )],
            Self::InvalidPropertyDataClass { .. } => vec![detail(
                "body.property_refs[].data_class",
                "must be a privacy-program data-class label",
            )],
            Self::IdempotencyKeyReused { .. } => vec![detail(
                "header.Idempotency-Key",
                "same key cannot be reused with a different request fingerprint",
            )],
            Self::Kernel(error) => vec![detail(
                "object_graph_kernel",
                object_graph_kernel_issue(error),
            )],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObjectGraphEntityUpsertApiStatusKind {
    BadRequest,
    Unauthorized,
    Forbidden,
    UnprocessableEntity,
}
