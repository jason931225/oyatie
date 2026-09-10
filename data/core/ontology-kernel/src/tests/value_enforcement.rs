//! Pins: declared value types are ENFORCED — the shared checker runs
//! as the final per-property / per-parameter conformance step, an untyped
//! (`None`) declaration requires the legacy `String` carrier, and every
//! pre-existing conformance error still wins its precedence slot.

use std::collections::BTreeMap;

use crate::*;
use data_boundary_kernel::{DataClass, PrivacyDataClass};

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn pii() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::PiiIdentifying).unwrap()
}

fn int_decl() -> ValueTypeDeclaration {
    ValueTypeDeclaration::Scalar(ScalarType::Integer)
}

/// Struct { metrics: Array<Struct { count: Integer, required }>, required }
fn nested_decl() -> ValueTypeDeclaration {
    ValueTypeDeclaration::Struct(StructSchema {
        fields: vec![StructFieldDeclaration {
            name: "metrics".into(),
            value_type: ValueTypeDeclaration::Array {
                element: Box::new(ValueTypeDeclaration::Struct(StructSchema {
                    fields: vec![StructFieldDeclaration {
                        name: "count".into(),
                        value_type: int_decl(),
                        required: true,
                    }],
                })),
            },
            required: true,
        }],
    })
}

fn struct_value(entries: Vec<(&str, PropertyValue)>) -> PropertyValue {
    PropertyValue::Struct(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn engine_with(properties: Vec<EntityTypePropertyDefinition>) -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(
            EntityTypeDefinition::new(
                "ten_test",
                EntityTypeId::new("ety_reading").unwrap(),
                "Reading",
                properties,
                1,
            )
            .unwrap(),
        )
        .unwrap();
    engine
}

fn entity(properties: Vec<ObjectProperty>) -> ObjectEntity {
    ObjectEntity::new(
        "ten_test".into(),
        "ent_reading_1".into(),
        "ety_reading".into(),
        properties,
    )
    .unwrap()
}

fn value_mismatch(
    name: &str,
    path: &str,
    expected: &'static str,
    found: &'static str,
) -> OntologyEngineError {
    OntologyEngineError::PropertyValueTypeMismatch {
        name: name.into(),
        path: path.into(),
        expected,
        found,
    }
}

#[test]
fn typed_declaration_rejects_wrong_carrier() {
    let engine = engine_with(vec![
        EntityTypePropertyDefinition::typed("count", int_decl(), internal(), true).unwrap(),
    ]);
    let entity = entity(vec![ObjectProperty::new(
        "count".into(),
        "3".into(),
        PropertyTier::Scalar,
        internal(),
    )]);
    assert_eq!(
        engine.check_instance_conformance(&entity),
        Err(value_mismatch("count", "", "integer", "string"))
    );
}

#[test]
fn typed_declaration_admits_matching_carrier() {
    let engine = engine_with(vec![
        EntityTypePropertyDefinition::typed("count", int_decl(), internal(), true).unwrap(),
    ]);
    let entity = entity(vec![ObjectProperty::typed(
        "count".into(),
        PropertyValue::Integer(3),
        internal(),
    )]);
    assert_eq!(engine.check_instance_conformance(&entity), Ok(()));
}

#[test]
fn nested_mismatch_names_the_full_path() {
    let engine = engine_with(vec![
        EntityTypePropertyDefinition::typed("cfg", nested_decl(), internal(), true).unwrap(),
    ]);
    let bad = struct_value(vec![(
        "metrics",
        PropertyValue::Array(vec![struct_value(vec![(
            "count",
            PropertyValue::String("x".into()),
        )])]),
    )]);
    let entity = entity(vec![ObjectProperty::typed("cfg".into(), bad, internal())]);
    assert_eq!(
        engine.check_instance_conformance(&entity),
        Err(value_mismatch(
            "cfg",
            "metrics[0].count",
            "integer",
            "string"
        ))
    );
}

#[test]
fn missing_required_struct_field_is_named() {
    let engine = engine_with(vec![
        EntityTypePropertyDefinition::typed("cfg", nested_decl(), internal(), true).unwrap(),
    ]);
    let bad = struct_value(vec![(
        "metrics",
        PropertyValue::Array(vec![struct_value(vec![])]),
    )]);
    let entity = entity(vec![ObjectProperty::typed("cfg".into(), bad, internal())]);
    assert_eq!(
        engine.check_instance_conformance(&entity),
        Err(value_mismatch(
            "cfg",
            "metrics[0].count",
            "integer",
            "absent"
        ))
    );
}

#[test]
fn undeclared_struct_field_fails_closed() {
    let engine = engine_with(vec![
        EntityTypePropertyDefinition::typed("cfg", nested_decl(), internal(), true).unwrap(),
    ]);
    let bad = struct_value(vec![
        (
            "metrics",
            PropertyValue::Array(vec![struct_value(vec![(
                "count",
                PropertyValue::Integer(1),
            )])]),
        ),
        ("extra", PropertyValue::Boolean(true)),
    ]);
    let entity = entity(vec![ObjectProperty::typed("cfg".into(), bad, internal())]);
    assert_eq!(
        engine.check_instance_conformance(&entity),
        Err(value_mismatch("cfg", "extra", "absent", "boolean"))
    );
}

#[test]
fn untyped_declaration_requires_string_carrier() {
    let engine = engine_with(vec![
        EntityTypePropertyDefinition::new("note", PropertyTier::Scalar, internal(), true).unwrap(),
    ]);
    let entity = entity(vec![ObjectProperty::typed(
        "note".into(),
        PropertyValue::Integer(3),
        internal(),
    )]);
    assert_eq!(
        engine.check_instance_conformance(&entity),
        Err(value_mismatch("note", "", "string", "integer"))
    );
}

#[test]
fn untyped_declaration_admits_bridge_carrier() {
    let engine = engine_with(vec![
        EntityTypePropertyDefinition::new("note", PropertyTier::Scalar, internal(), true).unwrap(),
    ]);
    let entity = entity(vec![ObjectProperty::new(
        "note".into(),
        "free text".into(),
        PropertyTier::Scalar,
        internal(),
    )]);
    assert_eq!(engine.check_instance_conformance(&entity), Ok(()));
}

#[test]
fn tier_mismatch_precedes_value_check() {
    let engine = engine_with(vec![
        EntityTypePropertyDefinition::typed("count", int_decl(), internal(), true).unwrap(),
    ]);
    let entity = entity(vec![ObjectProperty::new(
        "count".into(),
        "3".into(),
        PropertyTier::Vector,
        internal(),
    )]);
    assert_eq!(
        engine.check_instance_conformance(&entity),
        Err(OntologyEngineError::PropertyTierMismatch {
            name: "count".into()
        })
    );
}

#[test]
fn data_class_mismatch_precedes_value_check() {
    let engine = engine_with(vec![
        EntityTypePropertyDefinition::typed("count", int_decl(), internal(), true).unwrap(),
    ]);
    let entity = entity(vec![ObjectProperty::new(
        "count".into(),
        "3".into(),
        PropertyTier::Scalar,
        pii(),
    )]);
    assert_eq!(
        engine.check_instance_conformance(&entity),
        Err(OntologyEngineError::PropertyDataClassMismatch {
            name: "count".into()
        })
    );
}
