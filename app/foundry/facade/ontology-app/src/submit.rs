use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use data_ontology_kernel::{ActionInvocationRequest, ActionTypeId};
use foundry_edits::{EditSet, OntologyEdit, WireDataClass, WireProperty, WireTier, WireValue};
use foundry_records_draft::RecordsLogError;
use foundry_spine::{ActionSubmission, ApplyOutcome, WriteError, submit};

use crate::auth::{authenticate, bearer_token};
use crate::composition::AppState;
use crate::dto::{RefusalBody, SubmitRequest, SubmitResponse};
use crate::pdp::Surface;

pub async fn submit_action(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let Some(token) = bearer_token(
        headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
    ) else {
        state.metrics.submit_refused();
        return refuse(
            StatusCode::UNAUTHORIZED,
            "credential",
            "no bearer credential",
        );
    };
    let Some(caller) = authenticate(&state.operators, token) else {
        state.metrics.submit_refused();
        return refuse(
            StatusCode::UNAUTHORIZED,
            "credential",
            "the presented credential is not recognized",
        );
    };
    let Ok(request) = serde_json::from_str::<SubmitRequest>(&body) else {
        state.metrics.submit_refused();
        return refuse(
            StatusCode::BAD_REQUEST,
            "surface",
            "the request body is not a well-formed submission",
        );
    };
    let Ok(action_id) = ActionTypeId::new(request.action_type.clone()) else {
        state.metrics.submit_refused();
        return refuse(
            StatusCode::BAD_REQUEST,
            "surface",
            "the action type is not an action id",
        );
    };

    // AUTHORIZE before anything is built: the decision is the PDP's, and a
    // refusal ends the request here, with nothing appended anywhere.
    let Ok(decision) = state
        .pep
        .decide(&caller, Surface::Invoke, &request.object_ref)
    else {
        state.metrics.submit_refused();
        return refuse(
            StatusCode::FORBIDDEN,
            "authorization",
            "the policy decision point refused this invocation",
        );
    };

    // The tenant is the CREDENTIAL's. Nothing in the body can move it.
    let Some(tenant) = state.tenants.get(&caller.tenant_id) else {
        state.metrics.submit_refused();
        return refuse(
            StatusCode::FORBIDDEN,
            "authorization",
            "the credential names a tenant this process does not serve",
        );
    };
    let mut tenant = tenant.lock().await;

    let Ok(edits) = edits_for(&request) else {
        state.metrics.submit_refused();
        return refuse(
            StatusCode::BAD_REQUEST,
            "surface",
            "the submission carries no representable edit",
        );
    };
    let submission = ActionSubmission {
        request: ActionInvocationRequest {
            tenant_id: caller.tenant_id.clone(),
            principal_id: caller.principal_id.clone(),
            action_id,
            entity_id: request.object_ref.clone(),
            idempotency_key: request.idempotency_key.clone(),
            requested_at_epoch_seconds: request.occurred_at_epoch_seconds,
        },
        decision,
        parameters: Vec::new(),
        edits,
    };

    let (log, denial_log, projection) = tenant.write_handles();
    let outcome = submit(submission, log, denial_log, projection);
    match &outcome {
        Ok(_) => state.metrics.submit_served(),
        Err(_) => state.metrics.submit_refused(),
    }
    match outcome {
        Ok(ApplyOutcome::Applied { receipt }) => Json(SubmitResponse {
            outcome: "applied",
            ordinal: receipt.ordinal,
            deduplicated: receipt.deduplicated,
            poison_reason: None,
        })
        .into_response(),
        Ok(ApplyOutcome::Poisoned { receipt, reason }) => Json(SubmitResponse {
            outcome: "poisoned",
            ordinal: receipt.ordinal,
            deduplicated: receipt.deduplicated,
            poison_reason: Some(format!("{reason:?}")),
        })
        .into_response(),
        Err(WriteError::Refused(refused)) => {
            refuse(StatusCode::FORBIDDEN, refused.gate.label(), refused.cause)
        }
        // A divergent reuse of a spent key is the caller's conflict to
        // resolve, not something to retry into.
        Err(WriteError::Log(RecordsLogError::IdempotencyConflict { .. })) => refuse(
            StatusCode::CONFLICT,
            "log",
            "this idempotency key is already spent on different content",
        ),
        // The detail is deliberately not echoed: unlike the causes the other
        // arms echo, `rusqlite::Error` is unbounded and path-bearing.
        Err(WriteError::Log(RecordsLogError::Storage { .. })) => refuse(
            StatusCode::SERVICE_UNAVAILABLE,
            "log",
            "the action log could not be written; the submission was not accepted",
        ),
    }
}

/// The edit set is a PURE FUNCTION OF THE REQUEST, and that is the retry
/// contract, not a simplification. An earlier version chose the edit kind
/// by asking whether the projection already held the object — which made
/// the same request produce different bytes before and after it landed, so
/// a retry arrived as divergent content under a spent key and conflicted
/// instead of deduplicating. Nothing about how the payload is built may
/// depend on state the request cannot see.
fn edits_for(request: &SubmitRequest) -> Result<EditSet, ()> {
    let properties: Vec<WireProperty> = request
        .properties
        .iter()
        .filter_map(|(name, value)| {
            WireProperty::new(
                name,
                WireTier::Scalar,
                WireDataClass::InternalOnly,
                WireValue::String(value.clone()),
            )
            .ok()
        })
        .collect();
    if properties.len() != request.properties.len() {
        return Err(());
    }
    let edit = OntologyEdit::create_object(SEEDED_ENTITY_TYPE, properties).map_err(|_| ())?;
    EditSet::new(vec![edit]).map_err(|_| ())
}

const SEEDED_ENTITY_TYPE: &str = "ety_record";

fn refuse(status: StatusCode, gate: &str, cause: &str) -> Response {
    (
        status,
        Json(RefusalBody {
            gate: gate.to_owned(),
            cause: cause.to_owned(),
        }),
    )
        .into_response()
}
