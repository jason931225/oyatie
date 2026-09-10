mod facade_support;
mod failing_log;
mod out_of_band;
use facade_support as support;

use axum::http::StatusCode;
use support::{Fixture, Session, scrape, value_of};

#[tokio::test]
async fn an_operator_reads_the_status_surface() {
    let fixture = Fixture::new("statusz-operator");
    let session = fixture.session();

    let (status, body) = session
        .get(Some(fixture.operator_token()), "/statusz")
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    // VALUES, not field names. A key present with any value satisfies a
    // `contains` of its name, so that form passed over a body whose numbers
    // were all zero and whose type list was empty — which is exactly what
    // this surface looks like when it has stopped reading anything.
    assert!(
        body.contains(r#""served_tenants":1"#) && body.contains(r#""observed_tenants":1"#),
        "the one served tenant must be served AND observed: {body}"
    );
    assert!(
        body.contains(r#""projection_lag":0"#) && body.contains(r#""poisoned_entries":0"#),
        "a fresh process is caught up and unpoisoned: {body}"
    );
    assert!(
        body.contains(r#""entity_types":["ety_record"]"#),
        "the caller's own declared types, not an empty list: {body}"
    );
    // The TOKEN, not its Debug rendering: the field shipped as
    // `PolicyVersion("psv-000001")`, Rust syntax in a JSON body.
    //
    // This pins the RENDERING and cannot pin the SOURCE. A mutant hardcoding
    // the correct token passes, and no test here can separate the two,
    // because the version is a compile-time constant with exactly one value
    // — there is no state in which the loaded version differs from the
    // literal. Said plainly rather than left for someone to discover: a
    // config-varied policy version would close it, and does not exist.
    assert!(
        body.contains(r#""policy_version":"psv-000001""#),
        "the loaded version must be the token itself: {body}"
    );
}

#[tokio::test]
async fn a_roleless_caller_is_refused_the_status_surface() {
    let fixture = Fixture::new("statusz-roleless");
    let session = fixture.session();

    let (status, body) = session
        .get(Some(fixture.roleless_token()), "/statusz")
        .await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}

#[tokio::test]
async fn the_status_surface_reports_a_lag_that_moves() {
    let fixture = Fixture::new("statusz-lag");
    let session = fixture.session();

    let (_, before) = session
        .get(Some(fixture.operator_token()), "/statusz")
        .await;
    assert!(before.contains("\"projection_lag\":0"), "{before}");

    out_of_band::append_for(&fixture.action_log_path(), "ten_acme", "idem_statusz");

    let (_, after) = session
        .get(Some(fixture.operator_token()), "/statusz")
        .await;
    assert!(
        after.contains("\"projection_lag\":1"),
        "a durable entry this process has not folded must show here: {after}"
    );
}

/// And the poison count is read, not assumed zero.
///
/// Asserting zero on a clean fixture is satisfied by a hardcoded zero — the
/// value has to be driven off it somewhere, or the field is decoration. The
/// entry is seeded BEFORE boot so the fold consumes and refuses it; the same
/// bytes appended after boot would be lag instead.
#[tokio::test]
async fn the_status_surface_reports_a_poison_it_actually_has() {
    let fixture = Fixture::new("statusz-poison");
    out_of_band::append_for(
        &fixture.action_log_path(),
        "ten_acme",
        "idem_statusz_poison",
    );
    let session = Session::from_state(
        foundry_ontology_app::compose(&fixture.config()).expect("boots over a poisoned log"),
    );

    let (status, body) = session
        .get(Some(fixture.operator_token()), "/statusz")
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains(r#""poisoned_entries":1"#),
        "the fold consumed one entry and refused it, and the status says so: {body}"
    );
    assert!(
        body.contains(r#""projection_lag":0"#),
        "a poison advances the fold, so it is consumed rather than pending: {body}"
    );
}

#[tokio::test]
async fn a_contended_tenant_is_reported_not_waited_on() {
    let fixture = Fixture::new("statusz-contended");
    let state = std::sync::Arc::new(fixture.state());
    let session = Session::from_shared(state.clone());
    let held = state
        .tenants
        .get("ten_acme")
        .expect("the served tenant")
        .lock()
        .await;

    let (status, body) = session
        .get(Some(fixture.operator_token()), "/statusz")
        .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "it answers rather than hanging: {body}"
    );
    assert!(
        body.contains(r#""contended_tenants":1"#) && body.contains(r#""observed_tenants":0"#),
        "the tenant was busy and therefore unread, and the status says both: {body}"
    );
    assert!(
        body.contains(r#""entity_types":null"#),
        "null because it could not be read — an empty list would claim the \
         tenant declares none, which the seed makes impossible: {body}"
    );
    drop(held);
}

#[tokio::test]
async fn an_unreadable_store_is_reported_as_such() {
    let fixture = Fixture::new("statusz-unreadable");
    let session = Session::from_state(failing_log::state_with_a_failing_log(
        &fixture.config(),
        "the head is gone",
    ));

    let (status, body) = session
        .get(Some(fixture.operator_token()), "/statusz")
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains(r#""unreadable_tenants":1"#)
            && body.contains(r#""contended_tenants":0"#)
            && body.contains(r#""observed_tenants":0"#),
        "unreadable, not contended, and therefore unobserved: {body}"
    );
}

#[tokio::test]
async fn a_credential_for_an_unserved_tenant_is_refused() {
    let fixture = Fixture::new("statusz-unserved");
    let session = fixture.unserved_session();

    let (status, body) = session
        .get(Some(fixture.operator_token()), "/statusz")
        .await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(
        body.contains("does not serve"),
        "refused for the roster reason, the same as /v1/types: {body}"
    );
}

#[tokio::test]
async fn a_foreign_tenant_is_refused_the_status_surface() {
    let fixture = Fixture::new("statusz-foreign");
    let session = fixture.session();

    let (status, body) = session.get(Some(fixture.foreign_token()), "/statusz").await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(
        !body.contains("ety_record") && !body.contains("served_tenants"),
        "and it leaks no field of the status it was refused: {body}"
    );
}

/// `served_tenants` counts the roster, and `entity_types` is the CALLER'S.
///
/// Every other fixture here serves one tenant, where "the roster size" and
/// "one" are the same number, and "the caller's engine" and "the only
/// engine" are the same object. Two tenants separate both.
#[tokio::test]
async fn the_status_counts_the_roster_and_reads_the_callers_own_tenant() {
    let fixture = Fixture::new("statusz-two-tenants");
    let mut config = fixture.config();
    config.tenants = vec!["ten_acme".into(), "ten_second".into()];
    let session = Session::from_state(foundry_ontology_app::compose(&config).expect("boots"));

    let (status, body) = session
        .get(Some(fixture.operator_token()), "/statusz")
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains(r#""served_tenants":2"#) && body.contains(r#""observed_tenants":2"#),
        "both tenants are served and both were read: {body}"
    );
    assert!(
        body.contains(r#""entity_types":["ety_record"]"#),
        "the caller's own tenant's types: {body}"
    );
}

/// The status answer counts as a served read, deliberately.
///
/// It is an authenticated, authorized surface on the read plane, so its
/// availability is part of the read surface's availability, and a refusal
/// here already counts via `authorized`. Counting only the refusals would
/// depress the ratio for a surface that answered.
#[tokio::test]
async fn a_served_status_counts_as_a_served_read() {
    let fixture = Fixture::new("statusz-counts");
    let session = fixture.session();

    let before = value_of(&scrape(&session).await, "foundry_read_served_total");
    let (status, _) = session
        .get(Some(fixture.operator_token()), "/statusz")
        .await;
    assert_eq!(status, StatusCode::OK);
    let after = value_of(&scrape(&session).await, "foundry_read_served_total");

    assert_eq!(
        after,
        before + 1,
        "a status answer is a served read, counted once"
    );
}
