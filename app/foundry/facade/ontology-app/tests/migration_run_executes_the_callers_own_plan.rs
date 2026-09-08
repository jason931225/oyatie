//! `POST /v1/migrations/run` — execute a plan to its fixpoint.
//!
//! The executing half. Unlike attest this WRITES, so it is gated on `Invoke`
//! and every refusal must leave the log exactly as it found it: a migration
//! that half-ran and then refused is worse than one that never started,
//! because the operator's next decision is made against a population no plan
//! describes.
//!
//! THE AUTHORITY IS THE CALLER'S OWN DECISION. `MigrationAuthority` carries a
//! `decision_id` and the surfaces the decision allows, and the runner stamps
//! them onto every upcast it writes. Minting one from anything other than the
//! PDP's answer for THIS caller on THIS surface would put a fabricated
//! authorization into the durable record — an audit trail that says a
//! decision was made when none was.

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
    // The EXACT body. A field asserted at the one value it takes in the one
    // test that reads it is not pinned — a constant equal to that value
    // survives, which is the defect this suite already had to correct once
    // for `decision_id`. Every number here is a claim.
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

/// The upcast carries the decision the PDP MINTED FOR THIS RUN.
///
/// Two earlier shapes of this test both failed, and the way they failed is
/// the point. "Differs from the write's decision" is satisfied by a hardcoded
/// constant. "Two runs carry two decisions" is satisfied by any locally
/// minted counter — a `format!("dcn_forged_{n}")` in the handler survives it
/// while writing a fabricated authorization into every durable upcast. Both
/// assert VARIABILITY; neither asserts PROVENANCE, and only provenance is the
/// claim.
///
/// The process authorizes through a `SeededIdGenerator`, whose whole purpose
/// is determinism: decision N is `01hmz` followed by N. So the id is
/// predictable BY VALUE, and the claim becomes exact — the write is the
/// first decision this process made, the run is the second, and the upcast
/// must carry the second. A constant fails it, a private counter fails it,
/// and reusing the write's decision fails it by carrying the first.
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

/// The decision id this process's PDP mints on its `n`th authorization.
///
/// `SeededIdGenerator` renders `01HMZ` followed by the zero-padded counter;
/// the surface lowercases it. Binding to the generator rather than to a
/// literal keeps the assertion about provenance rather than about a string.
fn nth_decision(n: u64) -> String {
    format!("01hmz{n:021}")
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
