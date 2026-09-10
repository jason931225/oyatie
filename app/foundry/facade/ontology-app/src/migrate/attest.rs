use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use foundry_spine::{MigrationPlan, migration_attestation};
use serde::Serialize;

use super::PlanRequest;
use crate::composition::AppState;
use crate::reads::{TENANT_SCOPED_RESOURCE, authorized, refuse, tenant_of};

#[derive(Debug, Serialize)]
pub(crate) struct AttestBody {
    pub(crate) fixpoint: bool,       // data_class: INTERNAL_ONLY
    pub(crate) pending: Vec<String>, // data_class: INTERNAL_ONLY
    pub(crate) poisoned: Vec<u64>,   // data_class: INTERNAL_ONLY
}

pub async fn attest(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let caller = match authorized(&state, &headers, TENANT_SCOPED_RESOURCE) {
        Ok(caller) => caller,
        Err(response) => return *response,
    };
    let Ok(request) = serde_json::from_str::<PlanRequest>(&body) else {
        state.metrics.read_refused();
        return refuse(
            StatusCode::BAD_REQUEST,
            "surface",
            "the request is not a migration plan",
        );
    };
    if request.tenant_id != caller.tenant_id {
        state.metrics.read_refused();
        return refuse(
            StatusCode::BAD_REQUEST,
            "surface",
            "the plan names a tenant other than the credential's",
        );
    }
    let tenant = match tenant_of(&state, &caller) {
        Ok(tenant) => tenant,
        Err(response) => return *response,
    };
    let mut transforms = Vec::with_capacity(request.transforms.len());
    for wire in request.transforms {
        match wire.into_domain() {
            Ok(transform) => transforms.push(transform),
            Err(cause) => {
                state.metrics.read_refused();
                return refuse(StatusCode::BAD_REQUEST, "surface", cause);
            }
        }
    }
    let plan = MigrationPlan {
        tenant_id: request.tenant_id,
        entity_type: request.entity_type,
        from_revision: request.from_revision,
        to_revision: request.to_revision,
        action_type: request.action_type,
        audit_event_type: request.audit_event_type,
        declared_at_epoch_seconds: request.declared_at_epoch_seconds,
        transforms,
    };
    let tenant = tenant.lock().await;
    // `registry_input`, NOT `engine`: the runner's own admission gate reads
    // the untouched fold input (`runner.rs`), and the writer stamps revisions
    // from it. `engine` is that seed plus accumulated link instances, so
    // validating against it can admit a plan the runner would refuse — which
    // is exactly the fixpoint claim this surface must never make.
    if let Err(error) = plan.validate(&tenant.projection.registry_input) {
        state.metrics.read_refused();
        return refuse(
            StatusCode::BAD_REQUEST,
            "surface",
            &format!("the plan is not executable: {error:?}"),
        );
    }
    let attestation = migration_attestation(&tenant.projection, &plan);
    state.metrics.read_served();
    Json(AttestBody {
        fixpoint: attestation.fixpoint,
        pending: attestation.pending,
        poisoned: attestation.poisoned,
    })
    .into_response()
}
