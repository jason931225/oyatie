//! Catch-up refusals: every way the offered log can fail to be one this
//! store may resume against, and the store left untouched by each.
//!
//! Separate from the law tests because a refusal has two halves and the
//! second is easy to skip — the error is the visible one, and "changed
//! nothing" is the one that matters when the caller retries.

use foundry_projection_draft::{MemoryProjectionStore, ProjectionStore, ProjectionStoreError};
use foundry_spine::{CatchUpError, catch_up};

#[allow(dead_code)]
mod catchup_support;
#[allow(dead_code)]
mod write_through_support;

use catchup_support::{
    TENANT, corrupt, log, registry, sealed, sealed_at_unknown_revision, sealed_by_another_actor,
    sealed_for_another_tenant,
};
use write_through_support::FailsAt;

#[test]
fn a_store_ahead_of_its_log_is_refused() {
    let registry = registry();
    let log = log();
    let mut store = MemoryProjectionStore::default();
    catch_up(TENANT, &registry, &mut store, &log).expect("reach head 3");

    // Serving a truncated log would answer from rows it cannot justify.
    let error = catch_up(TENANT, &registry, &mut store, &log[..1]).expect_err("must refuse");

    assert_eq!(
        error,
        CatchUpError::StoreAheadOfLog {
            store_head: 3,
            log_head: 1
        }
    );
    assert_eq!(
        store.applied_head(TENANT).unwrap(),
        3,
        "and changed nothing"
    );
}

#[test]
fn a_store_holding_a_different_log_is_refused_rather_than_topped_up() {
    let registry = registry();
    let mine = log();
    let mut store = MemoryProjectionStore::default();
    catch_up(TENANT, &registry, &mut store, &mine).expect("reach head 3");

    // Same ordinals and keys, one different principal at the entry the
    // store stopped at, plus a fourth that would look like catching up.
    let theirs = vec![
        sealed(1, "one"),
        sealed(2, "two"),
        sealed_by_another_actor(3, "three"),
        sealed(4, "four"),
    ];

    let error = catch_up(TENANT, &registry, &mut store, &theirs).expect_err("must refuse");

    assert_eq!(error, CatchUpError::DivergentResumePoint { ordinal: 3 });
    assert_eq!(
        store.applied_head(TENANT).unwrap(),
        3,
        "the store neither advanced nor absorbed the other log"
    );
    assert_eq!(
        store.get(TENANT, "ent_3").unwrap().unwrap().last_actor,
        "prn_alice",
        "and it still holds what its own log wrote"
    );
}

#[test]
fn an_unreadable_store_is_refused_never_read_as_absent() {
    let registry = registry();
    let log = log();
    let unreadable = |head: bool, ledger: bool| {
        let mut built = MemoryProjectionStore::default();
        catch_up(TENANT, &registry, &mut built, &log[..2]).expect("reach head 2");
        FailsAt {
            inner: built,
            fail_on_ordinal: 0,
            fail_head: head,
            fail_poisoned: ledger,
        }
    };

    // An unreadable head read as 0 rebuilds the whole log over contents
    // we cannot see; an unreadable ledger read as empty lets the prefix
    // check pass over the same blindness. Neither is an absence.
    for mut store in [unreadable(true, false), unreadable(false, true)] {
        let before = store.inner.applied_head(TENANT).unwrap();
        let error = catch_up(TENANT, &registry, &mut store, &log).expect_err("must refuse");
        assert!(
            matches!(
                error,
                CatchUpError::Read(ProjectionStoreError::Storage { .. })
            ),
            "{error:?}"
        );
        // The error alone is not enough. Swallowing only the FIRST head
        // read still surfaces a refusal from the LAST one, so the test
        // passes while catch-up has already written over a store it
        // could not see. What separates them is that nothing changed.
        assert_eq!(
            before,
            store.inner.applied_head(TENANT).unwrap(),
            "wrote over a store it could not read"
        );
    }
}

