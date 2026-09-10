mod facade_support;
mod failing_log;
mod out_of_band;
use facade_support as support;

use axum::http::StatusCode;
use support::{Fixture, Session, scrape, value_of};

#[tokio::test]
async fn a_caught_up_process_reports_fresh() {
    let fixture = Fixture::new("fresh-caught-up");
    let session = Session::from_state(fixture.state());
    let (body, ready) = (scrape(&session).await, session.get(None, "/readyz").await);

    assert_eq!(value_of(&body, "foundry_projection_fresh"), 1);
    assert_eq!(ready.0, StatusCode::OK, "and the probe agrees");
}

#[tokio::test]
async fn a_lagging_process_does_not_report_fresh() {
    let fixture = Fixture::new("fresh-lagging");
    let session = Session::from_state(fixture.state());
    out_of_band::append_for(&fixture.action_log_path(), "ten_acme", "idem_fresh_lag");

    let body = scrape(&session).await;
    assert_eq!(
        value_of(&body, "foundry_projection_fresh"),
        0,
        "a durable entry this process has not folded is not freshness: {body}"
    );
}

#[tokio::test]
async fn an_unobserved_tenant_does_not_report_fresh() {
    let fixture = Fixture::new("fresh-unobserved");
    let session = Session::from_state(failing_log::state_with_a_failing_log(
        &fixture.config(),
        "the head is unreadable",
    ));

    let body = scrape(&session).await;
    assert_eq!(
        (
            value_of(&body, "foundry_projection_fresh"),
            value_of(&body, "foundry_projection_lag"),
            value_of(&body, "foundry_projection_lag_unknown"),
        ),
        (0, 0, 1),
        "lag is zero only because nothing could be read, and freshness must \
         not be claimed from that: {body}"
    );
}

/// The objective and the probe are built from the same observation, not two
/// expressions someone must keep in step.
///
/// This does NOT claim they can never differ: they are separate requests
/// taking separate passes, so a write landing between them changes the answer
/// — that is time passing, not disagreement, as `observation.rs` says. What
/// is pinned is that within one quiescent state they agree, and that they
/// answer from one predicate rather than two.
#[tokio::test]
async fn the_indicator_and_the_probe_read_the_same_predicate() {
    let fixture = Fixture::new("fresh-agrees");
    let session = Session::from_state(fixture.state());

    for expected in [1, 0] {
        if expected == 0 {
            out_of_band::append_for(&fixture.action_log_path(), "ten_acme", "idem_agree");
        }
        let fresh = value_of(&scrape(&session).await, "foundry_projection_fresh");
        let (status, _) = session.get(None, "/readyz").await;
        assert_eq!(fresh, expected);
        assert_eq!(
            status == StatusCode::OK,
            fresh == 1,
            "in one quiescent state the two must agree: fresh={fresh} status={status}"
        );
    }
}

/// A BUSY tenant among others is not an unfresh process.
///
/// `observe` uses `try_lock`, so a request in flight makes that tenant
/// unobservable for the pass. Scoring it unfresh would spend the error budget
/// on concurrency: at 99.9% over thirty days, forty-three minutes consumed by
/// the service being used rather than by anything being stale. `/readyz`
/// still fails closed — one retried 503 is cheap.
///
/// Two tenants, not one. With a single tenant "busy" and "nothing observed"
/// are the same state, and they must not have the same answer.
#[tokio::test]
async fn a_busy_tenant_among_others_is_not_reported_stale() {
    let fixture = Fixture::new("fresh-contended");
    let mut config = fixture.config();
    config.tenants = vec!["ten_acme".into(), "ten_second".into()];
    let state = foundry_ontology_app::compose(&config).expect("boots");
    let held = state
        .tenants
        .get("ten_acme")
        .expect("the served tenant")
        .lock()
        .await;

    let seen = foundry_ontology_app::observation::observe(&state);

    assert_eq!(
        (seen.observed, seen.contended, seen.unreadable),
        (1, 1, 0),
        "one tenant was busy and the other was read: {seen:?}"
    );
    assert!(
        seen.is_fresh(),
        "a lock held by a request in flight is not staleness: {seen:?}"
    );
    assert!(
        !seen.is_caught_up(),
        "readiness still fails closed on it, where one retried 503 is cheap"
    );
    drop(held);
}

#[tokio::test]
async fn a_process_that_observed_nothing_is_not_fresh() {
    let fixture = Fixture::new("fresh-wedged");
    let state = std::sync::Arc::new(fixture.state());
    let session = Session::from_shared(state.clone());
    let held = state
        .tenants
        .get("ten_acme")
        .expect("the served tenant")
        .lock()
        .await;

    let seen = foundry_ontology_app::observation::observe(&state);
    assert_eq!(seen.observed, 0, "no tenant was read at all: {seen:?}");
    assert!(
        !seen.is_fresh(),
        "a claim about every tenant we could read is empty when we read none"
    );

    let body = scrape(&session).await;
    assert_eq!(
        value_of(&body, "foundry_projection_fresh"),
        0,
        "and the wire says so, which is where an objective reads it: {body}"
    );
    drop(held);
}

#[tokio::test]
async fn the_exposition_distinguishes_busy_from_unreadable() {
    let fixture = Fixture::new("fresh-split-causes");
    let mut config = fixture.config();
    config.tenants = vec!["ten_acme".into(), "ten_second".into()];
    let state = std::sync::Arc::new(foundry_ontology_app::compose(&config).expect("boots"));
    let session = Session::from_shared(state.clone());
    let held = state.tenants.get("ten_acme").unwrap().lock().await;

    let body = scrape(&session).await;

    assert_eq!(
        (
            value_of(&body, "foundry_projection_contended"),
            value_of(&body, "foundry_projection_lag_unknown"),
            value_of(&body, "foundry_projection_fresh"),
        ),
        (1, 1, 1),
        "one busy, none unreadable, so the remainder is zero and the process \
         is fresh: {body}"
    );
    drop(held);
}
