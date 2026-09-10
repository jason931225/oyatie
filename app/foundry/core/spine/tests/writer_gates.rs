//! The writer's deny-by-default gates and retry contract: an unauthorized
//! caller appends NOTHING, refused submissions never reach the log, a
//! byte-identical retry deduplicates to the original outcome, and a
//! fresh-decision retry conflicts loudly.

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    ActionInvocationRequest, ActionPolicyDecision, ActionTypeDefinition, ActionTypeId,
    AutonomyTier, EntityTypeDefinition, EntityTypeId, EntityTypePropertyDefinition, OntologyEngine,
    PropertyTier, PropertyValue,
};
use foundry_edits::{EditSet, OntologyEdit, WireDataClass, WireProperty, WireTier, WireValue};
use foundry_records_draft::{ActionEnvelope, Receipt, RecordsLog, RecordsLogError, SealedEnvelope};
use foundry_spine::{
    ActionSubmission, ApplyOutcome, ProjectionState, RefusalGate, WriteError, submit,
};

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn registry() -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(
            EntityTypeDefinition::new(
                "ten_test",
                EntityTypeId::new("ety_reading").unwrap(),
                "Reading",
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

#[derive(Default)]
struct MemoryLog {
    entries: Vec<SealedEnvelope>,
}

impl RecordsLog for MemoryLog {
    // Dedup never fires in these tests; the retry contract has its own
    // file and fixture.
    fn append(&mut self, envelope: ActionEnvelope) -> Result<Receipt, RecordsLogError> {
        let ordinal = self
            .entries
            .iter()
            .filter(|sealed| sealed.envelope.tenant_id == envelope.tenant_id)
            .count() as u64
            + 1;
        let object_sequence = self
            .entries
            .iter()
            .filter(|sealed| {
                sealed.envelope.tenant_id == envelope.tenant_id
                    && sealed.envelope.object_ref == envelope.object_ref
            })
            .count() as u64
            + 1;
        let receipt = Receipt {
            ordinal,
            object_sequence,
            deduplicated: false,
        };
        self.entries.push(SealedEnvelope {
            envelope,
            receipt: receipt.clone(),
        });
        Ok(receipt)
    }

    fn replay(
        &self,
        tenant_id: &str,
        from_ordinal: u64,
    ) -> Result<Vec<SealedEnvelope>, RecordsLogError> {
        Ok(self
            .entries
            .iter()
            .filter(|sealed| {
                sealed.envelope.tenant_id == tenant_id && sealed.receipt.ordinal >= from_ordinal
            })
            .cloned()
            .collect())
    }

    fn head(&self, tenant_id: &str) -> Result<u64, RecordsLogError> {
        Ok(self
            .entries
            .iter()
            .filter(|sealed| sealed.envelope.tenant_id == tenant_id)
            .count() as u64)
    }
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

fn decision() -> ActionPolicyDecision {
    ActionPolicyDecision {
        decision_id: "dec_1".into(),
        tenant_id: "ten_test".into(),
        principal_id: "prn_alice".into(),
        allowed_surfaces: vec!["ops-console".into()],
        autonomy_tier: AutonomyTier::T1Assist,
    }
}

fn request(idempotency_key: &str) -> ActionInvocationRequest {
    ActionInvocationRequest {
        tenant_id: "ten_test".into(),
        principal_id: "prn_alice".into(),
        action_id: ActionTypeId::new("aty_calibrate").unwrap(),
        entity_id: "ent_r1".into(),
        idempotency_key: idempotency_key.into(),
        requested_at_epoch_seconds: 1_700_000_000,
    }
}

fn create_submission(idempotency_key: &str) -> ActionSubmission {
    ActionSubmission {
        request: request(idempotency_key),
        decision: decision(),
        parameters: vec![],
        edits: EditSet::new(vec![
            OntologyEdit::create_object("ety_reading", vec![name_property("Ada")]).unwrap(),
        ])
        .unwrap(),
    }
}

#[test]
fn authorized_write_applies_end_to_end() {
    let registry = registry();
    let mut log = MemoryLog::default();
    let mut denials = MemoryLog::default();
    let mut projection = ProjectionState::new("ten_test", &registry);
    let outcome = submit(
        create_submission("idem_1"),
        &mut log,
        &mut denials,
        &mut projection,
    )
    .unwrap();
    let ApplyOutcome::Applied { receipt } = outcome else {
        panic!("expected an applied outcome, got {outcome:?}");
    };
    assert_eq!(receipt.ordinal, 1);
    assert!(!receipt.deduplicated);
    let entity = projection
        .objects
        .get("ten_test", "ent_r1")
        .expect("projected");
    assert_eq!(
        entity.properties["name"].value.value,
        PropertyValue::String("Ada".into())
    );
    // The envelope carries the CURRENT registered revision, writer-stamped.
    assert_eq!(projection.bindings["ent_r1"].schema_revision, 1);
    assert_eq!(log.head("ten_test").unwrap(), 1);
}

#[test]
fn unauthorized_caller_appends_nothing() {
    let registry = registry();
    let mut log = MemoryLog::default();
    let mut denials = MemoryLog::default();
    let mut projection = ProjectionState::new("ten_test", &registry);
    let mut submission = create_submission("idem_1");
    submission.decision.allowed_surfaces = vec!["someone-elses-console".into()];
    let refused = submit(submission, &mut log, &mut denials, &mut projection).unwrap_err();
    let WriteError::Refused(refused) = refused else {
        panic!("expected a gate refusal, got {refused:?}");
    };
    assert_eq!(refused.gate, RefusalGate::Authorization);
    assert_eq!(log.head("ten_test").unwrap(), 0, "nothing appended");
    assert!(projection.objects.is_empty());
}

#[test]
fn undeclared_parameter_refuses_before_the_log() {
    let registry = registry();
    let mut log = MemoryLog::default();
    let mut denials = MemoryLog::default();
    let mut projection = ProjectionState::new("ten_test", &registry);
    let mut submission = create_submission("idem_1");
    submission.parameters = vec![name_property("Ada")];
    let WriteError::Refused(refused) =
        submit(submission, &mut log, &mut denials, &mut projection).unwrap_err()
    else {
        panic!("expected a gate refusal");
    };
    assert_eq!(refused.gate, RefusalGate::Parameters);
    assert_eq!(log.head("ten_test").unwrap(), 0);
}

#[test]
fn admission_dry_run_refuses_bad_edits_before_the_log() {
    let registry = registry();
    let mut log = MemoryLog::default();
    let mut denials = MemoryLog::default();
    let mut projection = ProjectionState::new("ten_test", &registry);
    let rogue = WireProperty::new(
        "rogue",
        WireTier::Scalar,
        WireDataClass::InternalOnly,
        WireValue::String("x".into()),
    )
    .unwrap();
    let mut submission = create_submission("idem_1");
    submission.edits = EditSet::new(vec![
        OntologyEdit::create_object("ety_reading", vec![name_property("Ada"), rogue]).unwrap(),
    ])
    .unwrap();
    let WriteError::Refused(refused) =
        submit(submission, &mut log, &mut denials, &mut projection).unwrap_err()
    else {
        panic!("expected a gate refusal");
    };
    assert_eq!(refused.gate, RefusalGate::Admission);
    assert_eq!(log.head("ten_test").unwrap(), 0);
}

#[test]
fn an_edit_for_a_foreign_entity_type_is_refused_at_admission() {
    let mut registry = registry();
    registry
        .register_entity_type(
            EntityTypeDefinition::new(
                "ten_test",
                EntityTypeId::new("ety_other").unwrap(),
                "Other",
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
    let mut log = MemoryLog::default();
    let mut denials = MemoryLog::default();
    let mut projection = ProjectionState::new("ten_test", &registry);
    let mut submission = create_submission("idem_1");
    submission.edits = EditSet::new(vec![
        OntologyEdit::create_object("ety_other", vec![name_property("Ada")]).unwrap(),
    ])
    .unwrap();
    let WriteError::Refused(refused) =
        submit(submission, &mut log, &mut denials, &mut projection).unwrap_err()
    else {
        panic!("expected a gate refusal");
    };
    assert_eq!(refused.gate, RefusalGate::Admission);
    assert_eq!(log.head("ten_test").unwrap(), 0);
}
