//! The write-through law: mirroring the fold into the durable store
//! leaves the store equal to `fold(log)`, and a store failure HALTS —
//! it never becomes a poison.

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    ActionTypeDefinition, ActionTypeId, AutonomyTier, EntityTypeDefinition, EntityTypeId,
    EntityTypePropertyDefinition, LinkCardinality, LinkTypeDefinition, LinkTypeId, OntologyEngine,
    PropertyTier,
};
use foundry_edits::{
    ActionRecord, EditSet, OntologyEdit, WireDataClass, WireProperty, WireTier, WireValue,
    encode_action_record,
};
use foundry_projection_draft::{MemoryProjectionStore, ProjectionStore, ProjectionStoreError};
use foundry_records_draft::{ActionEnvelope, Receipt, SealedEnvelope};
use foundry_spine::{ProjectionState, WriteThroughError, fold_from_scratch, project_through};

mod write_through_support;
use write_through_support::FailsAt;

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

/// `ety_reading` declares `name` as its primary key, so the write-through
/// must stamp that designation from the registry for the store to
/// enforce it.
fn registry() -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    let definition = EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_reading").unwrap(),
        "Reading",
        vec![
            EntityTypePropertyDefinition::new("name", PropertyTier::Scalar, internal(), true)
                .unwrap(),
        ],
        1,
    )
    .unwrap()
    .with_primary_key_property("name");
    engine.register_entity_type(definition).unwrap();
    engine
        .register_link_type(
            LinkTypeDefinition::new(
                "ten_test",
                LinkTypeId::new("lty_measures").unwrap(),
                EntityTypeId::new("ety_reading").unwrap(),
                EntityTypeId::new("ety_reading").unwrap(),
                LinkCardinality::ManyToMany,
                false,
            )
            .unwrap(),
        )
        .unwrap();
    engine
        .register_action_type(
            ActionTypeDefinition::new(
                "ten_test",
                ActionTypeId::new("aty_calibrate").unwrap(),
                EntityTypeId::new("ety_reading").unwrap(),
                "ops-console",
                AutonomyTier::T1Assist,
                "reading.calibrated",
            )
            .unwrap(),
        )
        .unwrap();
    engine
}

fn name_property(value: &str) -> WireProperty {
    WireProperty::new(
        "name",
        WireTier::Scalar,
        WireDataClass::InternalOnly,
        WireValue::String(value.into()),
    )
    .unwrap()
}

