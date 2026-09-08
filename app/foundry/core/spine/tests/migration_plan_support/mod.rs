//! Fixtures for the plan-validation suite.
//!
//! Lifted out because the suite reached the 300-line budget with a registry
//! that did not register the plan's action — a fixture less faithful than
//! production, and the reason `validate` could accept an action the registry
//! never held without any test noticing.

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    ActionTypeDefinition, ActionTypeId, AutonomyTier, EntityTypeDefinition, EntityTypeId,
    EntityTypePropertyDefinition, OntologyEngine, PropertyTier, ScalarType, ValueTypeDeclaration,
};
use foundry_spine::{MigrationPlan, UpcastTransform};

pub(crate) fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

pub(crate) fn untyped(name: &str, required: bool) -> EntityTypePropertyDefinition {
    EntityTypePropertyDefinition::new(name, PropertyTier::Scalar, internal(), required).unwrap()
}

pub(crate) fn typed(name: &str, scalar: ScalarType) -> EntityTypePropertyDefinition {
    let mut property =
        EntityTypePropertyDefinition::new(name, PropertyTier::Scalar, internal(), false).unwrap();
    property.value_type = Some(ValueTypeDeclaration::Scalar(scalar));
    property
}

pub(crate) fn rev1_properties() -> Vec<EntityTypePropertyDefinition> {
    vec![
        untyped("serial", true),
        untyped("note", true),
        typed("score", ScalarType::Integer),
        typed("flag", ScalarType::Boolean),
    ]
}

pub(crate) fn rev2_properties() -> Vec<EntityTypePropertyDefinition> {
    let mut properties = rev1_properties();
    properties.push(typed("score_text", ScalarType::String));
    properties.push(typed("score_copy", ScalarType::Integer));
    properties.push(typed("flag_rank", ScalarType::Integer));
    properties.push(untyped("grade", false));
    properties
}

pub(crate) fn definition(
    revision: u32,
    properties: Vec<EntityTypePropertyDefinition>,
) -> EntityTypeDefinition {
    EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_reading").unwrap(),
        "Reading",
        properties,
        revision,
    )
    .unwrap()
    .with_primary_key_property("serial")
}

/// Revision 1 -> revision 2, both retained; head is 2.
pub(crate) fn registry() -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(definition(1, rev1_properties()))
        .unwrap();
    engine
        .evolve_entity_type(definition(2, rev2_properties()))
        .unwrap();
    // The plan's action, registered as the seed registers one. Without it
    // this fixture was less faithful than production and could not have
    // noticed `validate` accepting an action the registry never held.
    engine.register_action_type(action()).unwrap();
    engine
}

pub(crate) fn action() -> ActionTypeDefinition {
    ActionTypeDefinition::new(
        "ten_test",
        ActionTypeId::new("aty_upcast_reading_2").unwrap(),
        EntityTypeId::new("ety_reading").unwrap(),
        "ops-console",
        AutonomyTier::T1Assist,
        "reading.upcast",
    )
    .unwrap()
}

pub(crate) fn plan(transforms: Vec<UpcastTransform>) -> MigrationPlan {
    MigrationPlan {
        tenant_id: "ten_test".into(),
        entity_type: "ety_reading".into(),
        from_revision: 1,
        to_revision: 2,
        action_type: "aty_upcast_reading_2".into(),
        audit_event_type: "reading.upcast_to_2".into(),
        declared_at_epoch_seconds: 1_700_000_000,
        transforms,
    }
}

pub(crate) fn copy(from: &str, to: &str) -> UpcastTransform {
    UpcastTransform::CopyAs {
        from: from.into(),
        to: to.into(),
    }
}
