//! `store == fold(log)` proven against the DURABLE store, not a second
//! in-memory one.
//!
//! The read path serves from the durable store, so anything the adapter
//! drops, reorders, or rounds on its way to disk is served as the
//! answer. This file folds a log that contains objects, an edge, and a
//! poison, rebuilds a real SQLite database from it, and compares that
//! database to the fold — then drops the connection and compares again,
//! because a projection that is only correct while its process is alive
//! is not durable.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use foundry_projection_draft::ProjectionStore;
use foundry_projection_sqlite_draft::SqliteProjectionStore;
use foundry_spine::{CatchUpError, catch_up, fold_from_scratch};

#[allow(dead_code)]
mod catchup_support;
use catchup_support::{
    TENANT, assert_agrees_with_fold, corrupt, log, mixed_log, registry, sealed,
    sealed_by_another_actor,
};

/// A database on disk that cleans itself up, and can be reopened
/// through the front door.
struct Database {
    path: PathBuf,
}

impl Database {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "foundry-catchup-{label}-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);
        Self { path }
    }

    fn open(&self) -> SqliteProjectionStore {
        SqliteProjectionStore::open(&self.path).expect("open the database")
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[test]
fn a_rebuilt_sqlite_projection_equals_the_fold_of_the_log() {
    let registry = registry();
    let log = mixed_log();
    let database = Database::new("equals-fold");
    let mut store = database.open();

    let caught = catch_up(TENANT, &registry, &mut store, &log).expect("rebuild onto disk");

    assert_eq!(caught.resumed_from, 0, "the database started empty");
    assert_eq!(caught.head, 5);
    let folded = fold_from_scratch(TENANT, &registry, log.iter());
    assert_agrees_with_fold(&store, &folded);

    // Non-vacuity: the comparison above is only worth something if the
    // fold actually holds objects, an edge, and a poison.
    assert_eq!(folded.bindings.len(), 3, "three objects survived the fold");
    assert_eq!(folded.poison.len(), 1, "and one entry poisoned");
    let outbound = store.links_from(TENANT, "ent_1").unwrap();
    assert_eq!(outbound.len(), 1, "the edge is on disk: {outbound:?}");
    assert_eq!(outbound[0].to_object_ref, "ent_2");
    assert_eq!(store.links_to(TENANT, "ent_2").unwrap(), outbound);
}

#[test]
fn the_rebuilt_projection_survives_dropping_the_connection() {
    let registry = registry();
    let log = mixed_log();
    let database = Database::new("survives-reopen");
    {
        let mut store = database.open();
        catch_up(TENANT, &registry, &mut store, &log).expect("rebuild onto disk");
    }

    // Back through the front door: nothing of the rebuild lived in the
    // process that performed it. Edges are asserted explicitly — the
    // shared oracle does not compare them, so durability measured only
    // through it would pass against an adapter that lost every edge.
    let reopened = database.open();
    assert_agrees_with_fold(&reopened, &fold_from_scratch(TENANT, &registry, log.iter()));
    let outbound = reopened.links_from(TENANT, "ent_1").unwrap();
    assert_eq!(outbound.len(), 1, "the edge survived: {outbound:?}");
    assert_eq!(outbound[0].to_object_ref, "ent_2");
    assert_eq!(reopened.links_to(TENANT, "ent_2").unwrap(), outbound);
}

#[test]
fn catching_up_a_durable_store_a_second_time_revalidates_and_advances_nothing() {
    let registry = registry();
    let log = log();
    let database = Database::new("idempotent");
    {
        let mut store = database.open();
        catch_up(TENANT, &registry, &mut store, &log).expect("rebuild onto disk");
    }

    // A restarted process re-runs catch-up against a store that is
    // already current. The head it reports must mean what it says
    // across that restart, or readiness is a guess.
    let mut reopened = database.open();
    let caught = catch_up(TENANT, &registry, &mut reopened, &log).expect("already current");

    assert_eq!(caught.resumed_from, 3);
    assert_eq!(caught.head, 3);
    assert!(
        caught.revalidated,
        "the durable resume point was re-applied and agreed"
    );
    assert_agrees_with_fold(&reopened, &fold_from_scratch(TENANT, &registry, log.iter()));
}

#[test]
fn a_log_that_does_not_begin_at_ordinal_one_is_refused() {
    let registry = registry();
    let full = mixed_log();

    // `RecordsLog::replay(tenant, from)` hands back a SLICE, so both of
    // these are what a caller naturally writes: the entries after the
    // head, and the entries from the head. Resume state is rebuilt by
    // folding everything BELOW that head, which neither can produce.
    // Unchecked, the first mirrored against a fresh fold and wrote a
    // poison derived from where the caller cut; the second refused a
    // healthy store while naming a different log.
    for (from, first_ordinal) in [(3usize, 4u64), (2usize, 3u64)] {
        let database = Database::new("partial-slice");
        let mut store = database.open();
        catch_up(TENANT, &registry, &mut store, &full[..3]).expect("reach head 3");

        let error = catch_up(TENANT, &registry, &mut store, &full[from..]).expect_err("refuse");

        assert!(
            matches!(error, CatchUpError::LogDoesNotStartAtOne { first_ordinal: f } if f == first_ordinal),
            "{error:?}"
        );
        assert_eq!(store.applied_head(TENANT).unwrap(), 3, "and wrote nothing");
        assert!(
            store.poisoned(TENANT).unwrap().is_empty(),
            "no poison was fabricated"
        );
    }
}

#[test]
fn a_log_missing_the_resume_point_is_refused() {
    let registry = registry();
    let held = vec![
        sealed(1, "one"),
        sealed(2, "two"),
        sealed_by_another_actor(3, "three"),
    ];
    let database = Database::new("absent-resume-point");
    let mut store = database.open();
    catch_up(TENANT, &registry, &mut store, &held).expect("reach head 3");

    // Begins at ordinal 1 but has no entry at 3, so the entry the store
    // stopped at cannot be re-applied and the resume is unvalidated. The
    // store's own ordinal 3 came from a different writer, and skipping
    // the check would retain that row while reporting a clean catch-up.
    let gapped = vec![sealed(1, "one"), sealed(2, "two"), sealed(4, "four")];
    let error = catch_up(TENANT, &registry, &mut store, &gapped).expect_err("must refuse");

    assert!(
        matches!(
            error,
            CatchUpError::ResumePointMissingFromLog { ordinal: 3 }
        ),
        "{error:?}"
    );
    assert_eq!(
        store.applied_head(TENANT).unwrap(),
        3,
        "and advanced nothing"
    );
    assert_eq!(
        store.get(TENANT, "ent_3").unwrap().unwrap().last_actor,
        "prn_bob",
        "the foreign row is still there, and still refused"
    );
}

#[test]
fn a_store_whose_prefix_poisoned_differently_is_refused() {
    let registry = registry();
    // The store's log poisoned at ordinal 2. The offered log does not.
    let held = vec![sealed(1, "one"), corrupt(2), sealed(3, "three")];
    let offered = vec![
        sealed(1, "one"),
        sealed(2, "two"),
        sealed(3, "three"),
        sealed(4, "four"),
    ];
    let database = Database::new("prefix-poisons");
    let mut store = database.open();
    catch_up(TENANT, &registry, &mut store, &held).expect("build from the first log");

    // Revalidating the head entry cannot see this: one envelope is one
    // object, so ordinal 3's mirrored entry carries only ent_3 and is
    // byte-identical under both logs. It dedups, and without a prefix
    // check the store would finish at head 4 holding a poison that
    // `fold(offered)` does not have — `applied_head == log head` over a
    // projection that is a mixture of two logs.
    let error = catch_up(TENANT, &registry, &mut store, &offered).expect_err("must refuse");

    assert!(
        matches!(error, CatchUpError::DivergentPrefixPoisons { .. }),
        "{error:?}"
    );
    assert_eq!(
        store.applied_head(TENANT).unwrap(),
        3,
        "and advanced nothing"
    );
    assert_eq!(
        store.poisoned(TENANT).unwrap().len(),
        1,
        "still its own log's poison"
    );
}

#[test]
fn a_durable_store_resumes_from_a_poisoned_head_without_false_divergence() {
    let registry = registry();
    let log = mixed_log();
    let database = Database::new("poisoned-head");
    {
        // Ordinal 4 poisons, so the interrupted rebuild leaves the head
        // ON a poison rather than on an object.
        let mut store = database.open();
        let caught = catch_up(TENANT, &registry, &mut store, &log[..4]).expect("partial rebuild");
        assert_eq!(caught.head, 4);
        assert_eq!(store.poisoned(TENANT).unwrap().len(), 1, "it did poison");
    }

    // Resuming re-applies that poisoned entry to revalidate the resume
    // point, which means the adapter has to recognise a re-applied
    // POISON as a duplicate. A store that reconstructed poisons lossily
    // would call this divergence and refuse a healthy resume — the
    // failure would look like corruption and be a false alarm.
    let mut reopened = database.open();
    let caught = catch_up(TENANT, &registry, &mut reopened, &log).expect("resume past the poison");

    assert_eq!(caught.resumed_from, 4);
    assert!(caught.revalidated);
    assert_eq!(caught.head, 5);
    assert_agrees_with_fold(&reopened, &fold_from_scratch(TENANT, &registry, log.iter()));
}

#[test]
fn a_durable_store_resumes_a_partial_rebuild_where_it_stopped() {
    let registry = registry();
    let log = mixed_log();
    let database = Database::new("partial");
    {
        // A rebuild interrupted after two entries — a killed process,
        // not a store failure.
        let mut store = database.open();
        catch_up(TENANT, &registry, &mut store, &log[..2]).expect("partial rebuild");
    }

    let mut reopened = database.open();
    let caught = catch_up(TENANT, &registry, &mut reopened, &log).expect("finish the rebuild");

    assert_eq!(
        caught.resumed_from, 2,
        "it resumed from what the disk held, not from zero"
    );
    assert_eq!(caught.head, 5);
    assert_agrees_with_fold(&reopened, &fold_from_scratch(TENANT, &registry, log.iter()));
}
