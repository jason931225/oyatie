//! Cross-tenant refusals are deliberately indistinguishable from one
//! another: a caller outside the tenant gets the same answer whether the
//! object exists or not, because a distinguishable "not found" would make
//! this surface an existence oracle for a tenant the caller was never
//! entitled to ask about.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use foundry_records_draft::SealedEnvelope;
use foundry_spine::{ViewError, audit_view, object_at_revision, object_history};

use crate::auth::{authenticate, bearer_token};
use crate::authz::Caller;
use crate::composition::AppState;
use crate::dto::RefusalBody;
use crate::pdp::Surface;
use crate::read_dto::{
    AuditRow, EntityTypeRow, HistoryRow, PinnedObjectBody, PropertyBody, RevisionPin, json_value,
};

pub(crate) fn authorized(
    state: &AppState,
    headers: &HeaderMap,
    object_ref: &str,
) -> Result<Caller, Box<Response>> {
    let Some(token) = bearer_token(headers.get("authorization").and_then(|v| v.to_str().ok()))
    else {
        state.metrics.read_refused();
        return Err(Box::new(refuse(
            StatusCode::UNAUTHORIZED,
            "credential",
            "no bearer credential",
        )));
    };
    let Some(caller) = authenticate(&state.operators, token) else {
        state.metrics.read_refused();
        return Err(Box::new(refuse(
            StatusCode::UNAUTHORIZED,
            "credential",
            "the presented credential is not recognized",
        )));
    };
    if state.pep.decide(&caller, Surface::Use, object_ref).is_err() {
        state.metrics.read_refused();
        return Err(Box::new(refuse(
            StatusCode::FORBIDDEN,
            "authorization",
            "the policy decision point refused this read",
        )));
    }
    Ok(caller)
}

pub(crate) fn tenant_of<'a>(
    state: &'a AppState,
    caller: &Caller,
) -> Result<&'a tokio::sync::Mutex<crate::composition::TenantState>, Box<Response>> {
    state.tenants.get(&caller.tenant_id).ok_or_else(|| {
        state.metrics.read_refused();
        Box::new(refuse(
            StatusCode::FORBIDDEN,
            "authorization",
            "the credential names a tenant this process does not serve",
        ))
    })
}

pub async fn object(
    State(state): State<Arc<AppState>>,
    Path(object_ref): Path<String>,
    RawQuery(raw_query): RawQuery,
    headers: HeaderMap,
) -> Response {
    let caller = match authorized(&state, &headers, &object_ref) {
        Ok(caller) => caller,
        Err(response) => return *response,
    };
    let tenant = match tenant_of(&state, &caller) {
        Ok(tenant) => tenant,
        Err(response) => return *response,
    };
    let RevisionPin::Pinned(revision) = RevisionPin::parse(raw_query.as_deref()) else {
        state.metrics.read_refused();
        return refuse(
            StatusCode::BAD_REQUEST,
            "surface",
            "a read must pin the revision it understands: ?revision=N",
        );
    };
    let tenant = tenant.lock().await;
    match object_at_revision(&tenant.projection, &object_ref, revision, None) {
        Ok(pinned) => {
            state.metrics.read_served();
            Json(PinnedObjectBody {
                object_ref,
                written_revision: pinned.written_revision,
                upcast_state: match pinned.upcast_state {
                    foundry_spine::UpcastState::Current => "current",
                    foundry_spine::UpcastState::UpcastPending => "upcast_pending",
                },
                properties: pinned
                    .properties
                    .iter()
                    .map(|(name, property)| {
                        (
                            name.clone(),
                            PropertyBody {
                                value_type: property.value.value.type_label(),
                                data_class: property.value.data_class.label(),
                                value: json_value(&property.value.value),
                            },
                        )
                    })
                    .collect(),
            })
            .into_response()
        }
        Err(ViewError::UnknownObject) => {
            state.metrics.read_refused();
            refuse(
                StatusCode::NOT_FOUND,
                "surface",
                "no applied entry ever bound this object",
            )
        }
        Err(ViewError::UnretainedRevision) => {
            state.metrics.read_refused();
            refuse(
                StatusCode::CONFLICT,
                "surface",
                "that revision was never accepted for this entity type",
            )
        }
    }
}

