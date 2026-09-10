//! Derivation of port-shaped audit events from the spine's facts.

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    ActionInvocationRequest, ActionPolicyDecision, ActionTypeDefinition, ActionTypeId,
    AutonomyTier, EntityTypeDefinition, EntityTypeId, EntityTypePropertyDefinition, OntologyEngine,
    PropertyTier,
};
use foundry_audit_draft::{AuditDisposition, AuditSink, FoundryAuditEvent, MemoryAuditSink};
use foundry_edits::{
    ActionRecord, EditSet, OntologyEdit, WireDataClass, WireProperty, WireTier, WireValue,
    encode_action_record,
};
use foundry_records_draft::{ActionEnvelope, Receipt, RecordsLog, RecordsLogError, SealedEnvelope};
use foundry_spine::{
    ActionSubmission, DENIED_AUDIT_EVENT_TYPE, POISONED_AUDIT_EVENT_TYPE, ProjectionState,
    Underivable, UnderivableReason, audit_view, derive_action_events, derive_denial_events,
    fold_from_scratch, submit,
};

const TS_MS: u64 = 1_700_000_000_000;

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn registry() -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    let property =
        EntityTypePropertyDefinition::new("name", PropertyTier::Scalar, internal(), true).unwrap();
    engine
        .register_entity_type(
            EntityTypeDefinition::new(
                "ten_test",
                EntityTypeId::new("ety_reading").unwrap(),
                "Reading",
                vec![property],
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
    fn append(&mut self, envelope: ActionEnvelope) -> Result<Receipt, RecordsLogError> {
        let receipt = Receipt {
            ordinal: self.entries.len() as u64 + 1,
            object_sequence: 1,
            deduplicated: false,
        };
        self.entries.push(SealedEnvelope {
            envelope,
            receipt: receipt.clone(),
        });
        Ok(receipt)
    }

    fn replay(&self, _tenant_id: &str, _from: u64) -> Result<Vec<SealedEnvelope>, RecordsLogError> {
        Ok(self.entries.clone())
    }

    fn head(&self, _tenant_id: &str) -> Result<u64, RecordsLogError> {
        Ok(self.entries.len() as u64)
    }
}

fn append_raw(log: &mut MemoryLog, obj: &str, idem: &str, payload: Vec<u8>) {
    let envelope =
        ActionEnvelope::new("ten_test", obj, "aty_calibrate", idem, 1, payload, TS_MS).unwrap();
    log.append(envelope).unwrap();
}

fn create_edit() -> EditSet {
    let property = WireProperty::new(
        "name",
        WireTier::Scalar,
        WireDataClass::InternalOnly,
        WireValue::String("Ada".into()),
    )
    .unwrap();
    EditSet::new(vec![
        OntologyEdit::create_object("ety_reading", vec![property]).unwrap(),
    ])
    .unwrap()
}

fn submission(principal: &str, decision: &str, entity: &str, idem: &str) -> ActionSubmission {
    ActionSubmission {
        request: ActionInvocationRequest {
            tenant_id: "ten_test".into(),
            principal_id: principal.into(),
            action_id: ActionTypeId::new("aty_calibrate").unwrap(),
            entity_id: entity.into(),
            idempotency_key: idem.into(),
            requested_at_epoch_seconds: 1_700_000_000,
        },
        decision: ActionPolicyDecision {
            decision_id: decision.into(),
            tenant_id: "ten_test".into(),
            principal_id: principal.into(),
            allowed_surfaces: vec!["ops-console".into()],
            autonomy_tier: AutonomyTier::T1Assist,
        },
        parameters: vec![],
        edits: create_edit(),
    }
}

/// Two applied entries, one decodable forgery (receipt mismatch), one
/// undecodable payload — folded from scratch like replay would.
fn seeded() -> (Vec<SealedEnvelope>, ProjectionState) {
    let registry = registry();
    let mut log = MemoryLog::default();
    let mut denials = MemoryLog::default();
    let mut projection = ProjectionState::new("ten_test", &registry);
    for (decision, entity, idem) in [
        ("dec_a1", "ent_r1", "idem_1"),
        ("dec_a2", "ent_r2", "idem_2"),
    ] {
        let accepted = submission("prn_alice", decision, entity, idem);
        submit(accepted, &mut log, &mut denials, &mut projection).unwrap();
    }
    let forged = ActionRecord::new(
        "prn_forger",
        "dec_forged",
        "reading.calibrated",
        "idem_other",
        TS_MS,
        vec![],
        create_edit(),
    )
    .unwrap();
    append_raw(&mut log, "ent_r3", "idem_3", encode_action_record(&forged));
    append_raw(&mut log, "ent_r4", "idem_4", vec![0xFF]);
    let entries = log.replay("ten_test", 0).unwrap();
    let state = fold_from_scratch("ten_test", &registry, &entries);
    (entries, state)
}

fn ordinal_of(event: &FoundryAuditEvent) -> u64 {
    match &event.disposition {
        AuditDisposition::Applied { ordinal } | AuditDisposition::Poisoned { ordinal, .. } => {
            *ordinal
        }
        AuditDisposition::Denied { .. } => panic!("action-log derivation never yields Denied"),
    }
}

fn event(kind: &str, who: &str, dec: &str, obj: &str, d: AuditDisposition) -> FoundryAuditEvent {
    FoundryAuditEvent::new("ten_test", kind, who, dec, obj, d, TS_MS).unwrap()
}

#[test]
fn applied_entries_become_applied_events_with_attribution() {
    let (entries, state) = seeded();
    let derived = derive_action_events(&state, &entries);
    let expected = event(
        "reading.calibrated",
        "prn_alice",
        "dec_a1",
        "ent_r1",
        AuditDisposition::Applied { ordinal: 1 },
    );
    assert_eq!(derived.events[0], expected);
    assert_eq!(ordinal_of(&derived.events[1]), 2);
}

#[test]
fn a_decodable_poison_keeps_attribution_under_its_own_event_type() {
    let (entries, state) = seeded();
    let derived = derive_action_events(&state, &entries);
    let expected = event(
        POISONED_AUDIT_EVENT_TYPE,
        "prn_forger",
        "dec_forged",
        "ent_r3",
        AuditDisposition::Poisoned {
            ordinal: 3,
            reason: "receipt_mismatch".into(),
        },
    );
    assert_eq!(derived.events[2], expected);
}

#[test]
fn an_undecodable_entry_is_reported_underivable_never_dropped() {
    let (entries, state) = seeded();
    let derived = derive_action_events(&state, &entries);
    assert!(derived.events.iter().all(|event| ordinal_of(event) != 4));
    let expected = Underivable {
        ordinal: 4,
        reason: UnderivableReason::PayloadUndecodable,
    };
    assert_eq!(derived.underivable, vec![expected]);
}

#[test]
fn every_consumed_entry_lands_in_exactly_one_channel() {
    let (entries, state) = seeded();
    let derived = derive_action_events(&state, &entries);
    let events: Vec<u64> = derived.events.iter().map(ordinal_of).collect();
    let underivable: Vec<u64> = derived.underivable.iter().map(|row| row.ordinal).collect();
    assert!(events.iter().all(|ordinal| !underivable.contains(ordinal)));
    assert_eq!(
        events.len() + underivable.len(),
        state.applied_ordinal as usize
    );
}

#[test]
fn every_consumed_ordinal_is_conserved_against_the_audit_view() {
    let (entries, state) = seeded();
    let derived = derive_action_events(&state, &entries);
    let mut derived_ordinals: Vec<u64> = derived.events.iter().map(ordinal_of).collect();
    derived_ordinals.extend(derived.underivable.iter().map(|row| row.ordinal));
    derived_ordinals.sort_unstable();
    let view_ordinals: Vec<u64> = audit_view(&state, &entries)
        .iter()
        .map(|entry| entry.ordinal)
        .collect();
    assert_eq!(derived_ordinals, view_ordinals);
}

#[test]
fn denials_become_denied_events_from_the_trail() {
    let registry = registry();
    let mut log = MemoryLog::default();
    let mut denials = MemoryLog::default();
    let mut projection = ProjectionState::new("ten_test", &registry);
    let mut unauthorized = submission("prn_mallory", "dec_m1", "ent_r1", "idem_m1");
    unauthorized.decision.allowed_surfaces = vec!["someone-elses-console".into()];
    let _ = submit(unauthorized, &mut log, &mut denials, &mut projection);

    let derived = derive_denial_events(&denials.replay("ten_test", 0).unwrap());
    assert!(derived.underivable.is_empty());
    let expected = event(
        DENIED_AUDIT_EVENT_TYPE,
        "prn_mallory",
        "dec_m1",
        "ent_r1",
        AuditDisposition::Denied {
            gate: "authorization".into(),
        },
    );
    assert_eq!(derived.events, vec![expected]);
}

#[test]
fn an_undecodable_denial_payload_is_reported_underivable() {
    let mut denials = MemoryLog::default();
    append_raw(&mut denials, "ent_r1", "idem_x", vec![0xFF]);
    let derived = derive_denial_events(&denials.replay("ten_test", 0).unwrap());
    assert!(derived.events.is_empty());
    assert_eq!(
        derived.underivable[0].reason,
        UnderivableReason::PayloadUndecodable
    );
}

#[test]
fn re_derivation_re_emitted_through_a_sink_stays_idempotent() {
    let (entries, state) = seeded();
    let mut sink = MemoryAuditSink::default();
    for _ in 0..2 {
        for event in derive_action_events(&state, &entries).events {
            sink.emit(event).unwrap();
        }
    }
    assert_eq!(
        sink.events().len(),
        derive_action_events(&state, &entries).events.len(),
        "derivation is deterministic and the sink dedups by identity",
    );
}
