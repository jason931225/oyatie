//! Pins: value-type declarations bind to properties and parameters,
//! registration re-validates them, tier coherence is enforced for
//! literal-built definitions, and the evolution freeze covers the
//! quadruple — Some immutable, None -> Some rejected.

use crate::*;
use data_boundary_kernel::{DataClass, PrivacyDataClass};

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn int_decl() -> ValueTypeDeclaration {
    ValueTypeDeclaration::Scalar(ScalarType::Integer)
}

fn def_with(revision: u32, properties: Vec<EntityTypePropertyDefinition>) -> EntityTypeDefinition {
    EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_measure").unwrap(),
        "Measure",
        properties,
        revision,
    )
    .unwrap()
}

#[test]
fn typed_constructors_derive_tier() {
    let prop = EntityTypePropertyDefinition::typed("count", int_decl(), internal(), true).unwrap();
    assert_eq!(prop.tier, PropertyTier::Scalar);
    assert_eq!(prop.value_type, Some(int_decl()));

    let array = ValueTypeDeclaration::Array {
        element: Box::new(int_decl()),
    };
    let param = ActionParameterDefinition::typed("samples", array, internal(), false).unwrap();
    assert_eq!(param.tier, PropertyTier::Vector);
}

#[test]
fn malformed_declaration_rejected_at_registration() {
    let bad = ValueTypeDeclaration::Struct(StructSchema { fields: vec![] });
    let mut prop =
        EntityTypePropertyDefinition::new("cfg", PropertyTier::Struct, internal(), true).unwrap();
    prop.value_type = Some(bad);

    let mut engine = OntologyEngine::default();
    assert_eq!(
        engine.register_entity_type(def_with(1, vec![prop])),
        Err(OntologyEngineError::InvalidValueType {
            name: "cfg".into(),
            cause: ValueTypeError::EmptyStructSchema
        })
    );
}

#[test]
fn tier_incoherence_rejected() {
    let mut stated_wrong =
        EntityTypePropertyDefinition::new("count", PropertyTier::Vector, internal(), true).unwrap();
    stated_wrong.value_type = Some(int_decl());
    let mut engine = OntologyEngine::default();
    assert_eq!(
        engine.register_entity_type(def_with(1, vec![stated_wrong])),
        Err(OntologyEngineError::ValueTypeTierMismatch {
            name: "count".into()
        })
    );

    let mut exotic =
        EntityTypePropertyDefinition::new("series", PropertyTier::Timeseries, internal(), true)
            .unwrap();
    exotic.value_type = Some(int_decl());
    assert_eq!(
        engine.register_entity_type(def_with(1, vec![exotic])),
        Err(OntologyEngineError::ValueTypeTierMismatch {
            name: "series".into()
        })
    );
}

#[test]
fn parameter_declarations_validated_at_registration() {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(def_with(
            1,
            vec![
                EntityTypePropertyDefinition::new("name", PropertyTier::Scalar, internal(), true)
                    .unwrap(),
            ],
        ))
        .unwrap();

    let mut bad =
        ActionParameterDefinition::new("p", PropertyTier::Scalar, internal(), true).unwrap();
    bad.value_type = Some(ValueTypeDeclaration::Struct(StructSchema {
        fields: vec![],
    }));
    let result = engine.register_action_type(
        ActionTypeDefinition::new(
            "ten_test",
            ActionTypeId::new("aty_measure").unwrap(),
            EntityTypeId::new("ety_measure").unwrap(),
            "console",
            AutonomyTier::T1Assist,
            "measure.recorded",
        )
        .unwrap()
        .with_parameters(vec![bad]),
    );
    assert_eq!(
        result,
        Err(OntologyEngineError::InvalidValueType {
            name: "p".into(),
            cause: ValueTypeError::EmptyStructSchema
        })
    );
}

#[test]
fn value_type_frozen_across_revisions() {
    let typed = EntityTypePropertyDefinition::typed("count", int_decl(), internal(), true).unwrap();
    let untyped =
        EntityTypePropertyDefinition::new("legacy", PropertyTier::Scalar, internal(), true)
            .unwrap();

    let mut engine = OntologyEngine::default();
    engine
        .evolve_entity_type(def_with(1, vec![typed.clone(), untyped.clone()]))
        .unwrap();

    // Some -> different Some: rejected.
    let retyped = EntityTypePropertyDefinition::typed(
        "count",
        ValueTypeDeclaration::Scalar(ScalarType::Double),
        internal(),
        true,
    )
    .unwrap();
    assert_eq!(
        engine.evolve_entity_type(def_with(2, vec![retyped, untyped.clone()])),
        Err(OntologyEngineError::IncompatibleSchemaEvolution)
    );

    // Some -> None: rejected.
    let stripped =
        EntityTypePropertyDefinition::new("count", PropertyTier::Scalar, internal(), true).unwrap();
    assert_eq!(
        engine.evolve_entity_type(def_with(2, vec![stripped, untyped.clone()])),
        Err(OntologyEngineError::IncompatibleSchemaEvolution)
    );

    // None -> Some in-place typing: rejected.
    let now_typed =
        EntityTypePropertyDefinition::typed("legacy", int_decl(), internal(), true).unwrap();
    assert_eq!(
        engine.evolve_entity_type(def_with(2, vec![typed.clone(), now_typed])),
        Err(OntologyEngineError::IncompatibleSchemaEvolution)
    );

    // Unchanged quadruple plus a NEW optional typed property: the blessed
    // idiom, accepted.
    let mut new_optional =
        EntityTypePropertyDefinition::typed("count2", int_decl(), internal(), false).unwrap();
    new_optional.value_type = Some(int_decl());
    engine
        .evolve_entity_type(def_with(2, vec![typed, untyped, new_optional]))
        .expect("new optional typed property must stay additive");
}
