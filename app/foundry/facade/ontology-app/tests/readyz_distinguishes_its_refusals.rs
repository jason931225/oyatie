mod facade_support;
mod failing_log;
mod out_of_band;
use facade_support as support;

use axum::http::StatusCode;
use support::{Fixture, Session};

#[tokio::test]
async fn an_unreadable_head_is_not_ready() {
    let fixture = Fixture::new("lag-unreadable-readyz");
    let state = failing_log::state_with_a_failing_log(&fixture.config(), "the head is gone");

    assert!(
        !foundry_ontology_app::observation::observe(&state).is_caught_up(),
        "a tenant whose head cannot be read must not report ready"
    );

    let session = Session::from_state(failing_log::state_with_a_failing_log(
        &fixture.config(),
        "the head is gone",
    ));
    let (status, body) = session.get(None, "/readyz").await;
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "the probe must refuse, not answer ready over an unreadable log"
    );
    assert_eq!(
        body, "unobserved\n",
        "a tenant nobody could read is not a tenant known to be behind"
    );
}

#[tokio::test]
async fn a_lagging_process_says_lagging() {
    let fixture = Fixture::new("readyz-lagging");
    let session = Session::from_state(fixture.state());
    let (ready, body) = session.get(None, "/readyz").await;
    assert_eq!(ready, StatusCode::OK);
    assert_eq!(body, "ready\n");

    out_of_band::append_for(&fixture.action_log_path(), "ten_acme", "idem_lagging_probe");

    let (status, body) = session.get(None, "/readyz").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body, "lagging\n",
        "this tenant was read, and it is behind: that is a different answer"
    );
}

#[tokio::test]
async fn a_contended_tenant_says_contended() {
    let fixture = Fixture::new("readyz-contended");
    let state = std::sync::Arc::new(fixture.state());
    let session = Session::from_shared(state.clone());
    let held = state
        .tenants
        .get("ten_acme")
        .expect("the served tenant")
        .lock()
        .await;

    let (status, body) = session.get(None, "/readyz").await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body, "contended\n",
        "busy is neither behind nor unreadable, and readiness says which"
    );
    drop(held);
}

/// A measured fault outranks a non-fault.
///
/// Lagging and contended together must say lagging: the process observed a
/// tenant behind its log, and "contended" is the one word here that means
/// nothing is wrong. Without this, the branch order is incidental — every
/// other test in this file drives exactly one cause, so all of them pass
/// under any permutation.
#[tokio::test]
async fn lagging_outranks_contended() {
    let fixture = Fixture::new("readyz-lag-and-busy");
    let mut config = fixture.config();
    config.tenants = vec!["ten_acme".into(), "ten_second".into()];
    let state = std::sync::Arc::new(foundry_ontology_app::compose(&config).expect("boots"));
    let session = Session::from_shared(state.clone());
    out_of_band::append_for(&fixture.action_log_path(), "ten_acme", "idem_lag_and_busy");
    let held = state.tenants.get("ten_second").unwrap().lock().await;

    let seen = foundry_ontology_app::observation::observe(&state);
    assert_eq!(
        (seen.lag, seen.contended),
        (1, 1),
        "the precondition: both causes are live, or the priority is untested"
    );

    let (status, body) = session.get(None, "/readyz").await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body, "lagging\n",
        "one tenant is measurably behind; the other merely being busy does not \
         downgrade that to a non-fault"
    );
    drop(held);
}

/// And an unreadable store outranks a lag.
///
/// A head nobody can read means the lag figure is not trustworthy in the
/// first place, so it is reported before any number derived from it.
///
/// BOTH causes must be live for this to say anything about priority. A
/// whole-roster fault cannot produce that: an unreadable tenant contributes
/// no lag, so the state collapses to a single cause and the test passes
/// under any order. One tenant read and behind, one unreadable.
#[tokio::test]
async fn unobserved_outranks_lagging() {
    let fixture = Fixture::new("readyz-unreadable-and-lag");
    let mut config = fixture.config();
    config.tenants = vec!["ten_acme".into(), "ten_second".into()];
    let state = std::sync::Arc::new(failing_log::state_with_one_failing_tenant(
        &config,
        "ten_second",
        "the head is gone",
    ));
    // AFTER compose. These bytes do not decode, so a boot fold would consume
    // them as a poison and advance past them, leaving no lag at all.
    out_of_band::append_for(
        &fixture.action_log_path(),
        "ten_acme",
        "idem_unreadable_and_lag",
    );
    let session = Session::from_shared(state.clone());

    let seen = foundry_ontology_app::observation::observe(&state);
    assert_eq!(
        (seen.lag, seen.unreadable),
        (1, 1),
        "the precondition: both causes are live, or the priority is untested"
    );

    let (status, body) = session.get(None, "/readyz").await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body, "unobserved\n",
        "a head that cannot be read makes any lag derived from it untrustworthy"
    );
}
