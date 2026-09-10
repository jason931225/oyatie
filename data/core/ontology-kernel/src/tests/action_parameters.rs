//! Typed action parameters: declared schema on the action type, duplicate
//! rejection at registration, and fail-closed conformance of submitted
//! values.

use crate::*;
use data_boundary_kernel::{DataClass, PrivacyDataClass};

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn pii() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::PiiIdentifying).unwrap()
}

fn param(name: &str, required: bool) -> ActionParameterDefinition {
    ActionParameterDefinition::new(name, PropertyTier::Scalar, internal(), required).unwrap()
}

fn value(name: &str, data_class: PrivacyDataClass) -> ObjectProperty {
    ObjectProperty::new(
        name.into(),
        "value".into(),
        PropertyTier::Scalar,
        data_class,
    )
}

fn engine_with_discharge_action(parameters: Vec<ActionParameterDefinition>) -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(
            EntityTypeDefinition::new(
                "ten_test",
                EntityTypeId::new("ety_patient").unwrap(),
                "Patient",
                vec![
                    EntityTypePropertyDefinition::new(
                        "name",
                        PropertyTier::Scalar,
                        internal(),
                        true,
                    )
                    .unwrap(),
                ],
                1,
            )
            .unwrap(),
        )
        .unwrap();
    engine
        .register_action_type(
            ActionTypeDefinition::new(
                "ten_test",
                ActionTypeId::new("aty_discharge").unwrap(),
                EntityTypeId::new("ety_patient").unwrap(),
                "clinical-console",
                AutonomyTier::T1Assist,
                "patient.discharged",
            )
            .unwrap()
            .with_parameters(parameters),
        )
        .unwrap();
    engine
}

fn aty() -> ActionTypeId {
    ActionTypeId::new("aty_discharge").unwrap()
}

#[test]
fn blank_parameter_name_rejected_at_construction() {
    assert_eq!(
        ActionParameterDefinition::new("  ", PropertyTier::Scalar, internal(), true),
        Err(OntologyEngineError::EmptyParameterName)
    );
}

#[test]
fn duplicate_parameter_names_rejected_at_registration() {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(
            EntityTypeDefinition::new(
                "ten_test",
                EntityTypeId::new("ety_patient").unwrap(),
                "Patient",
                vec![
                    EntityTypePropertyDefinition::new(
                        "name",
                        PropertyTier::Scalar,
                        internal(),
                        true,
                    )
                    .unwrap(),
                ],
                1,
            )
            .unwrap(),
        )
        .unwrap();

    let result = engine.register_action_type(
        ActionTypeDefinition::new(
            "ten_test",
            ActionTypeId::new("aty_discharge").unwrap(),
            EntityTypeId::new("ety_patient").unwrap(),
            "clinical-console",
            AutonomyTier::T1Assist,
            "patient.discharged",
        )
        .unwrap()
        .with_parameters(vec![param("reason", true), param("reason", false)]),
    );
    assert_eq!(
        result,
        Err(OntologyEngineError::DuplicateParameterName {
            name: "reason".into()
        })
    );
}

#[test]
fn conformant_submission_accepted() {
    let engine = engine_with_discharge_action(vec![param("reason", true), param("note", false)]);
    assert_eq!(
        engine.check_action_parameter_conformance(
            "ten_test",
            &aty(),
            &[value("reason", internal())]
        ),
        Ok(())
    );
}

#[test]
fn unknown_action_type_rejected() {
    let engine = engine_with_discharge_action(vec![]);
    assert_eq!(
        engine.check_action_parameter_conformance(
            "ten_test",
            &ActionTypeId::new("aty_absent").unwrap(),
            &[]
        ),
        Err(OntologyEngineError::UnknownActionType)
    );
}

#[test]
fn missing_required_parameter_rejected() {
    let engine = engine_with_discharge_action(vec![param("reason", true)]);
    assert_eq!(
        engine.check_action_parameter_conformance("ten_test", &aty(), &[]),
        Err(OntologyEngineError::MissingRequiredParameter {
            name: "reason".into()
        })
    );
}

#[test]
fn undeclared_parameter_rejected() {
    let engine = engine_with_discharge_action(vec![]);
    assert_eq!(
        engine.check_action_parameter_conformance(
            "ten_test",
            &aty(),
            &[value("surprise", internal())]
        ),
        Err(OntologyEngineError::UndeclaredParameter {
            name: "surprise".into()
        })
    );
}

#[test]
fn parameter_tier_mismatch_rejected() {
    let engine = engine_with_discharge_action(vec![param("reason", true)]);
    let mismatched = ObjectProperty::new(
        "reason".into(),
        "value".into(),
        PropertyTier::Vector,
        internal(),
    );
    assert_eq!(
        engine.check_action_parameter_conformance("ten_test", &aty(), &[mismatched]),
        Err(OntologyEngineError::ParameterTierMismatch {
            name: "reason".into()
        })
    );
}

#[test]
fn parameter_data_class_mismatch_rejected() {
    let engine = engine_with_discharge_action(vec![param("reason", true)]);
    assert_eq!(
        engine.check_action_parameter_conformance("ten_test", &aty(), &[value("reason", pii())]),
        Err(OntologyEngineError::ParameterDataClassMismatch {
            name: "reason".into()
        })
    );
}
