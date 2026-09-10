//! REST decision surface (axum).
//!
//! A `200` carries a decision, allow or deny alike. Any other status is a
//! REFUSAL to decide, which a PEP must itself treat as deny.

use std::sync::Arc;

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use os_trustd_domain::TrustBundle;
use os_trustd_domain::signer::EcdsaP256Signer;
use shared_pdp_kernel::{EntityRecord, EntitySlice, PdpError};
use shared_platform_contracts_kernel::pdp::AuthorizationRequest;

use crate::PdpState;
use crate::mtls::SpiffeCallerAuth;
use crate::mtls_transport::PeerCertInfo;

/// `POST /v1/authorize` body: the locked-contract request plus the
/// PEP-assembled entity slice.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthorizeBody {
    request: AuthorizationRequest,
    #[serde(default)]
    entities: Vec<EntityRecord>,
}

fn refusal_parts(error: &PdpError) -> (StatusCode, &'static str) {
    match error {
        PdpError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid_request"),
        PdpError::UnknownAction { .. } => (StatusCode::BAD_REQUEST, "unknown_action"),
        PdpError::StalePolicyVersion { .. } => (StatusCode::CONFLICT, "stale_policy_version"),
        PdpError::Evaluation { .. } => (StatusCode::UNPROCESSABLE_ENTITY, "evaluation_refused"),
        PdpError::BundleRejected { .. } => (StatusCode::SERVICE_UNAVAILABLE, "bundle_rejected"),
        PdpError::DecisionIdUnavailable { .. } => {
            (StatusCode::INTERNAL_SERVER_ERROR, "decision_id_unavailable")
        }
        PdpError::AuditChainEmission { .. } => {
            (StatusCode::INTERNAL_SERVER_ERROR, "audit_chain_emission")
        }
        PdpError::RuntimeTimeout { .. } => (StatusCode::GATEWAY_TIMEOUT, "runtime_timeout"),
        PdpError::RuntimePanic { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "runtime_panic"),
        PdpError::CircuitOpen { .. } => (StatusCode::SERVICE_UNAVAILABLE, "circuit_open"),
    }
}

fn refusal_response(error: &PdpError) -> Response {
    let (status, error_code) = refusal_parts(error);
    (
        status,
        Json(serde_json::json!({
            "error_code": error_code,
            "detail": error.to_string(),
        })),
    )
        .into_response()
}

/// Layered per connection. Its ABSENCE is what selects the plain-TCP path, on
/// which the request's tenant is taken verbatim.
type CallerAuthBundle = Arc<TrustBundle<EcdsaP256Signer>>;

/// The PDP's own cell authority, as a type distinct from the bundle so both can
/// be `Extension`s. Inner `None` = the server leaf carries no SPIFFE id, so
/// there is no cell to pin a caller against.
#[derive(Clone)]
struct CallerAuthCell(Option<String>);

/// `0` on a pre-epoch clock, which fails every SVID's validity window rather
/// than accepting one.
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Coarse by design: a 404 or a specific reason would let a caller enumerate
/// tenants and SVIDs.
fn caller_auth_refusal(message: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({
            "error_code": "caller_unauthenticated",
            "detail": message,
        })),
    )
        .into_response()
}

/// Replace the caller-asserted tenant with the one bound from its verified peer
/// SVID, so a caller can never name a tenant it does not hold.
fn bind_tenant_from_peer_svid(
    bundle: &TrustBundle<EcdsaP256Signer>,
    expected_cell: Option<&str>,
    peer_leaf: Option<&[u8]>,
    tenant_id: &mut String,
) -> Result<(), Box<Response>> {
    let pep = match expected_cell {
        Some(cell) => SpiffeCallerAuth::with_cell_pin(bundle, cell),
        None => SpiffeCallerAuth::new(bundle),
    }
    .map_err(|err| Box::new(caller_auth_refusal(&err.to_string())))?;
    let bound = pep
        .authenticate_caller(peer_leaf, tenant_id, now_secs())
        .map_err(|rej| Box::new(caller_auth_refusal(rej.public_message())))?;
    *tenant_id = bound.as_str().to_owned();
    Ok(())
}

async fn authorize(
    State(state): State<Arc<PdpState>>,
    bundle: Option<Extension<CallerAuthBundle>>,
    cell: Option<Extension<CallerAuthCell>>,
    peer: Option<Extension<PeerCertInfo>>,
    Json(mut body): Json<AuthorizeBody>,
) -> Response {
    if let Some(Extension(bundle)) = bundle {
        let peer_leaf = peer
            .as_ref()
            .and_then(|Extension(info)| info.leaf_der.as_deref());
        let expected_cell = cell.and_then(|Extension(c)| c.0);
        if let Err(refusal) = bind_tenant_from_peer_svid(
            &bundle,
            expected_cell.as_deref(),
            peer_leaf,
            &mut body.request.tenant_id,
        ) {
            return *refusal;
        }
    }

    let entities = EntitySlice {
        entities: body.entities,
    };
    match state.decide(&body.request, &entities) {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(error) => {
            PdpState::log_refusal(&body.request.request_id, &error);
            refusal_response(&error)
        }
    }
}

async fn healthz() -> Response {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}

async fn readyz(State(state): State<Arc<PdpState>>) -> Response {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ready",
            "policy_version": state.loaded_policy_version().as_str(),
        })),
    )
        .into_response()
}

async fn unknown_route() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error_code": "unknown_route",
            "detail": "no such surface; the decision surface is POST /v1/authorize",
        })),
    )
        .into_response()
}

/// Plain-TCP boot: this router does NOT authenticate the caller and takes the
/// request's tenant verbatim.
pub fn build_router(state: Arc<PdpState>) -> Router {
    Router::new()
        .route("/v1/authorize", post(authorize))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .fallback(unknown_route)
        .with_state(state)
}

/// As [`build_router`], with the mTLS PEP enforced. The per-connection peer leaf
/// is injected separately, by the accept loop in [`crate::mtls_transport`].
pub fn build_router_mtls(
    state: Arc<PdpState>,
    bundle: Arc<TrustBundle<EcdsaP256Signer>>,
    expected_cell_authority: Option<String>,
) -> Router {
    build_router(state)
        .layer(Extension(CallerAuthCell(expected_cell_authority)))
        .layer(Extension(bundle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_refusals_have_fail_closed_rest_mappings() {
        assert_eq!(
            refusal_parts(&PdpError::RuntimeTimeout { deadline_ms: 25 }),
            (StatusCode::GATEWAY_TIMEOUT, "runtime_timeout")
        );
        assert_eq!(
            refusal_parts(&PdpError::RuntimePanic {
                detail: "panic".to_owned(),
            }),
            (StatusCode::INTERNAL_SERVER_ERROR, "runtime_panic")
        );
        assert_eq!(
            refusal_parts(&PdpError::AuditChainEmission {
                detail: "append failed".to_owned(),
            }),
            (StatusCode::INTERNAL_SERVER_ERROR, "audit_chain_emission")
        );
        assert_eq!(
            refusal_parts(&PdpError::CircuitOpen {
                consecutive_failures: 3,
            }),
            (StatusCode::SERVICE_UNAVAILABLE, "circuit_open")
        );
    }
}
