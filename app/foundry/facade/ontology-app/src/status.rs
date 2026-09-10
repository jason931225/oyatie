//! Authorized, not merely authenticated, even though seven of its eight
//! fields are already visible on unauthenticated surfaces: the fields this
//! one is designed to grow — seed digests, attestations — are not public,
//! and a surface that starts unauthorized has to be taken back later.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::composition::AppState;
use crate::reads::{TENANT_SCOPED_RESOURCE, authorized, tenant_of};

#[derive(Debug, Serialize)]
pub(crate) struct StatusBody {
    pub(crate) policy_version: String,  // data_class: INTERNAL_ONLY
    pub(crate) served_tenants: u64,     // data_class: INTERNAL_ONLY
    pub(crate) observed_tenants: u64,   // data_class: INTERNAL_ONLY
    pub(crate) projection_lag: u64,     // data_class: INTERNAL_ONLY
    pub(crate) poisoned_entries: u64,   // data_class: INTERNAL_ONLY
    pub(crate) contended_tenants: u64,  // data_class: INTERNAL_ONLY
    pub(crate) unreadable_tenants: u64, // data_class: INTERNAL_ONLY
    /// `null` when the tenant was locked and the list could not be read.
    /// An empty list would be a lie: the seed always registers at least one
    /// type, so `[]` cannot honestly mean "declares none", and this read is
    /// a SECOND pass that may disagree with the aggregate above.
    pub(crate) entity_types: Option<Vec<String>>, // data_class: TENANT_SCOPED
}

pub async fn statusz(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let caller = match authorized(&state, &headers, TENANT_SCOPED_RESOURCE) {
        Ok(caller) => caller,
        Err(response) => return *response,
    };
    let tenant = match tenant_of(&state, &caller) {
        Ok(tenant) => tenant,
        Err(response) => return *response,
    };
    let seen = crate::observation::observe(&state);
    let entity_types = entity_types_without_waiting(tenant, &caller.tenant_id);
    state.metrics.read_served();
    Json(StatusBody {
        policy_version: state.pep.loaded_policy_version().as_str().to_owned(),
        served_tenants: state.tenant_count() as u64,
        observed_tenants: seen.observed,
        projection_lag: seen.lag,
        poisoned_entries: seen.poisoned,
        contended_tenants: seen.contended,
        unreadable_tenants: seen.unreadable,
        entity_types,
    })
    .into_response()
}

fn entity_types_without_waiting(
    tenant: &tokio::sync::Mutex<crate::composition::TenantState>,
    tenant_id: &str,
) -> Option<Vec<String>> {
    tenant.try_lock().ok().map(|tenant| {
        crate::seed::declared_entity_types(&tenant.projection.engine, tenant_id)
            .into_iter()
            .map(|definition| definition.id.value.clone())
            .collect()
    })
}
