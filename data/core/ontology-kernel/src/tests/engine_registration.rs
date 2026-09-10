use crate::*;
use data_boundary_kernel::{DataClass, PrivacyDataClass};

use super::support::property;

fn patient_type() -> EntityTypeDefinition {
    EntityTypeDefinition::new(
        "ten_clinic",
        EntityTypeId::new("ety_patient").unwrap(),
        "Patient",
        vec![property("mrn")],
        1,
    )
    .unwrap()
}
#[test]
fn ontology_engine_registers_entity_types_and_rejects_conflicts() {
    let mut engine = OntologyEngine::default();
    let id = engine.register_entity_type(patient_type()).unwrap();
    assert_eq!(id.value, "ety_patient");
    assert!(engine.entity_type("ten_clinic", &id).is_some());
    assert_eq!(
        engine.register_entity_type(patient_type()),
        Err(OntologyEngineError::DuplicateEntityType)
    );
}
#[test]
fn ontology_engine_type_checks_links_before_registration() {
    let mut engine = OntologyEngine::default();
    let patient = engine.register_entity_type(patient_type()).unwrap();
    let appointment = engine
        .register_entity_type(
            EntityTypeDefinition::new(
                "ten_clinic",
                EntityTypeId::new("ety_appointment").unwrap(),
                "Appointment",
                vec![property("starts_at")],
                1,
            )
            .unwrap(),
        )
        .unwrap();
    let link = LinkTypeDefinition::new(
        "ten_clinic",
        LinkTypeId::new("lty_patient_appointment").unwrap(),
        patient.clone(),
        appointment,
        LinkCardinality::OneToMany,
        false,
    )
    .unwrap();
    assert_eq!(
        engine.register_link_type(link),
        Ok(LinkTypeId {
            value: "lty_patient_appointment".to_string()
        })
    );
    let unknown = LinkTypeDefinition::new(
        "ten_clinic",
        LinkTypeId::new("lty_unknown").unwrap(),
        patient,
        EntityTypeId::new("ety_missing").unwrap(),
        LinkCardinality::OneToOne,
        false,
    )
    .unwrap();
    assert_eq!(
        engine.register_link_type(unknown),
        Err(OntologyEngineError::UnknownEntityTypeEndpoint)
    );
}
#[test]
fn ontology_engine_gates_action_invocation_by_policy_and_autonomy() {
    let mut engine = OntologyEngine::default();
    let patient = engine.register_entity_type(patient_type()).unwrap();
    engine
        .register_action_type(
            ActionTypeDefinition::new(
                "ten_clinic",
                ActionTypeId::new("aty_discharge_patient").unwrap(),
                patient,
                "ontology.action.discharge_patient",
                AutonomyTier::T1Assist,
                "EVT-ONTOLOGY-ACTION-INVOKED",
            )
            .unwrap(),
        )
        .unwrap();
    let request = ActionInvocationRequest {
        tenant_id: "ten_clinic".to_string(),
        principal_id: "usr_alice".to_string(),
        action_id: ActionTypeId::new("aty_discharge_patient").unwrap(),
        entity_id: "ent_patient_001".to_string(),
        idempotency_key: "idem-001".to_string(),
        requested_at_epoch_seconds: 1_779_523_600,
    };
    let decision = ActionPolicyDecision {
        decision_id: "dec_001".to_string(),
        tenant_id: "ten_clinic".to_string(),
        principal_id: "usr_alice".to_string(),
        allowed_surfaces: vec!["ontology.action.discharge_patient".to_string()],
        autonomy_tier: AutonomyTier::T1Assist,
    };
    let receipt = engine
        .authorize_action_invocation(request.clone(), decision.clone())
        .unwrap();
    assert_eq!(receipt.audit_event_type, "EVT-ONTOLOGY-ACTION-INVOKED");
    let denied = engine
        .authorize_action_invocation(
            request.clone(),
            ActionPolicyDecision {
                allowed_surfaces: vec!["ontology.action.other".to_string()],
                ..decision.clone()
            },
        )
        .unwrap_err();
    assert_eq!(denied, OntologyEngineError::AuthorizationDenied);
    let too = engine
        .authorize_action_invocation(
            request,
            ActionPolicyDecision {
                autonomy_tier: AutonomyTier::T3Autonomous,
                ..decision
            },
        )
        .unwrap_err();
    assert_eq!(too, OntologyEngineError::AutonomyTierExceeded);
}