fn entries_or_refuse(
    state: &AppState,
    tenant: &crate::composition::TenantState,
    tenant_id: &str,
) -> Result<Vec<SealedEnvelope>, Box<Response>> {
    tenant.entries_now(tenant_id).map_err(|_| {
        state.metrics.read_refused();
        Box::new(refuse(
            StatusCode::SERVICE_UNAVAILABLE,
            "log",
            "the action log could not be read; this view is unavailable",
        ))
    })
}

pub async fn history(
    State(state): State<Arc<AppState>>,
    Path(object_ref): Path<String>,
    headers: HeaderMap,
) -> Response {
    let caller = match authorized(&state, &headers, &object_ref) {
        Ok(caller) => caller,
        Err(response) => return *response,
    };
    let tenant = match tenant_of(&state, &caller) {
        Ok(tenant) => tenant,
        Err(response) => return *response,
    };
    let tenant = tenant.lock().await;
    let entries = match entries_or_refuse(&state, &tenant, &caller.tenant_id) {
        Ok(entries) => entries,
        Err(response) => return *response,
    };
    let rows: Vec<HistoryRow> = object_history(&tenant.projection, &entries, &object_ref)
        .into_iter()
        .map(|entry| HistoryRow {
            ordinal: entry.ordinal,
            object_sequence: entry.object_sequence,
            principal_id: entry.principal_id,
            decision_id: entry.decision_id,
            action_type: entry.action_type,
            audit_event_type: entry.audit_event_type,
            schema_revision: entry.schema_revision,
        })
        .collect();
    state.metrics.read_served();
    Json(rows).into_response()
}

pub async fn audit(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let caller = match authorized(&state, &headers, TENANT_SCOPED_RESOURCE) {
        Ok(caller) => caller,
        Err(response) => return *response,
    };
    let tenant = match tenant_of(&state, &caller) {
        Ok(tenant) => tenant,
        Err(response) => return *response,
    };
    let tenant = tenant.lock().await;
    let entries = match entries_or_refuse(&state, &tenant, &caller.tenant_id) {
        Ok(entries) => entries,
        Err(response) => return *response,
    };
    let rows: Vec<AuditRow> = audit_view(&tenant.projection, &entries)
        .into_iter()
        .map(|entry| {
            let (disposition, poison_reason) = match &entry.disposition {
                foundry_spine::AuditDisposition::Applied => ("applied", None),
                foundry_spine::AuditDisposition::Poisoned(reason) => {
                    ("poisoned", Some(format!("{reason:?}")))
                }
            };
            AuditRow {
                ordinal: entry.ordinal,
                object_ref: entry.object_ref,
                action_type: entry.action_type,
                disposition,
                poison_reason,
                principal_id: entry.principal_id,
                decision_id: entry.decision_id,
            }
        })
        .collect();
    state.metrics.read_served();
    Json(rows).into_response()
}

pub async fn types(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let caller = match authorized(&state, &headers, TENANT_SCOPED_RESOURCE) {
        Ok(caller) => caller,
        Err(response) => return *response,
    };
    let tenant = match tenant_of(&state, &caller) {
        Ok(tenant) => tenant,
        Err(response) => return *response,
    };
    let tenant = tenant.lock().await;
    let rows: Vec<EntityTypeRow> =
        crate::seed::declared_entity_types(&tenant.projection.engine, &caller.tenant_id)
            .into_iter()
            .map(|definition| EntityTypeRow {
                entity_type: definition.id.value.clone(),
                revision: definition.revision,
                properties: definition
                    .properties
                    .iter()
                    .map(|property| property.name.clone())
                    .collect(),
            })
            .collect();
    state.metrics.read_served();
    Json(rows).into_response()
}

/// The resource a tenant-wide read is authorized against. It is an
/// `ent_`-shaped id because the kernel's authorization vocabulary requires
/// one; it names the tenant's own view, not any object in it.
pub(crate) const TENANT_SCOPED_RESOURCE: &str = "ent_tenant_view";

pub(crate) fn refuse(status: StatusCode, gate: &str, cause: &str) -> Response {
    (
        status,
        Json(RefusalBody {
            gate: gate.to_owned(),
            cause: cause.to_owned(),
        }),
    )
        .into_response()
}
