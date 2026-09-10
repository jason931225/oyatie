//! Sync status as the DURABLE store reports it.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{ObjectEntity, ObjectProperty, PropertyTier};
use foundry_projection_draft::{
    AppliedEntry, ApplyReceipt, EntryOutcome, KeyDesignations, MemoryProjectionStore, Page,
    PageRequest, ProjectedLink, ProjectedObject, ProjectionStore, ProjectionStoreError,
    PropertyPredicate,
};
use foundry_spine::store_sync_status;

fn object(object_ref: &str) -> ProjectedObject {
    ProjectedObject {
        entity: ObjectEntity::new(
            "ten_test".to_owned(),
            object_ref.to_owned(),
            "ety_reading".to_owned(),
            vec![ObjectProperty::new(
                "name".to_owned(),
                "Ada".to_owned(),
                PropertyTier::Scalar,
                PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap(),
            )],
        )
        .unwrap(),
        schema_revision: 1,
        last_ordinal: 1,
        last_actor: "prn_projector".to_owned(),
    }
}

fn applied(ordinal: u64, object_ref: &str) -> AppliedEntry {
    AppliedEntry {
        tenant_id: "ten_test".to_owned(),
        ordinal,
        outcome: EntryOutcome::Applied {
            objects: vec![object(object_ref)],
            links: Vec::new(),
        },
    }
}

fn poisoned(ordinal: u64, reason: &str) -> AppliedEntry {
    AppliedEntry {
        tenant_id: "ten_test".to_owned(),
        ordinal,
        outcome: EntryOutcome::Poisoned {
            reason: reason.to_owned(),
        },
    }
}

/// A store whose reads fail — an outage must surface, never read as
/// "caught up".
struct BrokenStore;

impl ProjectionStore for BrokenStore {
    fn apply(
        &mut self,
        _entry: AppliedEntry,
        _keys: &KeyDesignations,
    ) -> Result<ApplyReceipt, ProjectionStoreError> {
        unreachable!("the lag surface never writes")
    }

    fn applied_head(&self, _tenant_id: &str) -> Result<u64, ProjectionStoreError> {
        Err(ProjectionStoreError::Storage {
            detail: "disk gone".to_owned(),
        })
    }

    fn get(
        &self,
        _tenant_id: &str,
        _object_ref: &str,
    ) -> Result<Option<ProjectedObject>, ProjectionStoreError> {
        unreachable!()
    }

    fn objects_of_type(
        &self,
        _tenant_id: &str,
        _entity_type: &str,
        _page: &PageRequest,
    ) -> Result<Page, ProjectionStoreError> {
        unreachable!()
    }

    fn filter(
        &self,
        _tenant_id: &str,
        _entity_type: &str,
        _predicate: &PropertyPredicate,
        _page: &PageRequest,
    ) -> Result<Page, ProjectionStoreError> {
        unreachable!()
    }

    fn links_from(
        &self,
        _tenant_id: &str,
        _object_ref: &str,
    ) -> Result<Vec<ProjectedLink>, ProjectionStoreError> {
        unreachable!()
    }

    fn links_to(
        &self,
        _tenant_id: &str,
        _object_ref: &str,
    ) -> Result<Vec<ProjectedLink>, ProjectionStoreError> {
        unreachable!()
    }

    fn poisoned(&self, _tenant_id: &str) -> Result<Vec<(u64, String)>, ProjectionStoreError> {
        unreachable!()
    }
}

#[test]
fn lag_is_measured_against_what_the_store_durably_holds() {
    let mut store = MemoryProjectionStore::default();
    store
        .apply(applied(1, "ent_r1"), &KeyDesignations::default())
        .unwrap();
    store
        .apply(applied(2, "ent_r2"), &KeyDesignations::default())
        .unwrap();

    // The log is at 5; the store durably holds 2.
    let status = store_sync_status(&store, "ten_test", 5).unwrap();
    assert_eq!(status.applied_ordinal, 2);
    assert_eq!(status.head, 5);
    assert_eq!(status.lag, 3, "three entries are not yet durable");
}

#[test]
fn a_caught_up_store_reports_no_lag() {
    let mut store = MemoryProjectionStore::default();
    store
        .apply(applied(1, "ent_r1"), &KeyDesignations::default())
        .unwrap();
    let status = store_sync_status(&store, "ten_test", 1).unwrap();
    assert_eq!(status.lag, 0);
    assert_eq!(status.poisoned_count, 0);
    assert_eq!(status.first_poisoned_ordinal, None);
}

#[test]
fn poisons_come_from_the_store_and_name_where_an_operator_starts() {
    let mut store = MemoryProjectionStore::default();
    store
        .apply(applied(1, "ent_r1"), &KeyDesignations::default())
        .unwrap();
    store
        .apply(poisoned(2, "payload_decode"), &KeyDesignations::default())
        .unwrap();
    store
        .apply(poisoned(3, "receipt_mismatch"), &KeyDesignations::default())
        .unwrap();

    let status = store_sync_status(&store, "ten_test", 3).unwrap();
    assert_eq!(status.poisoned_count, 2);
    assert_eq!(
        status.first_poisoned_ordinal,
        Some(2),
        "the EARLIEST poison is where an operator starts reading",
    );
}

#[test]
fn a_store_outage_surfaces_instead_of_reading_as_caught_up() {
    let failure = store_sync_status(&BrokenStore, "ten_test", 5)
        .expect_err("an unreadable store must not answer");
    assert!(
        matches!(failure, ProjectionStoreError::Storage { .. }),
        "the outage is reported, never rendered as lag 0: {failure:?}",
    );
}
