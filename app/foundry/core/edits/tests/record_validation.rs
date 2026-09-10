use foundry_edits::{
    ActionRecord, DenialRecord, EditSet, OntologyEdit, RecordError, WIRE_FORMAT_VERSION,
    WireDataClass, WireProperty, WireTier, WireValue,
};

fn edits() -> EditSet {
    let property = WireProperty::new(
        "name",
        WireTier::Scalar,
        WireDataClass::InternalOnly,
        WireValue::String("v".into()),
    )
    .unwrap();
    EditSet::new(vec![
        OntologyEdit::upsert_properties(vec![property]).unwrap(),
    ])
    .unwrap()
}

fn action(
    principal: &str,
    decision: &str,
    event: &str,
    key: &str,
) -> Result<ActionRecord, RecordError> {
    ActionRecord::new(
        principal,
        decision,
        event,
        key,
        1_700_000_000_000,
        vec![],
        edits(),
    )
}

#[test]
fn action_records_stamp_the_wire_version() {
    let record = action("prn_alice", "dec_1", "reading.calibrated", "idem_1").unwrap();
    assert_eq!(record.wire_format_version, WIRE_FORMAT_VERSION);
    assert_eq!(record.wire_format_version, 1);
}

#[test]
fn action_record_identity_fields_fail_closed() {
    for (case, expected_field) in [
        (action("", "dec_1", "evt", "idem_1"), "principal_id"),
        (action("prn_a", "  ", "evt", "idem_1"), "decision_id"),
        (action("prn_a", "dec_1", "", "idem_1"), "audit_event_type"),
        (action("prn_a", "dec_1", "evt", ""), "idempotency_key"),
    ] {
        assert_eq!(
            case,
            Err(RecordError::Empty {
                field: expected_field
            }),
        );
    }
    assert_eq!(
        action(" prn_a", "dec_1", "evt", "idem_1"),
        Err(RecordError::NotTrimmed {
            field: "principal_id"
        }),
    );
}

fn denial(
    gate: &str,
    cause: &str,
    principal: &str,
    decision: &str,
    action_id: &str,
    object_ref: &str,
) -> Result<DenialRecord, RecordError> {
    DenialRecord::new(
        gate,
        cause,
        principal,
        decision,
        action_id,
        object_ref,
        1_700_000_000_000,
    )
}

#[test]
fn denial_records_stamp_the_wire_version() {
    let record = denial(
        "authorization",
        "principal mismatch",
        "prn_alice",
        "dec_1",
        "aty_calibrate",
        "ent_reading_1",
    )
    .unwrap();
    assert_eq!(record.wire_format_version, WIRE_FORMAT_VERSION);
}

#[test]
fn denial_record_identity_fields_fail_closed() {
    for (case, expected_field) in [
        (denial("", "c", "p", "d", "a", "o"), "gate"),
        (denial("g", " ", "p", "d", "a", "o"), "cause"),
        (denial("g", "c", "", "d", "a", "o"), "principal_id"),
        (denial("g", "c", "p", "", "a", "o"), "decision_id"),
        (denial("g", "c", "p", "d", "", "o"), "action_id"),
        (denial("g", "c", "p", "d", "a", ""), "object_ref"),
    ] {
        assert_eq!(
            case,
            Err(RecordError::Empty {
                field: expected_field
            }),
        );
    }
    assert_eq!(
        denial("g ", "c", "p", "d", "a", "o"),
        Err(RecordError::NotTrimmed { field: "gate" }),
    );
}
