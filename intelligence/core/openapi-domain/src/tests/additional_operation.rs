use super::*;

#[test]
fn accepts_path_item_parameters_for_template_and_mutating_ingress_checks() {
    let inherited_parameters = r#"openapi: 3.2.0
info:
  title: Oyatie Copy API
  version: 1.0.0
paths:
  /v1/resources/{resource_id}:
    parameters:
      - name: resource_id
        in: path
        required: true
        schema:
          type: string
        x-oyatie-data-class: INTERNAL_ONLY
      - name: X-Request-Id
        in: header
        required: true
        schema:
          type: string
          minLength: 1
        x-oyatie-rust-type: String
        x-oyatie-data-class: INTERNAL_ONLY
      - name: X-Tenant-Id
        in: header
        required: true
        schema:
          type: string
          minLength: 1
        x-oyatie-rust-type: String
        x-oyatie-data-class: INTERNAL_ONLY
      - name: Idempotency-Key
        in: header
        required: true
        schema:
          type: string
          minLength: 1
        x-oyatie-rust-type: String
        x-oyatie-data-class: INTERNAL_ONLY
    additionalOperations:
      COPY:
        operationId: copyResource
        security:
          - bearerAuth: []
        responses:
          '200':
            description: Copy completed.
            content:
              application/json:
                schema:
                  $ref: '#/components/schemas/CopyResponse'
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
  schemas:
    CopyResponse:
      type: object
      required:
        - data
      properties:
        data:
          type: string
          x-oyatie-rust-type: String
          x-oyatie-data-class: INTERNAL_ONLY
"#;

    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/copy-v1.yaml",
            inherited_parameters,
        )]),
        Ok(OpenApiSourceReport {
            documents_checked: 1,
            operations_checked: 1,
            data_class_annotations_checked: 5,
        })
    );
}

#[test]
fn rejects_openapi_32_additional_operation_without_mutating_ingress_controls() {
    let missing_security =
        copy_operation_document().replace("        security:\n          - bearerAuth: []\n", "");
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/copy-v1.yaml",
            &missing_security,
        )]),
        Err(OpenApiSourceError::MissingOperationSecurity {
            path: "contracts/openapi/foundry/copy-v1.yaml".into(),
            api_path: "/v1/resources/{resource_id}".into(),
            method: "COPY".into(),
            scheme: "bearerAuth".into(),
        })
    );

    let missing_idempotency = copy_operation_document().replace(
        "          - name: Idempotency-Key\n            in: header\n            required: true\n            schema:\n              type: string\n              minLength: 1\n            x-oyatie-rust-type: String\n            x-oyatie-data-class: INTERNAL_ONLY\n",
        "",
    );
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/copy-v1.yaml",
            &missing_idempotency,
        )]),
        Err(OpenApiSourceError::MissingRequiredHeaderParameter {
            path: "contracts/openapi/foundry/copy-v1.yaml".into(),
            api_path: "/v1/resources/{resource_id}".into(),
            method: "COPY".into(),
            header: "Idempotency-Key".into(),
        })
    );
}

#[test]
fn rejects_openapi_32_additional_operations_fixed_method_collisions() {
    for method in ["POST", "QUERY"] {
        let invalid = copy_operation_document().replace("      COPY:", &format!("      {method}:"));
        assert_eq!(
            validate_openapi_documents([document(
                "contracts/openapi/foundry/copy-v1.yaml",
                &invalid,
            )]),
            Err(
                OpenApiSourceError::AdditionalOperationFixedMethodCollision {
                    path: "contracts/openapi/foundry/copy-v1.yaml".into(),
                    api_path: "/v1/resources/{resource_id}".into(),
                    method: method.into(),
                }
            )
        );
    }
}

#[test]
fn rejects_openapi_32_additional_operation_missing_operation_id_or_responses() {
    let missing_operation_id =
        copy_operation_document().replace("        operationId: copyResource\n", "");
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/copy-v1.yaml",
            &missing_operation_id,
        )]),
        Err(OpenApiSourceError::MissingOperationId {
            path: "contracts/openapi/foundry/copy-v1.yaml".into(),
            api_path: "/v1/resources/{resource_id}".into(),
            method: "COPY".into(),
        })
    );

    let missing_responses = copy_operation_document().replace(
        "        responses:\n          '200':\n            description: Copy completed.\n            content:\n              application/json:\n                schema:\n                  $ref: '#/components/schemas/CopyResponse'\n",
        "        responses:\n",
    );
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/copy-v1.yaml",
            &missing_responses,
        )]),
        Err(OpenApiSourceError::MissingResponses {
            path: "contracts/openapi/foundry/copy-v1.yaml".into(),
            api_path: "/v1/resources/{resource_id}".into(),
            method: "COPY".into(),
        })
    );
}

#[test]
fn runtime_parity_collects_query_and_additional_operations_as_bindable_operations() {
    let query = query_operation_document();
    assert_eq!(
        validate_openapi_runtime_parity(
            [document("contracts/openapi/foundry/query-v1.yaml", &query)],
            [],
            [],
            [],
        ),
        Err(OpenApiSourceError::MissingRuntimeBinding {
            operation_id: "queryCapability".into(),
            contract_path: "contracts/openapi/foundry/query-v1.yaml".into(),
        })
    );

    let copy = copy_operation_document();
    assert_eq!(
        validate_openapi_runtime_parity(
            [document("contracts/openapi/foundry/copy-v1.yaml", &copy)],
            [],
            [],
            [],
        ),
        Err(OpenApiSourceError::MissingRuntimeBinding {
            operation_id: "copyResource".into(),
            contract_path: "contracts/openapi/foundry/copy-v1.yaml".into(),
        })
    );
}

#[test]
fn additional_operation_runtime_parity_keeps_response_key_and_schema_guards() {
    let with_range = copy_operation_document().replace("          '200':", "          '2XX':");
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/copy-v1.yaml",
                &with_range
            )],
            [copy_runtime_binding("CopyResponse")],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                COPY_RUNTIME_API
            )],
            [runtime_source(
                "crates/intelligence-api/tests/copy_api.rs",
                COPY_RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::NonExplicitRuntimeResponseKey {
            operation_id: "copyResource".into(),
            contract_path: "contracts/openapi/foundry/copy-v1.yaml".into(),
            response_key: "2XX".into(),
        })
    );

    let missing_schema = copy_operation_document().replace(
        "            content:\n              application/json:\n                schema:\n                  $ref: '#/components/schemas/CopyResponse'\n",
        "",
    );
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/copy-v1.yaml",
                &missing_schema
            )],
            [copy_runtime_binding("CopyResponse")],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                COPY_RUNTIME_API
            )],
            [runtime_source(
                "crates/intelligence-api/tests/copy_api.rs",
                COPY_RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::MissingRuntimeResponseSchema {
            operation_id: "copyResource".into(),
            contract_path: "contracts/openapi/foundry/copy-v1.yaml".into(),
            status: "200".into(),
        })
    );

    let copy = copy_operation_document();
    assert_eq!(
        validate_openapi_runtime_parity(
            [document("contracts/openapi/foundry/copy-v1.yaml", &copy)],
            [copy_runtime_binding("OtherResponse")],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                COPY_RUNTIME_API
            )],
            [runtime_source(
                "crates/intelligence-api/tests/copy_api.rs",
                COPY_RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::RuntimeResponseSchemaMismatch {
            operation_id: "copyResource".into(),
            contract_path: "contracts/openapi/foundry/copy-v1.yaml".into(),
            status: "200".into(),
            expected_schema: "OtherResponse".into(),
            actual_schema: "CopyResponse".into(),
        })
    );
}