mod endpoint_reference_validation {
    use super::*;

    #[test]
    fn link_type_with_dangling_from_endpoint_rejected() {
        let mut engine = OntologyEngine::default();
        // Register only the "to" entity type; "from" is missing.
        engine
            .register_entity_type(
                EntityTypeDefinition::new(
                    "ten_clinic",
                    EntityTypeId::new("ety_appointment").unwrap(),
                    "Appointment",
                    vec![property("starts_at")],
                    1,
                )
                .unwrap(),
            )
            .unwrap();
        let link = LinkTypeDefinition::new(
            "ten_clinic",
            LinkTypeId::new("lty_missing_from").unwrap(),
            EntityTypeId::new("ety_patient").unwrap(), // not registered
            EntityTypeId::new("ety_appointment").unwrap(),
            LinkCardinality::OneToMany,
            false,
        )
        .unwrap();
        assert_eq!(
            engine.register_link_type(link),
            Err(OntologyEngineError::UnknownEntityTypeEndpoint)
        );
    }

    #[test]
    fn link_type_with_dangling_to_endpoint_rejected() {
        let mut engine = OntologyEngine::default();
        // Register only the "from" entity type; "to" is missing.
        engine.register_entity_type(patient_type()).unwrap();
        let link = LinkTypeDefinition::new(
            "ten_clinic",
            LinkTypeId::new("lty_missing_to").unwrap(),
            EntityTypeId::new("ety_patient").unwrap(),
            EntityTypeId::new("ety_appointment").unwrap(), // not registered
            LinkCardinality::OneToMany,
            false,
        )
        .unwrap();
        assert_eq!(
            engine.register_link_type(link),
            Err(OntologyEngineError::UnknownEntityTypeEndpoint)
        );
    }

    #[test]
    fn action_type_with_dangling_entity_type_rejected() {
        let mut engine = OntologyEngine::default();
        // No entity types registered at all.
        let action = ActionTypeDefinition::new(
            "ten_clinic",
            ActionTypeId::new("aty_discharge").unwrap(),
            EntityTypeId::new("ety_patient").unwrap(), // not registered
            "ontology.action.discharge",
            AutonomyTier::T1Assist,
            "EVT-DISCHARGE",
        )
        .unwrap();
        assert_eq!(
            engine.register_action_type(action),
            Err(OntologyEngineError::UnknownEntityTypeEndpoint)
        );
    }

    #[test]
    fn valid_link_and_action_type_registers_after_endpoints_present() {
        let mut engine = OntologyEngine::default();
        let patient = engine.register_entity_type(patient_type()).unwrap();
        let appointment = engine
            .register_entity_type(
                EntityTypeDefinition::new(
                    "ten_clinic",
                    EntityTypeId::new("ety_appointment").unwrap(),
                    "Appointment",
                    vec![property("starts_at")],
                    1,
                )
                .unwrap(),
            )
            .unwrap();
        // Both endpoints present: link type should register successfully.
        let link_id = engine
            .register_link_type(
                LinkTypeDefinition::new(
                    "ten_clinic",
                    LinkTypeId::new("lty_patient_appointment").unwrap(),
                    patient.clone(),
                    appointment,
                    LinkCardinality::OneToMany,
                    false,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(link_id.value, "lty_patient_appointment");
        // Endpoint present: action type should register successfully.
        let action_id = engine
            .register_action_type(
                ActionTypeDefinition::new(
                    "ten_clinic",
                    ActionTypeId::new("aty_discharge").unwrap(),
                    patient,
                    "ontology.action.discharge",
                    AutonomyTier::T1Assist,
                    "EVT-DISCHARGE",
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(action_id.value, "aty_discharge");
    }
}
