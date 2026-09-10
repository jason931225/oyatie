mod facade_support;
mod migration_support;

use axum::http::StatusCode;
use facade_support::{Fixture, Session, scrape, value_of};
use migration_support::{
    action_head, attest, plan_for, run, state_with_two_revisions, upcast_row, write_owing,
    write_settled,
};

#[tokio::test]
async fn an_operator_runs_a_plan_to_its_fixpoint() {
    let fixture = Fixture::new("run-fixpoint");
    let config = fixture.config();
    let session = Session::from_state(state_with_two_revisions(&config));
    write_owing(
        &session,
        Some(fixture.operator_token()),
        "ent_alpha",
        "idem_1",
    )
    .await;
    // A SECOND object that owes nothing. `total` counts the population of the
    // plan's entity type and `upcast` counts the work; with one object the two
    // numbers coincide, so a `total` fixed at the number owed reads correct
    // forever. They must be able to disagree for either to mean anything.
    write_settled(
        &session,
        Some(fixture.operator_token()),
        "ent_beta",
        "idem_2",
    )
    .await;

    let (status, body) = run(
        &session,
        Some(fixture.operator_token()),
        &plan_for("ten_acme"),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        r#"{"total":2,"upcast":1,"pending":0,"refused":0,"conflicted":0,"unavailable":0,"poisoned":0,"fixpoint":true}"#
    );
    // The attestation is the independent witness: the surface's own report
    // could say anything, but a plan at its fixpoint owes nothing.
    let (_, attested) = attest(
        &session,
        Some(fixture.operator_token()),
        &plan_for("ten_acme"),
    )
    .await;
    assert!(
        attested.contains(r#""fixpoint":true"#),
        "after the run the plan owes nothing: {attested}"
    );
}

#[tokio::test]
async fn the_upcast_carries_the_decision_the_pdp_minted_for_this_run() {
    let fixture = Fixture::new("run-attribution");
    let config = fixture.config();
    let session = Session::from_state(state_with_two_revisions(&config));
    let token = Some(fixture.operator_token());

    write_owing(&session, token, "ent_alpha", "idem_1").await;
    let (status, body) = run(&session, token, &plan_for("ten_acme")).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (decision, principal) = upcast_row(&session, token, "ent_alpha").await;
    assert_eq!(
        decision,
        nth_decision(2),
        "the run is the SECOND authorization this process made, and the upcast \
         must carry that decision rather than one it invented"
    );
    assert_eq!(
        principal, "prn_alice",
        "attributed to the principal that asked"
    );
}

const SEEDED_ID_PREFIX: &str = "01hmz";
const SEEDED_ID_COUNTER_WIDTH: usize = 21;

fn nth_decision(n: u64) -> String {
    format!(
        "{SEEDED_ID_PREFIX}{n:0width$}",
        width = SEEDED_ID_COUNTER_WIDTH
    )
}

#[tokio::test]
async fn a_second_run_at_the_fixpoint_writes_nothing() {
    let fixture = Fixture::new("run-idempotent");
    let config = fixture.config();
    let session = Session::from_state(state_with_two_revisions(&config));
    write_owing(
        &session,
        Some(fixture.operator_token()),
        "ent_alpha",
        "idem_1",
    )
    .await;
    let token = Some(fixture.operator_token());
    let (first, body) = run(&session, token, &plan_for("ten_acme")).await;
    assert_eq!(first, StatusCode::OK, "{body}");
    let settled = action_head(&config);

    let (second, again) = run(&session, token, &plan_for("ten_acme")).await;

    assert_eq!(second, StatusCode::OK, "{again}");
    assert!(
        again.contains(r#""upcast":0"#),
        "a settled population owes nothing to upcast: {again}"
    );
    assert_eq!(
        action_head(&config),
        settled,
        "and the log did not grow: {again}"
    );
}
