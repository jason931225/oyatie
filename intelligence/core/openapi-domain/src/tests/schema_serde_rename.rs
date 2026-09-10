use super::*;

#[test]
fn rejects_schema_property_when_runtime_field_has_serde_rename_drift() {
    let renamed_runtime = RUNTIME_API.replacen(
        "    pub data: CapabilityInvocationReceipt, // data_class: INTERNAL_ONLY\n",
        "    #[serde(rename = \"payload\")]\n    pub data: CapabilityInvocationReceipt, // data_class: INTERNAL_ONLY\n",
        1,
    );

    assert_eq!(
        validate_openapi_schema_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            schema_bindings(),
            [
                runtime_source("crates/foundation-app/src/lib.rs", RUNTIME_STRUCTS),
                runtime_source("crates/intelligence-api/src/lib.rs", &renamed_runtime),
            ],
        ),
        Err(OpenApiSourceError::SchemaFieldMismatch {
            schema_name: "CapabilityInvokeApiSuccessResponse".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            missing_properties: vec!["payload".into()],
            extra_properties: vec!["data".into()],
        })
    );
}

#[test]
fn rejects_schema_property_when_runtime_struct_has_serde_rename_all_drift() {
    let renamed_runtime = RUNTIME_API.replacen(
        "pub struct CapabilityInvokeApiResponseMetadata",
        "#[serde(rename_all = \"camelCase\")]\npub struct CapabilityInvokeApiResponseMetadata",
        1,
    );

    assert_eq!(
        validate_openapi_schema_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            schema_bindings(),
            [
                runtime_source("crates/foundation-app/src/lib.rs", RUNTIME_STRUCTS),
                runtime_source("crates/intelligence-api/src/lib.rs", &renamed_runtime),
            ],
        ),
        Err(OpenApiSourceError::SchemaFieldMismatch {
            schema_name: "CapabilityInvokeApiResponseMetadata".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            missing_properties: vec!["requestId".into()],
            extra_properties: vec!["request_id".into()],
        })
    );
}

#[test]
fn rejects_schema_binding_with_unsupported_serde_rename_all_rule() {
    let renamed_runtime = RUNTIME_API.replacen(
        "pub struct CapabilityInvokeApiResponseMetadata",
        "#[serde(rename_all = \"Train-Case\")]\npub struct CapabilityInvokeApiResponseMetadata",
        1,
    );

    assert_eq!(
        validate_openapi_schema_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            schema_bindings(),
            [
                runtime_source("crates/foundation-app/src/lib.rs", RUNTIME_STRUCTS),
                runtime_source("crates/intelligence-api/src/lib.rs", &renamed_runtime),
            ],
        ),
        Err(OpenApiSourceError::InvalidRuntimeStruct {
            schema_name: "CapabilityInvokeApiResponseMetadata".into(),
            path: "crates/intelligence-api/src/lib.rs".into(),
            rust_struct: "CapabilityInvokeApiResponseMetadata".into(),
            reason: "unsupported serde rename_all rule Train-Case".into(),
        })
    );
}
