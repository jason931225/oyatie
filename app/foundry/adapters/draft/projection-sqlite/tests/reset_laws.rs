//! Adapter laws for the discard: what only a store with tables can be
//! held to.
//!
//! Separate from the other adapter laws because both of these need raw
//! SQL — one writes a corrupt head the port cannot express, the other
//! reads the schema itself.

// ADR-0083 Tier 3: integration tests are separate crates and carry their
// own header; the workspace lints are TEMPORARY allows pending restore.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{ObjectEntity, ObjectProperty, PropertyValue};
use foundry_projection_draft::{
    AppliedEntry, EntryOutcome, KeyDesignations, ProjectedLink, ProjectedObject, ProjectionStore,
    ProjectionStoreError,
};
use foundry_projection_sqlite_draft::SqliteProjectionStore;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

/// Owns the database file so it is swept on UNWIND as well as on the
/// happy path. A cleanup that is the last statement of a test is the
/// cleanup a failing test skips — and a failing test is exactly when an
/// operator is most likely to be looking at the directory.
struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(case: &str) -> Self {
        Self {
            path: std::env::temp_dir().join(format!(
                "foundry-projection-reset-{case}-{}-{}.sqlite",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            )),
        }
    }

    fn open(&self) -> SqliteProjectionStore {
        SqliteProjectionStore::open(&self.path).expect("open")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        sweep(&self.path);
    }
}

fn sweep(path: &Path) {
    let _ = std::fs::remove_file(path);
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_owned();
        sidecar.push(suffix);
        let _ = std::fs::remove_file(PathBuf::from(sidecar));
    }
}

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn typed(name: &str, value: PropertyValue) -> ObjectProperty {
    ObjectProperty::typed(name.to_owned(), value, internal())
}

fn projected(object_ref: &str) -> ProjectedObject {
    ProjectedObject {
        entity: ObjectEntity::new(
            "ten_a".to_owned(),
            object_ref.to_owned(),
            "ety_reading".to_owned(),
            vec![typed("name", PropertyValue::String("Ada".to_owned()))],
        )
        .unwrap(),
        schema_revision: 1,
        last_ordinal: 1,
        last_actor: "prn_projector".to_owned(),
    }
}

fn entry(ordinal: u64, objects: Vec<ProjectedObject>) -> AppliedEntry {
    AppliedEntry {
        tenant_id: "ten_a".to_owned(),
        ordinal,
        outcome: EntryOutcome::Applied {
            objects,
            links: Vec::new(),
        },
    }
}

/// A head that will not convert is a CORRUPT store, not an empty one,
/// so the discard refuses.
#[test]
fn a_corrupt_head_refuses_the_discard_and_keeps_the_rows() {
    let fixture = Fixture::new("corrupt-head");
    let stored = projected("ent_a1");
    {
        let mut store = fixture.open();
        store
            .apply(entry(1, vec![stored.clone()]), &KeyDesignations::default())
            .unwrap();
    }
    rusqlite::Connection::open(&fixture.path)
        .unwrap()
        .execute(
            "UPDATE projection_heads SET applied_ordinal = -5 WHERE tenant_id = 'ten_a'",
            [],
        )
        .unwrap();

    let mut store = fixture.open();
    let refused = store.reset_tenant("ten_a");

    assert!(
        matches!(refused, Err(ProjectionStoreError::Storage { .. })),
        "a corrupt head refuses: {refused:?}"
    );
    let survivors = rusqlite::Connection::open(&fixture.path)
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM projection_objects WHERE tenant_id = 'ten_a'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap();
    assert_eq!(survivors, 1, "nothing was destroyed by the refusal");
}

/// Every tenant-scoped table the schema declares is emptied — asserted
/// against `sqlite_master`, not against a list written by hand.
///
/// The delete list duplicates the DDL, so a seventh table added to
/// `open()` would silently under-delete unless some law happened to read
/// it back. Enumerating the schema covers such a table the day it is
/// added rather than the day someone remembers this test.
#[test]
fn a_reset_empties_every_tenant_scoped_table_the_schema_declares() {
    let fixture = Fixture::new("schema-drift");
    let mut store = fixture.open();
    store
        .apply(
            entry(1, vec![projected("ent_a1")]),
            &KeyDesignations::default(),
        )
        .unwrap();
    // An edge, or the `projection_links` limb of the assertion below is
    // already true before the reset and proves nothing.
    store
        .apply(
            AppliedEntry {
                tenant_id: "ten_a".to_owned(),
                ordinal: 2,
                outcome: EntryOutcome::Applied {
                    objects: vec![projected("ent_a2")],
                    links: vec![ProjectedLink {
                        link_type: "lty_measures".to_owned(),
                        from_object_ref: "ent_a1".to_owned(),
                        to_object_ref: "ent_a2".to_owned(),
                        observed_at_epoch_ms: 1_700_000_000_000,
                    }],
                },
            },
            &KeyDesignations::default(),
        )
        .unwrap();
    store
        .apply(
            AppliedEntry {
                tenant_id: "ten_a".to_owned(),
                ordinal: 3,
                outcome: EntryOutcome::Poisoned {
                    reason: "payload_decode".to_owned(),
                },
            },
            &KeyDesignations::default(),
        )
        .unwrap();
    store.reset_tenant("ten_a").unwrap();

    let connection = rusqlite::Connection::open(&fixture.path).unwrap();
    let tables: Vec<String> = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    let mut inspected = 0usize;
    for table in tables {
        let tenant_scoped = connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .any(|column| column.as_deref() == Ok("tenant_id"));
        if !tenant_scoped {
            continue;
        }
        inspected += 1;
        let remaining: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?1"),
                ["ten_a"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            remaining, 0,
            "{table} still holds rows for the reset tenant"
        );
    }
    // Non-vacuity: an enumeration that silently matched nothing would
    // pass every assertion above without examining anything.
    assert_eq!(inspected, 6, "every tenant-scoped table was inspected");
}
