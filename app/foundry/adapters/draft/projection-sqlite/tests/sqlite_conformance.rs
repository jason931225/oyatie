//! The SQLite projection store, held to the port's conformance suite —
//! including the durability clause the in-memory reference declines.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// The wall-clock recipe alone duplicated across parallel tests under
/// load — two tests then shared one database and produced
/// divergent-replay and malformed-image failures that read as real
/// defects.
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

use foundry_projection_draft::ProjectionStore;
use foundry_projection_draft::conformance::{
    ProjectionFixture, check_a_composite_key_value_is_refused,
    check_a_duplicate_primary_key_is_refused, check_a_later_observation_updates_the_edge,
    check_a_missing_key_property_is_refused, check_a_refused_apply_leaves_state_untouched,
    check_a_refused_apply_writes_no_links, check_an_object_may_keep_its_own_key,
    check_an_undeclared_key_constrains_nothing, check_applies_restart_at_ordinal_one_after_reset,
    check_apply_requires_the_next_dense_ordinal, check_cross_kind_comparisons_fail_closed,
    check_divergent_reapply_is_refused, check_durability_across_reopen,
    check_equals_predicate_matches_exactly, check_get_returns_the_projected_object,
    check_identical_reapply_is_a_deduplicated_noop,
    check_keys_are_scoped_to_their_entity_type_and_tenant, check_links_are_tenant_isolated,
    check_links_round_trip_in_both_directions,
    check_poisoned_entries_advance_the_head_without_objects, check_range_kind_mismatch_is_refused,
    check_range_predicate_is_kind_scoped, check_re_applying_an_entry_does_not_duplicate_links,
    check_reads_are_tenant_isolated, check_reset_discards_everything_for_the_tenant,
    check_reset_leaves_other_tenants_untouched, check_reset_refuses_a_blank_tenant,
    check_reset_survives_reopen, check_resetting_an_unknown_tenant_discards_nothing,
    check_two_objects_in_one_entry_cannot_share_a_key,
    check_type_scan_pages_partition_deterministically,
};
use foundry_projection_sqlite_draft::SqliteProjectionStore;

struct SqliteFixture {
    path: PathBuf,
    store: SqliteProjectionStore,
}

impl SqliteFixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "foundry-projection-{}-{}-{}.sqlite",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = SqliteProjectionStore::open(&path).expect("open a fresh database");
        Self { path, store }
    }
}

impl Drop for SqliteFixture {
    fn drop(&mut self) {
        sweep(&self.path);
    }
}

fn sweep(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_owned();
        sidecar.push(suffix);
        let _ = std::fs::remove_file(std::path::PathBuf::from(sidecar));
    }
}

impl ProjectionFixture for SqliteFixture {
    type Store = SqliteProjectionStore;

    fn store(&mut self) -> &mut Self::Store {
        &mut self.store
    }

    fn reopen(&mut self) -> bool {
        // The durability clause: drop the connection entirely and come
        // back through the front door.
        self.store = SqliteProjectionStore::open(&self.path).expect("reopen the database");
        true
    }
}

fn run(check: fn(&mut SqliteFixture) -> Result<(), String>) {
    let mut fixture = SqliteFixture::new();
    if let Err(detail) = check(&mut fixture) {
        panic!("conformance refused: {detail}");
    }
}

#[test]
fn sqlite_store_passes_the_write_laws() {
    run(check_apply_requires_the_next_dense_ordinal);
    run(check_identical_reapply_is_a_deduplicated_noop);
    run(check_divergent_reapply_is_refused);
    run(check_a_refused_apply_leaves_state_untouched);
    run(check_poisoned_entries_advance_the_head_without_objects);
}

#[test]
fn sqlite_store_passes_the_read_laws() {
    run(check_get_returns_the_projected_object);
    run(check_reads_are_tenant_isolated);
    run(check_type_scan_pages_partition_deterministically);
}

#[test]
fn sqlite_store_passes_the_predicate_laws() {
    run(check_equals_predicate_matches_exactly);
    run(check_range_predicate_is_kind_scoped);
    run(check_range_kind_mismatch_is_refused);
    run(check_cross_kind_comparisons_fail_closed);
}

#[test]
fn sqlite_store_is_durable_across_reopen() {
    run(check_durability_across_reopen);
}

#[test]
fn the_sqlite_store_is_a_projection_store() {
    fn assert_impl<S: ProjectionStore>() {}
    assert_impl::<SqliteProjectionStore>();
}

#[test]
fn sqlite_store_passes_the_primary_key_laws() {
    run(check_a_duplicate_primary_key_is_refused);
    run(check_two_objects_in_one_entry_cannot_share_a_key);
    run(check_an_object_may_keep_its_own_key);
    run(check_keys_are_scoped_to_their_entity_type_and_tenant);
    run(check_an_undeclared_key_constrains_nothing);
    run(check_a_missing_key_property_is_refused);
    run(check_a_composite_key_value_is_refused);
}

#[test]
fn sqlite_store_passes_the_link_laws() {
    run(check_links_round_trip_in_both_directions);
    run(check_links_are_tenant_isolated);
    run(check_a_refused_apply_writes_no_links);
    run(check_re_applying_an_entry_does_not_duplicate_links);
    run(check_a_later_observation_updates_the_edge);
}

#[test]
fn reset_discards_everything_for_the_tenant() {
    run(check_reset_discards_everything_for_the_tenant);
}

#[test]
fn reset_leaves_other_tenants_untouched() {
    run(check_reset_leaves_other_tenants_untouched);
}

#[test]
fn resetting_an_unknown_tenant_discards_nothing() {
    run(check_resetting_an_unknown_tenant_discards_nothing);
}

#[test]
fn applies_restart_at_ordinal_one_after_reset() {
    run(check_applies_restart_at_ordinal_one_after_reset);
}

#[test]
fn reset_survives_reopen() {
    run(check_reset_survives_reopen);
}

#[test]
fn reset_refuses_a_blank_tenant() {
    run(check_reset_refuses_a_blank_tenant);
}
