//! The catch-up law: a durable projection behind its log can be brought
//! to the log's head, and `applied_head` means what it says.
//!
//! These pin the part most easily got wrong: catch-up resumes INCLUSIVE
//! of the store's own head entry, so a store built from a DIFFERENT log
//! is refused rather than topped up to a head that lies. `catchup.rs`
//! states what that does and does not prove.

use foundry_projection_draft::{MemoryProjectionStore, ProjectionStore};
use foundry_spine::{CatchUpError, ProjectionState, catch_up, fold_from_scratch, project_through};

#[allow(dead_code)]
mod catchup_support;
#[allow(dead_code)]
mod write_through_support;

use catchup_support::{
    TENANT, assert_agrees_with_fold, corrupt, log, mixed_log, registry, sealed,
    sealed_at_unknown_revision,
};
use write_through_support::FailsAt;

#[test]
fn a_cold_start_catches_up_the_whole_log_and_is_never_divergence() {
    let registry = registry();
    // A long log against a store never written: a fresh deployment, and
    // the one case that MUST NOT refuse. A caller refusing boot on
    // divergence keys off this distinction, and it holds by
    // construction rather than by care — ordinals are dense from 1 and
    // `resumed_from` is 0, so the divergence arm is unreachable here.
    let log: Vec<_> = (1..=40)
        .map(|ordinal| sealed(ordinal, &format!("reading {ordinal}")))
        .collect();
    let mut store = MemoryProjectionStore::default();

    let caught = catch_up(TENANT, &registry, &mut store, &log).expect("a cold start must succeed");

    assert_eq!(caught.resumed_from, 0);
    assert_eq!(caught.head, 40, "it caught up the whole log");
    assert!(
        !caught.revalidated,
        "an empty store has no resume point to revalidate"
    );
    assert_agrees_with_fold(&store, &fold_from_scratch(TENANT, &registry, log.iter()));
}

#[test]
fn catch_up_resumes_where_the_store_stopped_rather_than_restarting() {
    let registry = registry();
    let log = log();
    let mut store = MemoryProjectionStore::default();
    let mut state = ProjectionState::new(TENANT, &registry);
    project_through(&mut state, &mut store, &log[..2]).expect("mirror the first two");

    let caught = catch_up(TENANT, &registry, &mut store, &log).expect("resume");

    assert_eq!(
        caught.resumed_from, 2,
        "resume begins at what the store durably held"
    );
    assert_eq!(caught.head, 3);
    assert!(
        caught.revalidated,
        "the resume point was re-applied and agreed"
    );
    assert_agrees_with_fold(&store, &fold_from_scratch(TENANT, &registry, log.iter()));
}

#[test]
fn a_rebuilt_store_equals_an_incrementally_written_one() {
    let registry = registry();
    // The MIXED log, not the plain one: over three bare objects the claim
    // is vacuous on both edges and poison — the link assertion below would
    // compare `[] == []`, and no poison would exist to disagree about.
    let log = mixed_log();

    let mut incremental = MemoryProjectionStore::default();
    let mut state = ProjectionState::new(TENANT, &registry);
    for sealed in &log {
        project_through(&mut state, &mut incremental, std::slice::from_ref(sealed))
            .expect("mirror one at a time");
    }

    let mut rebuilt = MemoryProjectionStore::default();
    catch_up(TENANT, &registry, &mut rebuilt, &log).expect("rebuild in one call");

    // Entry-at-a-time and all-at-once are the same function of the log. Held
    // to the FOLD rather than to each other for the object set and the poison
    // ledger: comparing the two planes only to one another cannot see a row
    // both retain and the fold never produced, and walking the fold's own
    // keys cannot see a row the store holds and the fold does not.
    assert_agrees_with_fold(&incremental, &state);
    assert_agrees_with_fold(&rebuilt, &state);

    // The edge must actually be there, or the comparison below is `[] == []`
    // and passes against a plane that emits no edges at all. Asserting the
    // fixture's own content is what makes the equality mean something.
    assert!(
        !incremental.links_from(TENANT, "ent_1").unwrap().is_empty(),
        "the fixture must exercise an edge for the equality to be a claim"
    );
    for object_ref in state.bindings.keys() {
        assert_eq!(
            incremental.get(TENANT, object_ref).unwrap(),
            rebuilt.get(TENANT, object_ref).unwrap(),
            "{object_ref} differs between the two paths"
        );
        assert_eq!(
            incremental.links_from(TENANT, object_ref).unwrap(),
            rebuilt.links_from(TENANT, object_ref).unwrap(),
            "{object_ref} edges differ between the two paths"
        );
    }
}

