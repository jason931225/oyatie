use super::*;

#[test]
fn rejects_schema_rust_type_annotations_that_drift_from_runtime_struct_fields() {
    let invalid = VALID.replacen("          x-oyatie-rust-type: Purpose\n", "", 1);
    assert_eq!(
        validate_openapi_schema_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                &invalid,
            )],
            schema_bindings(),
            schema_runtime_sources(),
        ),
        Err(OpenApiSourceError::SchemaTypeMismatch {
            schema_name: "CapabilityInvocationRequest".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            mismatches: vec!["purpose: missing x-oyatie-rust-type, expected Purpose".into()],
        })
    );
}

#[test]
fn rejects_schema_required_fields_that_drift_from_runtime_optionality() {
    let invalid = VALID.replace(
        "      required:\n        - tenant_id\n        - user_id\n        - capability_id\n        - evidence_event_hash\n",
        "      required:\n        - tenant_id\n        - user_id\n        - capability_id\n        - evidence_event_hash\n        - run_id\n",
    );
    assert_eq!(
        validate_openapi_schema_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                &invalid,
            )],
            schema_bindings(),
            schema_runtime_sources(),
        ),
        Err(OpenApiSourceError::SchemaRequiredMismatch {
            schema_name: "CapabilityInvocationReceipt".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            missing_required: vec![],
            extra_required: vec!["run_id".into()],
        })
    );
}
