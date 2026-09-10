use super::*;

#[test]
fn rejects_schema_binding_with_unmodeled_serde_struct_clauses() {
    for (attribute, unsupported) in [
        ("#[serde(transparent)]", "transparent"),
        ("#[serde(untagged)]", "untagged"),
        ("#[serde(tag = \"kind\")]", "tag"),
        ("#[serde(deny_unknown_fields)]", "deny_unknown_fields"),
        (
            "#[serde(rename_all(serialize = \"camelCase\"))]",
            "rename_all",
        ),
    ] {
        let runtime = RUNTIME_API.replacen(
            "pub struct CapabilityInvokeApiResponseMetadata",
            &format!("{attribute}\npub struct CapabilityInvokeApiResponseMetadata"),
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
                    runtime_source("crates/intelligence-api/src/lib.rs", &runtime),
                ],
            ),
            Err(OpenApiSourceError::InvalidRuntimeStruct {
                schema_name: "CapabilityInvokeApiResponseMetadata".into(),
                path: "crates/intelligence-api/src/lib.rs".into(),
                rust_struct: "CapabilityInvokeApiResponseMetadata".into(),
                reason: format!(
                    "unsupported serde {unsupported} on struct CapabilityInvokeApiResponseMetadata"
                ),
            }),
            "attribute {attribute} must fail closed"
        );
    }
}

#[test]
fn rejects_schema_property_when_runtime_field_is_serde_skipped() {
    let skipped_runtime = RUNTIME_API.replacen(
        "    pub request_id: String, // data_class: INTERNAL_ONLY\n",
        "    #[serde(skip)]\n    pub request_id: String, // data_class: INTERNAL_ONLY\n",
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
                runtime_source("crates/intelligence-api/src/lib.rs", &skipped_runtime),
            ],
        ),
        Err(OpenApiSourceError::SchemaFieldMismatch {
            schema_name: "CapabilityInvokeApiResponseMetadata".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            missing_properties: vec![],
            extra_properties: vec!["request_id".into()],
        })
    );
}

#[test]
fn rejects_schema_binding_with_unsupported_serde_flatten_field() {
    let flattened_runtime = RUNTIME_API.replacen(
        "    pub metadata: CapabilityInvokeApiResponseMetadata, // data_class: INTERNAL_ONLY\n",
        "    #[serde(flatten)]\n    pub metadata: CapabilityInvokeApiResponseMetadata, // data_class: INTERNAL_ONLY\n",
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
                runtime_source("crates/intelligence-api/src/lib.rs", &flattened_runtime),
            ],
        ),
        Err(OpenApiSourceError::InvalidRuntimeStruct {
            schema_name: "CapabilityInvokeApiSuccessResponse".into(),
            path: "crates/intelligence-api/src/lib.rs".into(),
            rust_struct: "CapabilityInvokeApiSuccessResponse".into(),
            reason: "unsupported serde flatten on field metadata".into(),
        })
    );
}

#[test]
fn rejects_schema_required_when_runtime_field_is_conditionally_serialized() {
    let conditional_runtime = RUNTIME_API.replacen(
        "    pub request_id: String, // data_class: INTERNAL_ONLY\n",
        "    #[serde(skip_serializing_if = \"String::is_empty\")]\n    pub request_id: String, // data_class: INTERNAL_ONLY\n",
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
                runtime_source("crates/intelligence-api/src/lib.rs", &conditional_runtime),
            ],
        ),
        Err(OpenApiSourceError::SchemaRequiredMismatch {
            schema_name: "CapabilityInvokeApiResponseMetadata".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            missing_required: vec![],
            extra_required: vec!["request_id".into()],
        })
    );
}

#[test]
fn rejects_schema_required_when_runtime_field_has_serde_default() {
    let defaulted_runtime = RUNTIME_API.replacen(
        "    pub request_id: String, // data_class: INTERNAL_ONLY\n",
        "    #[serde(default)]\n    pub request_id: String, // data_class: INTERNAL_ONLY\n",
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
                runtime_source("crates/intelligence-api/src/lib.rs", &defaulted_runtime),
            ],
        ),
        Err(OpenApiSourceError::SchemaRequiredMismatch {
            schema_name: "CapabilityInvokeApiResponseMetadata".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            missing_required: vec![],
            extra_required: vec!["request_id".into()],
        })
    );
}

#[test]
fn rejects_schema_binding_with_unsupported_serde_alias_field() {
    let aliased_runtime = RUNTIME_API.replacen(
        "    pub request_id: String, // data_class: INTERNAL_ONLY\n",
        "    #[serde(alias = \"requestId\")]\n    pub request_id: String, // data_class: INTERNAL_ONLY\n",
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
                runtime_source("crates/intelligence-api/src/lib.rs", &aliased_runtime),
            ],
        ),
        Err(OpenApiSourceError::InvalidRuntimeStruct {
            schema_name: "CapabilityInvokeApiResponseMetadata".into(),
            path: "crates/intelligence-api/src/lib.rs".into(),
            rust_struct: "CapabilityInvokeApiResponseMetadata".into(),
            reason: "unsupported serde alias requestId on field request_id".into(),
        })
    );
}

#[test]
fn rejects_schema_binding_with_unmodeled_serde_field_clauses() {
    for (attribute, unsupported) in [
        (
            "#[serde(serialize_with = \"serialize_request_id\")]",
            "serialize_with",
        ),
        (
            "#[serde(deserialize_with = \"deserialize_request_id\")]",
            "deserialize_with",
        ),
        ("#[serde(with = \"request_id_codec\")]", "with"),
        ("#[serde(skip_deserializing)]", "skip_deserializing"),
        ("#[serde(rename(serialize = \"requestId\"))]", "rename"),
    ] {
        let serialized_runtime = RUNTIME_API.replacen(
            "    pub request_id: String, // data_class: INTERNAL_ONLY\n",
            &format!("    {attribute}\n    pub request_id: String, // data_class: INTERNAL_ONLY\n"),
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
                    runtime_source("crates/intelligence-api/src/lib.rs", &serialized_runtime),
                ],
            ),
            Err(OpenApiSourceError::InvalidRuntimeStruct {
                schema_name: "CapabilityInvokeApiResponseMetadata".into(),
                path: "crates/intelligence-api/src/lib.rs".into(),
                rust_struct: "CapabilityInvokeApiResponseMetadata".into(),
                reason: format!("unsupported serde {unsupported} on field request_id"),
            }),
            "attribute {attribute} must fail closed"
        );
    }
}
