//! Parameter plane: declared value types on action
//! parameters are enforced as the final conformance step, mirroring the
//! property plane — same `None`-requires-`String` rule, same precedence.

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

fn engine_with_action(parameters: Vec<ActionParameterDefinition>) -> OntologyEngine {
    let mut engine = engine_with(vec![
        EntityTypePropertyDefinition::new("name", PropertyTier::Scalar, internal(), true).unwrap(),
    ]);
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
            .unwrap()
            .with_parameters(parameters),
        )
        .unwrap();
    engine
}

fn check_params(
    engine: &OntologyEngine,
    values: &[ObjectProperty],
) -> Result<(), OntologyEngineError> {
    engine.check_action_parameter_conformance(
        "ten_test",
        &ActionTypeId::new("aty_calibrate").unwrap(),
        values,
    )
}

fn param_mismatch(
    name: &str,
    path: &str,
    expected: &'static str,
    found: &'static str,
) -> OntologyEngineError {
    OntologyEngineError::ParameterValueTypeMismatch {
        name: name.into(),
        path: path.into(),
        expected,
        found,
    }
}

#[test]
fn typed_parameter_rejects_wrong_carrier() {
    let engine = engine_with_action(vec![
        ActionParameterDefinition::typed("amount", int_decl(), internal(), true).unwrap(),
    ]);
    let submitted = [ObjectProperty::new(
        "amount".into(),
        "5".into(),
        PropertyTier::Scalar,
        internal(),
    )];
    assert_eq!(
        check_params(&engine, &submitted),
        Err(param_mismatch("amount", "", "integer", "string"))
    );
}

#[test]
fn typed_parameter_admits_matching_carrier() {
    let engine = engine_with_action(vec![
        ActionParameterDefinition::typed("amount", int_decl(), internal(), true).unwrap(),
    ]);
    let submitted = [ObjectProperty::typed(
        "amount".into(),
        PropertyValue::Integer(5),
        internal(),
    )];
    assert_eq!(check_params(&engine, &submitted), Ok(()));
}

#[test]
fn typed_parameter_array_mismatch_names_the_index() {
    let array = ValueTypeDeclaration::Array {
        element: Box::new(int_decl()),
    };
    let engine = engine_with_action(vec![
        ActionParameterDefinition::typed("samples", array, internal(), true).unwrap(),
    ]);
    let submitted = [ObjectProperty::typed(
        "samples".into(),
        PropertyValue::Array(vec![
            PropertyValue::Integer(1),
            PropertyValue::String("x".into()),
        ]),
        internal(),
    )];
    assert_eq!(
        check_params(&engine, &submitted),
        Err(param_mismatch("samples", "[1]", "integer", "string"))
    );
}

#[test]
fn untyped_parameter_requires_string_carrier() {
    let engine = engine_with_action(vec![
        ActionParameterDefinition::new("memo", PropertyTier::Scalar, internal(), true).unwrap(),
    ]);
    let submitted = [ObjectProperty::typed(
        "memo".into(),
        PropertyValue::Boolean(true),
        internal(),
    )];
    assert_eq!(
        check_params(&engine, &submitted),
        Err(param_mismatch("memo", "", "string", "boolean"))
    );
}

#[test]
fn parameter_data_class_mismatch_precedes_value_check() {
    let engine = engine_with_action(vec![
        ActionParameterDefinition::typed("amount", int_decl(), internal(), true).unwrap(),
    ]);
    let submitted = [ObjectProperty::new(
        "amount".into(),
        "5".into(),
        PropertyTier::Scalar,
        pii(),
    )];
    assert_eq!(
        check_params(&engine, &submitted),
        Err(OntologyEngineError::ParameterDataClassMismatch {
            name: "amount".into()
        })
    );
}