fn sealed(object_ref: &str, ordinal: u64, name: &str) -> SealedEnvelope {
    let key = format!("idem_{ordinal}");
    let record = ActionRecord::new(
        "prn_alice",
        "dec_1",
        "reading.calibrated",
        &key,
        1_700_000_000_000,
        vec![],
        EditSet::new(vec![
            OntologyEdit::create_object("ety_reading", vec![name_property(name)]).unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    SealedEnvelope {
        envelope: ActionEnvelope::new(
            "ten_test",
            object_ref,
            "aty_calibrate",
            &key,
            1,
            encode_action_record(&record),
            1_700_000_000_000,
        )
        .unwrap(),
        receipt: Receipt {
            ordinal,
            object_sequence: 1,
            deduplicated: false,
        },
    }
}

fn corrupt(object_ref: &str, ordinal: u64) -> SealedEnvelope {
    let key = format!("idem_{ordinal}");
    SealedEnvelope {
        envelope: ActionEnvelope::new(
            "ten_test",
            object_ref,
            "aty_calibrate",
            &key,
            1,
            vec![0xFF],
            1_700_000_000_000,
        )
        .unwrap(),
        receipt: Receipt {
            ordinal,
            object_sequence: 1,
            deduplicated: false,
        },
    }
}

#[test]
fn the_store_equals_the_fold_of_the_log() {
    let registry = registry();
    let entries = vec![
        sealed("ent_r1", 1, "Ada"),
        sealed("ent_r2", 2, "Grace"),
        sealed("ent_r3", 3, "Katherine"),
    ];

    let mut state = ProjectionState::new("ten_test", &registry);
    let mut store = MemoryProjectionStore::default();
    let mirrored = project_through(&mut state, &mut store, &entries).expect("a healthy store");
    assert_eq!(mirrored, 3);

    // The independent authority: fold the same log from scratch.
    let folded = fold_from_scratch("ten_test", &registry, &entries);
    assert_eq!(
        store.applied_head("ten_test").unwrap(),
        folded.applied_ordinal
    );
    for (object_ref, binding) in &folded.bindings {
        let stored = store
            .get("ten_test", object_ref)
            .unwrap()
            .unwrap_or_else(|| panic!("{object_ref} must be in the store"));
        let projected = folded.objects.get("ten_test", object_ref).unwrap();
        assert_eq!(
            &stored.entity, projected,
            "{object_ref} differs from fold(log)"
        );
        assert_eq!(stored.schema_revision, binding.schema_revision);
        assert_eq!(stored.last_ordinal, binding.last_ordinal);
        assert_eq!(stored.last_actor, binding.last_actor);
    }
}

#[test]
fn a_store_outage_halts_and_never_poisons() {
    let registry = registry();
    let entries = vec![
        sealed("ent_r1", 1, "Ada"),
        sealed("ent_r2", 2, "Grace"),
        sealed("ent_r3", 3, "Katherine"),
    ];
    let mut state = ProjectionState::new("ten_test", &registry);
    let mut store = FailsAt {
        inner: MemoryProjectionStore::default(),
        fail_on_ordinal: 2,
        fail_head: false,
        fail_poisoned: false,
    };

    let failure = project_through(&mut state, &mut store, &entries)
        .expect_err("a refusing store must halt the runner");
    assert!(
        matches!(failure, WriteThroughError::Store { ordinal: 2, .. }),
        "the halt names the ordinal it stopped at: {failure:?}",
    );

    assert_eq!(
        store.applied_head("ten_test").unwrap(),
        1,
        "the store holds exactly the entries mirrored BEFORE the outage",
    );
    assert!(
        state.poison.is_empty(),
        "an outage is infrastructure; it never enters the poison ledger: {:?}",
        state.poison,
    );
    assert!(
        store.poisoned("ten_test").unwrap().is_empty(),
        "and it is never mirrored as a poison either",
    );
}

#[test]
fn a_poisoned_entry_mirrors_as_poisoned() {
    let registry = registry();
    // Ordinal 2 carries bytes that are not a canonical ActionRecord, so
    // it poisons WITHOUT breaking density — the store's own dense-ordinal
    // law still admits the mirror, which is the interaction that matters.
    let entries = vec![sealed("ent_r1", 1, "Ada"), corrupt("ent_r2", 2)];
    let mut state = ProjectionState::new("ten_test", &registry);
    let mut store = MemoryProjectionStore::default();
    project_through(&mut state, &mut store, &entries).expect("poisons do not halt the runner");

    let poisons = store.poisoned("ten_test").unwrap();
    assert_eq!(
        poisons,
        vec![(2, "payload_decode".to_owned())],
        "the poison reaches the store under its static label",
    );
    assert_eq!(
        store.applied_head("ten_test").unwrap(),
        2,
        "a poisoned entry still spends its ordinal in the store",
    );
}

#[test]
fn primary_key_designations_are_stamped_from_the_registry() {
    let registry = registry();
    // Two DIFFERENT objects claiming one key value: the registry says
    // `name` is the key, so the store must refuse the second.
    let entries = vec![sealed("ent_r1", 1, "Ada"), sealed("ent_r2", 2, "Ada")];
    let mut state = ProjectionState::new("ten_test", &registry);
    let mut store = MemoryProjectionStore::default();

    let failure = project_through(&mut state, &mut store, &entries)
        .expect_err("the duplicate key must reach the store as a refusal");
    assert!(
        matches!(
            failure,
            WriteThroughError::Store {
                ordinal: 2,
                error: ProjectionStoreError::DuplicatePrimaryKey { .. },
            }
        ),
        "the designation was stamped from the registry: {failure:?}",
    );
}