#[test]
fn a_log_carrying_another_tenants_entry_is_refused() {
    let registry = registry();
    let mut store = MemoryProjectionStore::default();
    // Populated FIRST. The harm this refusal prevents is spending a live
    // tenant's ordinal, so asserting "nothing was spent" against a store that
    // was never written asserts absence against absence: it would hold just
    // as well if catch-up had done nothing at all, ever.
    catch_up(TENANT, &registry, &mut store, &log()).expect("reach head 3");
    let mut mixed = log();
    mixed.push(sealed_for_another_tenant(4));

    // The fold WOULD poison it, correctly, and that is the problem: the
    // poison spends this tenant's ordinal 4 and enters its ledger,
    // wedging it against its own log forever.
    let error = catch_up(TENANT, &registry, &mut store, &mixed).expect_err("must refuse");

    assert_eq!(error, CatchUpError::ForeignTenantEntry { ordinal: 4 });
    assert_eq!(
        store.applied_head(TENANT).unwrap(),
        3,
        "the store keeps the head it had; ordinal 4 was not spent"
    );
    assert!(
        store.poisoned(TENANT).unwrap().is_empty(),
        "and the foreign entry did not enter this tenant's ledger"
    );
}

#[test]
fn a_prefix_poisoning_for_a_different_reason_is_refused() {
    let registry = registry();
    // Same ordinal, same count, different reason — the axis an
    // ordinals-only comparison cannot see. Ordinal 3 is byte-identical
    // under both logs, so revalidation dedups and only the reason
    // separates them.
    let held = vec![sealed(1, "one"), corrupt(2), sealed(3, "three")];
    let offered = vec![
        sealed(1, "one"),
        sealed_at_unknown_revision(2),
        sealed(3, "three"),
        sealed(4, "four"),
    ];
    let mut store = MemoryProjectionStore::default();
    catch_up(TENANT, &registry, &mut store, &held).expect("build from the first log");

    let error = catch_up(TENANT, &registry, &mut store, &offered).expect_err("must refuse");

    match error {
        CatchUpError::DivergentPrefixPoisons {
            store_holds,
            log_produces,
        } => {
            assert_eq!(store_holds, vec![(2, "payload_decode".to_owned())]);
            assert_eq!(log_produces, vec![(2, "unknown_revision".to_owned())]);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        store.applied_head(TENANT).unwrap(),
        3,
        "and advanced nothing"
    );
}

#[test]
fn a_log_beginning_at_ordinal_zero_is_refused() {
    let registry = registry();
    let mut store = MemoryProjectionStore::default();

    // Ordinals are dense from ONE, so zero is not an early log — it is a
    // malformed one. The guard is `!= 1` rather than `> 1` for exactly
    // this: admitting zero folds it to a fabricated `non_dense_ordinal`
    // poison and refuses later under a wholly misleading name.
    let error = catch_up(
        TENANT,
        &registry,
        &mut store,
        &[sealed(0, "zero"), sealed(1, "one")],
    )
    .expect_err("must refuse");

    assert_eq!(
        error,
        CatchUpError::LogDoesNotStartAtOne { first_ordinal: 0 }
    );
    assert_eq!(store.applied_head(TENANT).unwrap(), 0, "and wrote nothing");
}

/// Log compaction is unimplemented. When it lands, the retention boundary
/// needs a SNAPSHOT of the fold taken AT that ordinal — not a guard on some
/// other arm. `LogDoesNotStartAtOne` is the arm a compacted log reaches, and
/// a guard placed anywhere else lets the mirror run against a fresh fold and
/// write poisons derived from where the log was cut rather than from the log.
#[test]
#[ignore = "log compaction is unimplemented; the doc above pins where its guard belongs"]
fn log_compaction_snapshots_at_the_retention_boundary() {
    todo!("no compaction surface exists to fold a snapshot against yet")
}
