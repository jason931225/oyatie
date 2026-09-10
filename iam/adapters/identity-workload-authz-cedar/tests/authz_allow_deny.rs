#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Black-box allow/deny coverage for the Cedar workload authz gate.

use iam_identity_workload_authz_cedar::{
    ActionCondition, CedarWorkloadAuthorizer, Policy, PrincipalCondition, ResourceCondition,
    WorkloadAuthorizer,
};
use iam_identity_workload_domain::{
    Action, AuthorizationRequest, ClaimValue, Effect, Resource, WorkloadPrincipal, WorkloadState,
};

fn kms_reader() -> WorkloadPrincipal {
    let mut principal =
        WorkloadPrincipal::provision("ten_acme", "wl_secrets_sync", "cap.cloud.kms")
            .expect("valid ids");
    principal
        .transition_to(WorkloadState::Active)
        .expect("activate");
    principal
        .with_scope("cloud.kms.decrypt")
        .expect("scope ok")
        .with_claim("env", ClaimValue::Text("prod".into()))
        .expect("claim ok")
}

fn decrypt_request(principal: WorkloadPrincipal) -> AuthorizationRequest {
    AuthorizationRequest::new(
        principal,
        Action::new("cloud.kms.Decrypt"),
        Resource::new("Secret", "prod/db-password"),
    )
}

fn production_policy_set() -> CedarWorkloadAuthorizer {
    CedarWorkloadAuthorizer::with_policies(vec![
        // permit: acme workloads with the decrypt scope may decrypt Secrets.
        Policy::permit("permit-kms-decrypt")
            .when_principal(PrincipalCondition::TenantIs("ten_acme".into()))
            .when_principal(PrincipalCondition::HasScope("cloud.kms.decrypt".into()))
            .for_action(ActionCondition::Equals("cloud.kms.Decrypt".into()))
            .for_resource(ResourceCondition::TypeIs("Secret".into())),
        // forbid: nobody touches the break-glass root secret.
        Policy::forbid("forbid-root-secret").for_resource(ResourceCondition::Is {
            resource_type: "Secret".into(),
            resource_id: "prod/root-of-trust".into(),
        }),
    ])
    .expect("production policy set compiles to valid cedar")
}

#[test]
fn authorized_workload_is_allowed() {
    let decision = production_policy_set().authorize(&decrypt_request(kms_reader()));
    assert!(decision.is_allow(), "expected allow, got {decision:?}");
}

#[test]
fn unrelated_action_is_denied_by_default() {
    let request = AuthorizationRequest::new(
        kms_reader(),
        Action::new("cloud.kms.Encrypt"), // policy only permits Decrypt
        Resource::new("Secret", "prod/db-password"),
    );
    let decision = production_policy_set().authorize(&request);
    assert_eq!(decision.effect(), Effect::Deny);
}

#[test]
fn forbidden_resource_overrides_matching_permit() {
    let request = AuthorizationRequest::new(
        kms_reader(),
        Action::new("cloud.kms.Decrypt"),
        Resource::new("Secret", "prod/root-of-trust"),
    );
    let decision = production_policy_set().authorize(&request);
    assert_eq!(
        decision.effect(),
        Effect::Deny,
        "forbid must win over the matching permit"
    );
}

#[test]
fn retired_workload_cannot_be_authorized() {
    let mut principal = kms_reader();
    principal
        .transition_to(WorkloadState::Retired)
        .expect("retire");
    let decision = production_policy_set().authorize(&decrypt_request(principal));
    assert_eq!(decision.effect(), Effect::Deny);
}
