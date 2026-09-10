//! Checkpoint law: resume equals fold-from-scratch, a foreign registry
//! discards the checkpoint (mechanized rebuild-on-evolve), and sync
//! status reports the projection's honest position.

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    ActionTypeDefinition, ActionTypeId, AutonomyTier, EntityTypeDefinition, EntityTypeId,
    EntityTypePropertyDefinition, OntologyEngine, PropertyTier,
};
use foundry_edits::{
    ActionRecord, EditSet, OntologyEdit, WireDataClass, WireProperty, WireTier, WireValue,
    encode_action_record,
};
use foundry_records_draft::{ActionEnvelope, Receipt, SealedEnvelope};
use foundry_spine::{Checkpoint, ProjectionState, apply_sealed, fold_from_scratch};

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn reading_type(revision: u32, extra_optional: bool) -> EntityTypeDefinition {
    let mut properties = vec![
        EntityTypePropertyDefinition::new("name", PropertyTier::Scalar, internal(), true).unwrap(),
    ];
    if extra_optional {
        properties.push(
            EntityTypePropertyDefinition::new("unit", PropertyTier::Scalar, internal(), false)
                .unwrap(),
        );
    }
    EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_reading").unwrap(),
        "Reading",
        properties,
        revision,
    )
    .unwrap()
}

fn registry() -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    engine.register_entity_type(reading_type(1, false)).unwrap();
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

fn evolved_registry() -> OntologyEngine {
    let mut engine = registry();
    engine.evolve_entity_type(reading_type(2, true)).unwrap();
    engine
}

fn sealed(object_ref: &str, ordinal: u64, schema_revision: u32, name: &str) -> SealedEnvelope {
    let key = format!("idem_{ordinal}");
    let record = ActionRecord::new(
        "prn_alice",
        "dec_1",
        "reading.calibrated",
        &key,
        1_700_000_000_000,
        vec![],
        EditSet::new(vec![
            OntologyEdit::create_object(
                "ety_reading",
                vec![
                    WireProperty::new(
                        "name",
                        WireTier::Scalar,
                        WireDataClass::InternalOnly,
                        WireValue::String(name.into()),
                    )
                    .unwrap(),
                ],
            )
            .unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    SealedEnvelope {
        envelope: ActionEnvelope::new(
            "ten_test",
            object_ref,
            "aty_calibrate",
            key,
            schema_revision,
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

fn log_entries() -> Vec<SealedEnvelope> {
    vec![
        sealed("ent_r1", 1, 1, "Ada"),
        sealed("ent_r2", 2, 2, "Grace"), // revision-ahead: poisons at rev 1
        sealed("ent_r3", 3, 1, "Edsger"),
    ]
}

#[test]
fn resume_from_checkpoint_equals_fold_from_scratch() {
    let registry = registry();
    let entries = log_entries();
    let mut live = ProjectionState::new("ten_test", &registry);
    apply_sealed(&mut live, &entries[0]);
    let checkpoint = Checkpoint::capture(&live);
    assert_eq!(checkpoint.applied_ordinal(), 1);

    let resumed = checkpoint.resume(&registry, &entries);
    assert_eq!(resumed, fold_from_scratch("ten_test", &registry, &entries));
    assert_eq!(resumed.applied_ordinal, 3);
    assert_eq!(resumed.poison.len(), 1);
}

#[test]
fn a_foreign_registry_discards_the_checkpoint() {
    let registry = registry();
    let entries = log_entries();
    let mut live = ProjectionState::new("ten_test", &registry);
    for entry in &entries {
        apply_sealed(&mut live, entry);
    }
    // The rev-2 entry poisoned under the rev-1 registry snapshot.
    assert_eq!(live.poison.keys().copied().collect::<Vec<_>>(), vec![2]);

    let evolved = evolved_registry();
    let resumed = Checkpoint::capture(&live).resume(&evolved, &entries);
    assert_eq!(resumed, fold_from_scratch("ten_test", &evolved, &entries));
    assert!(resumed.poison.is_empty());
    assert!(resumed.objects.get("ten_test", "ent_r2").is_some());
}

#[test]
fn sync_status_reports_the_honest_position() {
    let registry = registry();
    let entries = log_entries();
    let mut live = ProjectionState::new("ten_test", &registry);
    for entry in &entries[..2] {
        apply_sealed(&mut live, entry);
    }
    let status = live.sync_status(5);
    assert_eq!(status.applied_ordinal, 2);
    assert_eq!(status.head, 5);
    assert_eq!(status.lag, 3);
    assert_eq!(status.poisoned_count, 1);
    assert_eq!(status.first_poisoned_ordinal, Some(2));

    let fresh = ProjectionState::new("ten_test", &registry);
    let empty = fresh.sync_status(0);
    assert_eq!(empty.lag, 0);
    assert_eq!(empty.first_poisoned_ordinal, None);
}
