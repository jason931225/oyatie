//! Shared fixture for the migration-runner suites: a two-revision
//! registry, hand-sealed rev-1 objects, and the plan and authority
//! under test.

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    ActionTypeDefinition, ActionTypeId, AutonomyTier, EntityTypeDefinition, EntityTypeId,
    EntityTypePropertyDefinition, OntologyEngine, PropertyTier, ScalarType, ValueTypeDeclaration,
};
use foundry_edits::{
    ActionRecord, EditSet, OntologyEdit, WireDataClass, WireProperty, WireTier, WireValue,
    encode_action_record,
};
use foundry_records_draft::{ActionEnvelope, Receipt, SealedEnvelope};
use foundry_spine::{
    DefaultValue, MigrationAuthority, MigrationPlan, ProjectionState, UpcastTransform,
    ValueConversion, fold_from_scratch,
};

pub fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

pub fn untyped(name: &str, required: bool) -> EntityTypePropertyDefinition {
    EntityTypePropertyDefinition::new(name, PropertyTier::Scalar, internal(), required).unwrap()
}

pub fn typed(name: &str, scalar: ScalarType) -> EntityTypePropertyDefinition {
    let mut property =
        EntityTypePropertyDefinition::new(name, PropertyTier::Scalar, internal(), false).unwrap();
    property.value_type = Some(ValueTypeDeclaration::Scalar(scalar));
    property
}

fn action(engine: &mut OntologyEngine, id: &str, surface: &str, audit: &str) {
    engine
        .register_action_type(
            ActionTypeDefinition::new(
                "ten_test",
                ActionTypeId::new(id).unwrap(),
                EntityTypeId::new("ety_reading").unwrap(),
                surface,
                AutonomyTier::T1Assist,
                audit,
            )
            .unwrap(),
        )
        .unwrap();
}

/// Head is revision 2; revision 1 is retained. Two action types: the
/// ordinary write action and the per-plan migration action.
pub fn registry() -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(
            EntityTypeDefinition::new(
                "ten_test",
                EntityTypeId::new("ety_reading").unwrap(),
                "Reading",
                vec![untyped("name", true), typed("score", ScalarType::Integer)],
                1,
            )
            .unwrap(),
        )
        .unwrap();
    engine
        .evolve_entity_type(
            EntityTypeDefinition::new(
                "ten_test",
                EntityTypeId::new("ety_reading").unwrap(),
                "Reading",
                vec![
                    untyped("name", true),
                    typed("score", ScalarType::Integer),
                    typed("score_text", ScalarType::String),
                    untyped("grade", false),
                ],
                2,
            )
            .unwrap(),
        )
        .unwrap();
    action(
        &mut engine,
        "aty_calibrate",
        "ops-console",
        "reading.calibrated",
    );
    action(
        &mut engine,
        "aty_upcast_reading_2",
        "migration-console",
        "reading.upcast_to_2",
    );
    engine
}

mod log;
pub use log::MemoryLog;

pub fn wire_string(name: &str, value: &str) -> WireProperty {
    WireProperty::new(
        name,
        WireTier::Scalar,
        WireDataClass::InternalOnly,
        WireValue::String(value.into()),
    )
    .unwrap()
}

pub fn wire_integer(name: &str, value: i64) -> WireProperty {
    WireProperty::new(
        name,
        WireTier::Scalar,
        WireDataClass::InternalOnly,
        WireValue::Integer(value),
    )
    .unwrap()
}

fn sealed(
    object_ref: &str,
    ordinal: u64,
    schema_revision: u32,
    edit: OntologyEdit,
    object_sequence: u64,
) -> SealedEnvelope {
    let key = format!("idem_{ordinal}");
    let record = ActionRecord::new(
        "prn_alice",
        "dec_seed",
        "reading.calibrated",
        &key,
        1_700_000_000_000,
        vec![],
        EditSet::new(vec![edit]).unwrap(),
    )
    .unwrap();
    SealedEnvelope {
        envelope: ActionEnvelope::new(
            "ten_test",
            object_ref,
            "aty_calibrate",
            &key,
            schema_revision,
            encode_action_record(&record),
            1_700_000_000_000,
        )
        .unwrap(),
        receipt: Receipt {
            ordinal,
            object_sequence,
            deduplicated: false,
        },
    }
}

pub fn sealed_create(
    object_ref: &str,
    ordinal: u64,
    schema_revision: u32,
    properties: Vec<WireProperty>,
) -> SealedEnvelope {
    sealed(
        object_ref,
        ordinal,
        schema_revision,
        OntologyEdit::create_object("ety_reading", properties).unwrap(),
        1,
    )
}

pub fn sealed_upsert(
    object_ref: &str,
    ordinal: u64,
    schema_revision: u32,
    properties: Vec<WireProperty>,
) -> SealedEnvelope {
    sealed(
        object_ref,
        ordinal,
        schema_revision,
        OntologyEdit::upsert_properties(properties).unwrap(),
        2,
    )
}

/// `ent_a` (revision 1, score 7, no targets) is owed; `ent_b` (revision 1
/// but every target already correct) is not — pending is a VALUE predicate.
pub fn fixture() -> (OntologyEngine, MemoryLog, ProjectionState) {
    let engine = registry();
    let a = sealed_create(
        "ent_a",
        1,
        1,
        vec![wire_string("name", "Ada"), wire_integer("score", 7)],
    );
    let b = sealed_create(
        "ent_b",
        2,
        1,
        vec![
            wire_string("name", "Bea"),
            wire_integer("score", 9),
            wire_string("score_text", "9"),
            wire_string("grade", "B"),
        ],
    );
    let state = fold_from_scratch("ten_test", &engine, [&a, &b]);
    assert!(state.poison.is_empty(), "fixture folds clean");
    let mut log = MemoryLog::default();
    log.seed(a);
    log.seed(b);
    (engine, log, state)
}

pub fn plan() -> MigrationPlan {
    MigrationPlan {
        tenant_id: "ten_test".into(),
        entity_type: "ety_reading".into(),
        from_revision: 1,
        to_revision: 2,
        action_type: "aty_upcast_reading_2".into(),
        audit_event_type: "reading.upcast_to_2".into(),
        declared_at_epoch_seconds: 1_700_000_100,
        transforms: vec![
            UpcastTransform::ConvertAs {
                from: "score".into(),
                to: "score_text".into(),
                conversion: ValueConversion::IntegerToString,
            },
            UpcastTransform::DefaultTo {
                to: "grade".into(),
                value: DefaultValue::String("F".into()),
            },
        ],
    }
}

pub fn authority() -> MigrationAuthority {
    MigrationAuthority {
        principal_id: "prn_migrator".into(),
        decision_id: "dec_migration_run".into(),
        allowed_surfaces: vec!["migration-console".into()],
        autonomy_tier: AutonomyTier::T1Assist,
    }
}
