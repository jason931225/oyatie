//! `POST /v1/migrations/run` — what it reports when it cannot finish.
//!
//! The states these cover are the ones the reported fields exist for, and the
//! only ones that tell an honest report from a flattering constant: at a
//! clean fixpoint `pending`, `refused`, `conflicted`, `unavailable` and
//! `poisoned` are all zero and `fixpoint` is true, so a constant equal to
//! that survives every test in the sibling suite. Each body here is asserted
//! VERBATIM, because a field asserted at the one value it takes in the one
//! test that reads it is not pinned.

mod facade_support;
mod failing_log;
mod migration_support;
mod out_of_band;

use axum::http::StatusCode;
use facade_support::{Fixture, Session};
use failing_log::AlwaysFailingLog;
use migration_support::{plan_for, run, state_with_two_revisions, write_owing};

/// A run that CANNOT finish says so, in every field that carries the news.
///
/// This is the state the reported fields exist for, and the only one that
/// tells an honest report from a flattering constant: at a clean fixpoint
/// `pending`, `refused`, `conflicted` and `poisoned` are all zero and
/// `fixpoint` is true, so hardcoding any of them survives every other test in
/// this file. The store is broken AFTER the object lands, so an object is
/// genuinely owed and genuinely cannot be written.
///
/// It is a 200, not a refusal: the plan was executable and the run did what it
/// could. The failure is reported in the body, because a migration that
/// stopped short is a fact about the population, not a bad request.
#[tokio::test]
async fn a_run_that_cannot_write_reports_the_work_it_could_not_do() {
    let fixture = Fixture::new("run-store-outage");
    let config = fixture.config();
    let state = std::sync::Arc::new(state_with_two_revisions(&config));
    let session = Session::from_shared(std::sync::Arc::clone(&state));
    let token = Some(fixture.operator_token());
    write_owing(&session, token, "ent_alpha", "idem_1").await;
    for tenant in state.tenants.values() {
        tenant.lock().await.action_log = Box::new(AlwaysFailingLog {
            detail: "the store went away mid-migration",
        });
    }

    let (status, body) = run(&session, token, &plan_for("ten_acme")).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    // Exact, so `conflicted` is pinned at ZERO here and at one nowhere else:
    // a store that could not accept the append is not a caller who reused an
    // idempotency key, and reporting it as one is blame in the wrong place
    // and advice against the retry that would work.
    assert_eq!(
        body,
        r#"{"total":1,"upcast":0,"pending":1,"refused":0,"conflicted":0,"unavailable":1,"poisoned":0,"fixpoint":false}"#
    );
}
/// THE RUN TERMINATES. Not "converges quickly" — terminates at all.
///
/// A poison stands in the log and advances the fold, but it never binds the
/// object, so the plan still owes it, the same drift-sensitive key is
/// re-derived, and the byte-identical append deduplicates onto the same
/// poisoned ordinal. Counting that as progress made the loop a fixed point of
/// its own body: every pass identical, no append, nothing converging. The
/// handler never returned, and it holds the tenant's lock for the whole run —
/// so one migration against a projection one entry behind its log wedged
/// every request for that tenant until the process was restarted.
///
/// The state is not exotic. `out_of_band` exists because this repository's own
/// writer commits before it folds, and says a panic between the two "leaves
/// this process permanently one behind for its lifetime".
///
/// Driven on its own thread with a deadline, because the failure this pins is
/// non-termination: an `#[tokio::test]` would hang the runtime rather than
/// fail, and a hang in CI reads as an infrastructure problem rather than as
/// this defect.
#[test]
fn a_run_against_a_lagging_projection_terminates_rather_than_spinning() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        runtime.block_on(async {
            let fixture = Fixture::new("run-lagging-projection");
            let config = fixture.config();
            let session = Session::from_state(state_with_two_revisions(&config));
            let token = Some(fixture.operator_token());
            write_owing(&session, token, "ent_alpha", "idem_1").await;
            // One entry the durable log holds and this projection never
            // applied, exactly as a crash between append and fold leaves it.
            out_of_band::append_for(&config.action_log, "ten_acme", "idem_out_of_band");
            let _ = tx.send(run(&session, token, &plan_for("ten_acme")).await);
        });
    });

    let (status, body) = rx
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect("the run must return; a migration that never ends holds the tenant lock forever");

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        r#"{"total":1,"upcast":0,"pending":1,"refused":0,"conflicted":0,"unavailable":0,"poisoned":1,"fixpoint":false}"#
    );
}
/// A SECOND run over a poisoned object reports a CONFLICT, and the conflict
/// is this process's own retry rather than anything the caller did.
///
/// Worth pinning for two reasons. It is the only state in which `conflicted`
/// is non-zero, so without it a hardcoded zero survives every other test
/// here. And it is a diagnosis an operator will misread: `conflicted` means
/// "a spent idempotency key, different content", and the caller reused
/// nothing — the upcast's payload carries the decision id, the PDP mints a
/// fresh decision per request, so the retry differs from the first attempt in
/// a byte the idempotency key does not cover. There is no action the operator
/// can take, and the object stays owed until the projection is refolded.
#[tokio::test]
async fn a_retried_upcast_conflicts_with_its_own_earlier_attempt() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        runtime.block_on(async {
            let fixture = Fixture::new("run-retry-conflict");
            let config = fixture.config();
            let session = Session::from_state(state_with_two_revisions(&config));
            let token = Some(fixture.operator_token());
            write_owing(&session, token, "ent_alpha", "idem_1").await;
            out_of_band::append_for(&config.action_log, "ten_acme", "idem_out_of_band");
            let _ = run(&session, token, &plan_for("ten_acme")).await;
            let _ = tx.send(run(&session, token, &plan_for("ten_acme")).await);
        });
    });

    let (status, body) = rx
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect("the retry must return");

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        r#"{"total":1,"upcast":0,"pending":1,"refused":0,"conflicted":1,"unavailable":0,"poisoned":0,"fixpoint":false}"#
    );
}
/// A poisoned ENTRY is counted once, however many passes re-attempt it.
///
/// Two objects owed and the projection one entry behind: the first attempt
/// poisons and — in poisoning — advances `applied_ordinal`, which repairs the
/// lag, so the second object upcasts normally. That progress buys another
/// pass, which re-submits the poisoned object under the same drift-sensitive
/// key and deduplicates onto the ordinal already poisoned. Counting the
/// re-observation reported two poisoned entries where one exists, telling an
/// operator a population of two had failed entirely when half of it landed.
#[tokio::test]
async fn a_poisoned_entry_is_counted_once_not_once_per_pass() {
    let fixture = Fixture::new("run-poison-counted-once");
    let config = fixture.config();
    let session = Session::from_state(state_with_two_revisions(&config));
    let token = Some(fixture.operator_token());
    write_owing(&session, token, "ent_alpha", "idem_1").await;
    write_owing(&session, token, "ent_beta", "idem_2").await;
    out_of_band::append_for(&config.action_log, "ten_acme", "idem_out_of_band");

    let (status, body) = run(&session, token, &plan_for("ten_acme")).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        r#"{"total":2,"upcast":1,"pending":1,"refused":0,"conflicted":0,"unavailable":0,"poisoned":1,"fixpoint":false}"#
    );
}
