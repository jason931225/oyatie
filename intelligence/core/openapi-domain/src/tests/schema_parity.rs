use super::*;

#[test]
fn accepts_schema_parity_when_openapi_fields_match_runtime_structs() {
    assert_eq!(
        validate_openapi_schema_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            schema_bindings(),
            schema_runtime_sources(),
        ),
        Ok(OpenApiSchemaParityReport {
            schemas_checked: 7,
            bindings_checked: 7,
            sources_checked: 2,
            fields_checked: 29,
            types_checked: 29,
        })
    );
}

#[test]
fn accepts_schema_parity_for_vec_string_scalar_items() {
    let contract = r#"openapi: 3.2.0
info:
  title: Oyatie Tags API
  version: 1.0.0
paths:
  /v1/tags:
    get:
      operationId: listTags
      security:
        - bearerAuth: []
      responses:
        '200':
          description: Tags listed.
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/TagResponse'
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
  schemas:
    TagResponse:
      type: object
      required:
        - tags
      properties:
        tags:
          type: array
          items:
            type: string
          x-oyatie-rust-type: Vec<String>
          x-oyatie-data-class: INTERNAL_ONLY
"#;
    let runtime = r#"
pub struct TagResponse {
    pub tags: Vec<String>, // data_class: INTERNAL_ONLY
}
"#;

    assert_eq!(
        validate_openapi_schema_parity(
            [document("contracts/openapi/foundry/tags-v1.yaml", contract)],
            [OpenApiSchemaBinding {
                schema_name: "TagResponse".into(),
                contract_path: "contracts/openapi/foundry/tags-v1.yaml".into(),
                runtime_crate: "intelligence-api".into(),
                source_path: "crates/intelligence-api/src/lib.rs".into(),
                rust_struct: "TagResponse".into(),
            }],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                runtime
            )],
        ),
        Ok(OpenApiSchemaParityReport {
            schemas_checked: 1,
            bindings_checked: 1,
            sources_checked: 1,
            fields_checked: 1,
            types_checked: 1,
        })
    );
}

#[test]
fn rejects_component_schema_without_runtime_shape_binding() {
    assert_eq!(
        validate_openapi_schema_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [],
            [],
        ),
        Err(OpenApiSourceError::MissingSchemaBinding {
            schema_name: "CapabilityInvocationReceipt".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
        })
    );
}

#[test]
fn rejects_schema_properties_that_drift_from_runtime_struct_fields() {
    let invalid = VALID.replacen(
        "        tenant_id:\n          type: string\n          x-oyatie-rust-type: String\n          x-oyatie-data-class: INTERNAL_ONLY\n        user_id:\n          type: string\n          x-oyatie-rust-type: String\n          x-oyatie-data-class: INTERNAL_ONLY\n        capability_id:",
        "        tenant_id:\n          type: string\n          x-oyatie-rust-type: String\n          x-oyatie-data-class: INTERNAL_ONLY\n        capability_id:",
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
        Err(OpenApiSourceError::SchemaFieldMismatch {
            schema_name: "CapabilityInvocationRequest".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            missing_properties: vec!["user_id".into()],
            extra_properties: vec![],
        })
    );
}

#[test]
fn rejects_schema_binding_when_runtime_struct_is_not_public_api_type() {
    let private_success_response = RUNTIME_API.replacen(
        "pub struct CapabilityInvokeApiSuccessResponse",
        "struct CapabilityInvokeApiSuccessResponse",
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
                runtime_source(
                    "crates/intelligence-api/src/lib.rs",
                    &private_success_response,
                ),
            ],
        ),
        Err(OpenApiSourceError::MissingRuntimeStruct {
            schema_name: "CapabilityInvokeApiSuccessResponse".into(),
            path: "crates/intelligence-api/src/lib.rs".into(),
            rust_struct: "CapabilityInvokeApiSuccessResponse".into(),
        })
    );
}

#[test]
fn rejects_schema_types_that_drift_from_runtime_struct_fields() {
    let invalid = VALID.replacen(
        "        projected_cost_micros:\n          type: integer\n          format: uint64\n          minimum: 0\n          x-oyatie-rust-type: u64\n          x-oyatie-data-class: INTERNAL_ONLY\n",
        "        projected_cost_micros:\n          type: integer\n          format: int64\n          minimum: 0\n          x-oyatie-rust-type: u64\n          x-oyatie-data-class: INTERNAL_ONLY\n",
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
            schema_name: "CapabilityInvocationRequest".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            mismatches: vec![
                "projected_cost_micros: expected type integer format uint64 for Rust u64, found type integer format int64"
                    .into()
            ],
        })
    );
}

#[test]
fn rejects_schema_array_items_with_only_nested_spoofed_ref() {
    let invalid = VALID.replace(
        "        details:\n          type: array\n          items:\n            $ref: '#/components/schemas/CapabilityInvokeApiErrorDetail'\n          x-oyatie-rust-type: Vec<CapabilityInvokeApiErrorDetail>\n          x-oyatie-data-class: INTERNAL_ONLY\n",
        "        details:\n          type: array\n          items:\n            type: object\n            properties:\n              spoof:\n                $ref: '#/components/schemas/CapabilityInvokeApiErrorDetail'\n                x-oyatie-data-class: INTERNAL_ONLY\n          x-oyatie-rust-type: Vec<CapabilityInvokeApiErrorDetail>\n          x-oyatie-data-class: INTERNAL_ONLY\n",
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
            schema_name: "CapabilityInvokeApiErrorBody".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            mismatches: vec![
                "details: missing items $ref, expected CapabilityInvokeApiErrorDetail for Rust Vec<CapabilityInvokeApiErrorDetail>"
                    .into()
            ],
        })
    );
}

#[test]
fn rejects_schema_ref_property_with_conflicting_scalar_type() {
    let invalid = VALID.replace(
        "        data:\n          $ref: '#/components/schemas/CapabilityInvocationReceipt'\n          x-oyatie-rust-type: CapabilityInvocationReceipt\n          x-oyatie-data-class: INTERNAL_ONLY\n",
        "        data:\n          type: string\n          $ref: '#/components/schemas/CapabilityInvocationReceipt'\n          x-oyatie-rust-type: CapabilityInvocationReceipt\n          x-oyatie-data-class: INTERNAL_ONLY\n",
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
            schema_name: "CapabilityInvokeApiSuccessResponse".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            mismatches: vec![
                "data: $ref property for Rust CapabilityInvocationReceipt must not declare type string"
                    .into()
            ],
        })
    );
}

#[test]
fn rejects_schema_nullable_property_when_runtime_field_is_not_option() {
    let invalid = VALID.replacen(
        "        request_id:\n          type: string\n          x-oyatie-rust-type: String\n          x-oyatie-data-class: INTERNAL_ONLY\n",
        "        request_id:\n          type: string\n          nullable: true\n          x-oyatie-rust-type: String\n          x-oyatie-data-class: INTERNAL_ONLY\n",
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
                "request_id: OpenAPI nullable true requires Rust Option<...>, found String".into(),
            ],
        })
    );
}
