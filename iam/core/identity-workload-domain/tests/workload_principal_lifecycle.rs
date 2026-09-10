#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! End-to-end exercise of the workload-identity domain kernel, from provision
//! through to an assembled authorization request.

use iam_identity_workload_domain::{
    Action, AuthorizationDecision, AuthorizationRequest, ClaimValue, Effect, Resource,
    WorkloadIdentityError, WorkloadPrincipal, WorkloadState,
};

#[test]
fn provisioned_workload_walks_to_active_then_serves_authz_requests() {
    let principal = WorkloadPrincipal::provision("ten_globex", "wl_payments_api", "cap.payments")
        .expect("valid ids");
    assert_eq!(principal.state(), WorkloadState::Provisioned);

    let mut principal = principal;
    principal
        .transition_to(WorkloadState::Active)
        .expect("provisioned -> active is legal");

    let principal = principal
        .with_claim("iss", ClaimValue::Text("https://idp.oyatie.com".into()))
        .expect("claim ok")
        .with_claim("trust_tier", ClaimValue::Int(2))
        .expect("claim ok")
        .with_scope("payments.ledger.read")
        .expect("scope ok");

    assert!(principal.state().is_operational());
    assert!(principal.has_scope("payments.ledger.read"));
    assert_eq!(
        principal.claim("iss").and_then(ClaimValue::as_text),
        Some("https://idp.oyatie.com")
    );

    let request = AuthorizationRequest::new(
        principal,
        Action::new("payments.ledger.Read"),
        Resource::new("Ledger", "2026-q2"),
    )
    .with_context("mfa", ClaimValue::Bool(true));

    assert_eq!(request.action.as_str(), "payments.ledger.Read");
    assert_eq!(request.resource.resource_id(), "2026-q2");
    assert!(
        request
            .context
            .get("mfa")
            .map(|claim| claim.contains("true") || matches!(claim, ClaimValue::Bool(true)))
            .unwrap_or(false)
    );
}

#[test]
fn retired_workload_is_terminal_and_not_operational() {
    let mut principal =
        WorkloadPrincipal::provision("ten_globex", "wl_old", "cap.x").expect("valid");
    principal
        .transition_to(WorkloadState::Active)
        .expect("activate");
    principal
        .transition_to(WorkloadState::Retired)
        .expect("retire");

    assert!(!principal.state().is_operational());
    assert!(matches!(
        principal.transition_to(WorkloadState::Active),
        Err(WorkloadIdentityError::IllegalStateTransition { .. })
    ));
}

#[test]
fn decision_effect_helpers_round_trip() {
    assert_eq!(AuthorizationDecision::permit("p").effect(), Effect::Allow);
    assert!(AuthorizationDecision::permit("p").is_allow());
    assert!(!AuthorizationDecision::default_deny().is_allow());
}
