//! Runner fault law: refusals land on the denial trail and never in the
//! log; divergent key reuse is a loud conflict, never a silent dedup; an
//! invalid plan touches nothing.

#[allow(dead_code)]
#[path = "migration_support/mod.rs"]
mod support;

use foundry_records_draft::RecordsLog;
use foundry_spine::{MigrationAuthority, PlanError, run_to_fixpoint, upcast_idempotency_key};
use support::{MemoryLog, authority, fixture, plan, sealed_create, wire_string};

#[test]
fn refusal_lands_on_the_denial_trail_not_the_log() {
    let (_, mut log, mut state) = fixture();
    let mut denials = MemoryLog::default();
    let unauthorized = MigrationAuthority {
        allowed_surfaces: vec!["wrong-console".into()],
        ..authority()
    };
    let status =
        run_to_fixpoint(&plan(), &unauthorized, &mut log, &mut denials, &mut state).unwrap();
    assert_eq!(status.upcast, 0);
    assert_eq!(status.refused, 1);
    assert_eq!(status.pending, 1);
    assert!(!status.fixpoint);
    assert_eq!(log.head("ten_test").unwrap(), 2, "nothing appended");
    assert_eq!(denials.head("ten_test").unwrap(), 1, "denial recorded");
}

#[test]
fn divergent_key_reuse_is_a_loud_conflict_not_a_silent_dedup() {
    let (_, mut log, mut state) = fixture();
    let mut denials = MemoryLog::default();
    let key = upcast_idempotency_key(&plan(), "ent_a", 1);
    // A forged prior entry under the exact key the runner will mint,
    // carrying different bytes.
    let mut forged = sealed_create("ent_a", 3, 2, vec![wire_string("grade", "Z")]);
    forged.envelope.idempotency_key = key;
    log.seed(forged);
    let status =
        run_to_fixpoint(&plan(), &authority(), &mut log, &mut denials, &mut state).unwrap();
    assert_eq!(status.conflicted, 1);
    assert_eq!(status.upcast, 0);
    assert!(!status.fixpoint);
}

#[test]
fn invalid_plan_never_touches_the_log() {
    let (_, mut log, mut state) = fixture();
    let mut denials = MemoryLog::default();
    let mut ahead = plan();
    ahead.to_revision = 3;
    ahead.from_revision = 2;
    let refused = run_to_fixpoint(&ahead, &authority(), &mut log, &mut denials, &mut state);
    assert_eq!(refused, Err(PlanError::RegistryHeadMismatch { head: 2 }));
    assert_eq!(log.head("ten_test").unwrap(), 2);
    assert_eq!(denials.head("ten_test").unwrap(), 0);
}

/// A poisoned entry is counted by EVERY run that still owes it.
///
/// The count is over distinct ordinals THIS run observed, not over receipts
/// it appended. Gating on `!receipt.deduplicated` fixes the double count
/// within one run and breaks the next one: a byte-identical retry from an
/// earlier run also deduplicates, so a second run reported every diagnostic
/// field zero while a poisoned entry still blocked the object — a bare count
/// with no reason in it, which is the failure this module refuses to emit.
#[test]
fn a_poison_is_counted_by_each_run_that_still_owes_it() {
    let (_engine, mut log, mut state) = fixture();
    // One entry the log holds and this projection never applied, so the next
    // append lands non-dense and poisons.
    log.seed(sealed_create(
        "ent_c",
        3,
        1,
        vec![wire_string("name", "Cyd")],
    ));
    let mut denials = MemoryLog::default();

    let first = run_to_fixpoint(&plan(), &authority(), &mut log, &mut denials, &mut state).unwrap();
    let second =
        run_to_fixpoint(&plan(), &authority(), &mut log, &mut denials, &mut state).unwrap();

    assert_eq!(first.poisoned, 1, "the run that appended it: {first:?}");
    assert_eq!(
        second.poisoned, 1,
        "and the run that still owes it — a retry deduplicates onto the same \
         poisoned ordinal, which is a re-observation, not an absence: {second:?}"
    );
}
