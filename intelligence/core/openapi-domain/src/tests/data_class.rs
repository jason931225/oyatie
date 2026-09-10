use super::*;

#[test]
fn rejects_schema_property_without_data_class_annotation() {
    let invalid = VALID.replace(
        "        tenant_id:\n          type: string\n          x-oyatie-rust-type: String\n          x-oyatie-data-class: INTERNAL_ONLY\n",
        "        tenant_id:\n          type: string\n          x-oyatie-rust-type: String\n",
    );
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &invalid,
        )]),
        Err(OpenApiSourceError::MissingDataClassAnnotation {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            location: "schema CapabilityInvocationRequest.tenant_id".into(),
        })
    );
}

#[test]
fn rejects_parameter_with_invalid_data_class_annotation() {
    let invalid = VALID.replace(
        "          x-oyatie-data-class: INTERNAL_ONLY\n      requestBody:",
        "          x-oyatie-data-class: UNKNOWN\n      requestBody:",
    );
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &invalid,
        )]),
        Err(OpenApiSourceError::InvalidDataClassAnnotation {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            location: "parameter Idempotency-Key".into(),
            data_class: "UNKNOWN".into(),
        })
    );
}
