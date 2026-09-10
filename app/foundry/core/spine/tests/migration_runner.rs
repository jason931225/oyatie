#[allow(dead_code)]
#[path = "migration_support/mod.rs"]
mod support;

use data_ontology_kernel::PropertyValue;
use foundry_edits::WireValue;
use foundry_records_draft::RecordsLog;
use foundry_spine::{apply_sealed, pending_objects, run_to_fixpoint, upcast_idempotency_key};
use support::{MemoryLog, authority, fixture, plan, sealed_upsert, wire_integer};

#[test]
fn pending_owes_only_objects_whose_computed_targets_differ() {
    let (_, _, state) = fixture();
    let pending = pending_objects(&state, &plan());
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].object_ref, "ent_a");
    assert_eq!(pending[0].last_ordinal, 1);
    let names: Vec<&str> = pending[0].targets.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["score_text", "grade"]);
    assert_eq!(pending[0].targets[0].value, WireValue::String("7".into()));
    assert_eq!(pending[0].targets[1].value, WireValue::String("F".into()));
}

#[test]
fn run_lands_ordinary_upserts_stamped_at_head() {
    let (_, mut log, mut state) = fixture();
    let mut denials = MemoryLog::default();
    let status =
        run_to_fixpoint(&plan(), &authority(), &mut log, &mut denials, &mut state).unwrap();
    assert_eq!(status.total, 2);
    assert_eq!(status.upcast, 1);
    assert_eq!(status.pending, 0);
    assert!(status.fixpoint);
    assert_eq!(log.head("ten_test").unwrap(), 3);
    let entity = state.objects.get("ten_test", "ent_a").unwrap();
    assert_eq!(
        entity.properties["score_text"].value.value,
        PropertyValue::String("7".into())
    );
    assert_eq!(
        entity.properties["grade"].value.value,
        PropertyValue::String("F".into())
    );
    assert_eq!(
        entity.properties["score"].value.value,
        PropertyValue::Integer(7),
        "the source value is never destroyed"
    );
    assert_eq!(state.bindings["ent_a"].schema_revision, 2);
    assert_eq!(state.bindings["ent_a"].last_actor, "prn_migrator");
    // The no-op object was never touched: still at its seeded write.
    assert_eq!(state.bindings["ent_b"].last_ordinal, 2);
}

#[test]
fn rerun_after_fixpoint_appends_nothing() {
    let (_, mut log, mut state) = fixture();
    let mut denials = MemoryLog::default();
    run_to_fixpoint(&plan(), &authority(), &mut log, &mut denials, &mut state).unwrap();
    let head = log.head("ten_test").unwrap();
    let again = run_to_fixpoint(&plan(), &authority(), &mut log, &mut denials, &mut state).unwrap();
    assert_eq!(again.upcast, 0);
    assert!(again.fixpoint);
    assert_eq!(log.head("ten_test").unwrap(), head);
}

#[test]
fn drifted_object_mints_a_fresh_key_and_converges() {
    let (_, mut log, mut state) = fixture();
    let mut denials = MemoryLog::default();
    // The object drifts: an ordinary write moves its ordinal and its score.
    let drift = sealed_upsert("ent_a", 3, 2, vec![wire_integer("score", 8)]);
    apply_sealed(&mut state, &drift);
    assert!(state.poison.is_empty(), "drift write applies clean");
    log.seed(drift);
    assert_ne!(
        upcast_idempotency_key(&plan(), "ent_a", 3),
        upcast_idempotency_key(&plan(), "ent_a", 1),
        "the key law is drift-sensitive"
    );
    let status =
        run_to_fixpoint(&plan(), &authority(), &mut log, &mut denials, &mut state).unwrap();
    assert_eq!(status.upcast, 1);
    assert!(status.fixpoint);
    let entity = state.objects.get("ten_test", "ent_a").unwrap();
    assert_eq!(
        entity.properties["score_text"].value.value,
        PropertyValue::String("8".into()),
        "the fresh key carries the recomputed value"
    );
}
