use super::*;

#[test]
fn rejects_schema_property_with_unsupported_composition_keyword() {
    for (keyword, unsupported_block) in [
        ("allOf", "          allOf:\n            - type: string\n"),
        ("anyOf", "          anyOf:\n            - type: string\n"),
        ("oneOf", "          oneOf:\n            - type: string\n"),
        ("not", "          not:\n            type: integer\n"),
        (
            "additionalProperties",
            "          additionalProperties: false\n",
        ),
    ] {
        let invalid = VALID.replacen(
            "        request_id:\n          type: string\n          x-oyatie-rust-type: String\n          x-oyatie-data-class: INTERNAL_ONLY\n",
            &format!(
                "        request_id:\n          type: string\n{unsupported_block}          x-oyatie-rust-type: String\n          x-oyatie-data-class: INTERNAL_ONLY\n"
            ),
            1,
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
            Err(OpenApiSourceError::SchemaTypeMismatch {
                schema_name: "CapabilityInvokeApiResponseMetadata".into(),
                contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
                mismatches: vec![format!(
                    "request_id: unsupported OpenAPI schema keyword {keyword}"
                )],
            }),
            "keyword {keyword} must fail closed"
        );
    }
}

#[test]
fn rejects_component_schema_with_unsupported_composition_keyword() {
    let invalid = VALID.replacen(
        "    CapabilityInvokeApiResponseMetadata:\n      type: object\n",
        "    CapabilityInvokeApiResponseMetadata:\n      type: object\n      additionalProperties: true\n",
        1,
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
        Err(OpenApiSourceError::SchemaTypeMismatch {
            schema_name: "CapabilityInvokeApiResponseMetadata".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            mismatches: vec![
                "CapabilityInvokeApiResponseMetadata: unsupported OpenAPI schema keyword additionalProperties"
                    .into(),
            ],
        })
    );
}
