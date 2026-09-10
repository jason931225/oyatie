#[path = "facade_support/mod.rs"]
mod support;

use axum::http::StatusCode;
use foundry_ontology_app::{PepError, PolicyEnforcementPoint};
use support::{Fixture, post};

#[test]
fn an_uncompilable_policy_version_refuses_to_load() {
    assert!(
        matches!(
            PolicyEnforcementPoint::load(""),
            Err(PepError::BundleRejected { .. })
        ),
        "the process never serves a policy set it could not validate"
    );
}

#[tokio::test]
async fn a_cross_tenant_write_is_refused_and_recorded_on_the_denial_trail() {
    let fixture = Fixture::new("outage-cross-tenant");
    // The credential names tenant B; the seeded objects belong to tenant A.
    let (status, _) = post(
        &fixture,
        Some(fixture.foreign_token()),
        r#"{"object_ref":"ent_alpha","action_type":"aty_record_write","idempotency_key":"idem_1","occurred_at_epoch_seconds":1700000000,"properties":{"name":"Ada"}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        fixture.log_head(),
        0,
        "a refused submission never reaches the action log"
    );
}

#[tokio::test]
async fn a_roleless_operator_is_refused_by_absence_of_a_permit() {
    let fixture = Fixture::new("outage-roleless");
    let (status, _) = post(
        &fixture,
        Some(fixture.roleless_token()),
        r#"{"object_ref":"ent_alpha","action_type":"aty_record_write","idempotency_key":"idem_1","occurred_at_epoch_seconds":1700000000,"properties":{"name":"Ada"}}"#,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "deny-by-default: no permit reaches a principal outside the operator role"
    );
    assert_eq!(fixture.log_head(), 0);
}
