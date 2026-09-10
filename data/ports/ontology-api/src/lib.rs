//! Platform Object Graph entity upsert API boundary.

use std::collections::BTreeMap;

mod codes;
mod contract;
mod error;
mod handler;
mod mapping;

pub use codes::{ObjectGraphEntityUpsertApiErrorCode, ObjectGraphEntityUpsertApiStatus};
pub use contract::{
    ObjectGraphApiAuthorization, ObjectGraphApiBoundaryContext, ObjectGraphApiPrincipal,
    ObjectGraphEntityDirectory, ObjectGraphEntityPropertyRef, ObjectGraphEntityRecord,
    ObjectGraphEntityUpsertApiErrorBody, ObjectGraphEntityUpsertApiErrorDetail,
    ObjectGraphEntityUpsertApiErrorResponse, ObjectGraphEntityUpsertApiRequest,
    ObjectGraphEntityUpsertIdempotencyLedger, ObjectGraphEntityUpsertMetadata,
    ObjectGraphEntityUpsertRequest, ObjectGraphEntityUpsertSuccessResponse,
};
pub use error::ObjectGraphEntityUpsertApiError;
pub use handler::{
    upsert_object_graph_entity_from_api, validate_object_graph_entity_upsert_request,
};

pub const OBJECT_GRAPH_ENTITY_UPSERT_SURFACE: &str = "object-graph.entity.upsert";
pub const OBJECT_GRAPH_ENTITY_UPSERT_OPENAPI_CONTRACT: &str =
    "contracts/openapi/platform/platform-object-graph-v1.yaml";