#[test]
fn catch_up_reproduces_the_same_poison_on_every_rebuild() {
    let registry = registry();
    // Ordinal 2 is undecodable — a poison derived from (log bytes,
    // registry), so it lands identically however often we rebuild.
    let log = vec![sealed(1, "one"), corrupt(2), sealed(3, "three")];

    let mut first = MemoryProjectionStore::default();
    let mut second = MemoryProjectionStore::default();
    catch_up(TENANT, &registry, &mut first, &log).expect("first rebuild");
    catch_up(TENANT, &registry, &mut second, &log).expect("second rebuild");

    let poisoned = first.poisoned(TENANT).unwrap();
    assert_eq!(poisoned.len(), 1, "exactly the corrupt entry poisoned");
    assert_eq!(poisoned[0].0, 2, "and it was ordinal 2");
    assert_eq!(
        poisoned[0].1, "payload_decode",
        "and it was poisoned for the reason the bytes give, not merely poisoned"
    );
    assert_eq!(
        poisoned,
        second.poisoned(TENANT).unwrap(),
        "a poison is a function of the log, not of when it was folded"
    );
    assert_eq!(first.applied_head(TENANT).unwrap(), 3);
}

#[test]
fn a_store_outage_halts_catch_up_and_a_later_call_resumes_from_there() {
    let registry = registry();
    let log = log();
    let mut store = FailsAt {
        inner: MemoryProjectionStore::default(),
        fail_on_ordinal: 2,
        fail_head: false,
        fail_poisoned: false,
    };

    let error = catch_up(TENANT, &registry, &mut store, &log).expect_err("the store is out");
    assert!(
        matches!(error, CatchUpError::Mirror(_)),
        "an outage is infrastructure, never a poison: {error:?}"
    );
    assert_eq!(
        store.applied_head(TENANT).unwrap(),
        1,
        "entries before the outage stayed durable"
    );

    // The outage clears. Nothing was wedged.
    store.fail_on_ordinal = 0;
    let caught = catch_up(TENANT, &registry, &mut store, &log).expect("resume after the outage");

    assert_eq!(caught.resumed_from, 1);
    assert_eq!(caught.head, 3);
    assert_agrees_with_fold(&store, &fold_from_scratch(TENANT, &registry, log.iter()));
}

#[test]
fn catch_up_re_mirrors_nothing_the_store_already_holds() {
    let registry = registry();
    let log: Vec<_> = (1..=40)
        .map(|ordinal| sealed(ordinal, &format!("reading {ordinal}")))
        .collect();
    let mut built = MemoryProjectionStore::default();
    catch_up(TENANT, &registry, &mut built, &log[..20]).expect("reach head 20");

    // Refusing ordinal 5, inside the prefix the store holds: re-mirroring
    // from 1 would trip it, so writes are bounded by the store's head.
    let mut store = FailsAt {
        inner: built,
        fail_on_ordinal: 5,
        fail_head: false,
        fail_poisoned: false,
    };

    let caught =
        catch_up(TENANT, &registry, &mut store, &log).expect("resume without re-mirroring");

    assert_eq!(caught.resumed_from, 20);
    assert_eq!(caught.head, 40);
}

#[test]
#[should_panic(expected = "no more")]
fn the_oracle_rejects_a_store_holding_more_than_the_fold() {
    let registry = registry();
    let log = log();
    let mut store = MemoryProjectionStore::default();
    catch_up(TENANT, &registry, &mut store, &log).expect("three objects");

    // The oracle is this suite's definition of correct, so it needs its
    // own proof that it can fail. Iterating only the fold's bindings
    // shows the store holds everything it should and never that it holds
    // nothing MORE, so a row retained from another log is invisible.
    assert_agrees_with_fold(
        &store,
        &fold_from_scratch(TENANT, &registry, log[..2].iter()),
    );
}

#[test]
#[should_panic(expected = "by ordinal AND reason")]
fn the_oracle_rejects_a_ledger_that_differs_only_in_reason() {
    let registry = registry();
    let mut store = MemoryProjectionStore::default();
    catch_up(
        TENANT,
        &registry,
        &mut store,
        &[sealed(1, "one"), corrupt(2), sealed(3, "three")],
    )
    .expect("poisoned at 2");

    // Same head, same objects, same poison COUNT — one different reason
    // at the same ordinal. A count comparison cannot see it.
    let other = fold_from_scratch(
        TENANT,
        &registry,
        [
            sealed(1, "one"),
            sealed_at_unknown_revision(2),
            sealed(3, "three"),
        ]
        .iter(),
    );
    assert_agrees_with_fold(&store, &other);
}
