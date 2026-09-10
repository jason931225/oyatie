use super::*;

#[test]
fn accepts_versioned_openapi_32_document_with_operation_responses() {
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            VALID
        )]),
        Ok(OpenApiSourceReport {
            documents_checked: 1,
            operations_checked: 1,
            data_class_annotations_checked: 33,
        })
    );
}

#[test]
fn accepts_openapi_32_range_and_default_response_keys_as_source_shape() {
    let with_range = VALID.replace("        '202':", "        '2XX':");
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &with_range
        )]),
        Ok(OpenApiSourceReport {
            documents_checked: 1,
            operations_checked: 1,
            data_class_annotations_checked: 33,
        })
    );

    let with_default = VALID.replace("        '403':", "        default:");
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &with_default
        )]),
        Ok(OpenApiSourceReport {
            documents_checked: 1,
            operations_checked: 1,
            data_class_annotations_checked: 33,
        })
    );
}

#[test]
fn accepts_openapi_32_query_fixed_operation() {
    let query = query_operation_document();
    assert_eq!(
        validate_openapi_documents([document("contracts/openapi/foundry/query-v1.yaml", &query,)]),
        Ok(OpenApiSourceReport {
            documents_checked: 1,
            operations_checked: 1,
            data_class_annotations_checked: 1,
        })
    );
}

#[test]
fn rejects_duplicate_operation_ids_during_source_validation() {
    let duplicate = query_operation_document().replace(
        "components:\n",
        "  /v1/capability-queries/duplicate:\n    query:\n      operationId: queryCapability\n      responses:\n        '200':\n          description: Query completed.\n          content:\n            application/json:\n              schema:\n                $ref: '#/components/schemas/QueryResponse'\ncomponents:\n",
    );

    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/query-v1.yaml",
            &duplicate,
        )]),
        Err(OpenApiSourceError::DuplicateOperationId {
            operation_id: "queryCapability".into(),
            first_path: "contracts/openapi/foundry/query-v1.yaml".into(),
            second_path: "contracts/openapi/foundry/query-v1.yaml".into(),
        })
    );
}

#[test]
fn accepts_openapi_32_additional_operations_custom_method() {
    let copy = copy_operation_document();
    assert_eq!(
        validate_openapi_documents([document("contracts/openapi/foundry/copy-v1.yaml", &copy,)]),
        Ok(OpenApiSourceReport {
            documents_checked: 1,
            operations_checked: 1,
            data_class_annotations_checked: 5,
        })
    );
}

#[test]
fn rejects_openapi_31_contracts_after_32_pivot() {
    let invalid = VALID.replace("openapi: 3.2.0", "openapi: 3.1.0");

    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &invalid
        )]),
        Err(OpenApiSourceError::UnsupportedOpenApiVersion {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            version: "3.1.0".into(),
        })
    );
}

#[test]
fn rejects_empty_or_non_contract_documents() {
    assert_eq!(
        validate_openapi_documents([]),
        Err(OpenApiSourceError::NoDocuments)
    );
    assert_eq!(
        validate_openapi_documents([document("docs/openapi.yaml", VALID)]),
        Err(OpenApiSourceError::InvalidPath {
            path: "docs/openapi.yaml".into(),
            reason: "OpenAPI documents must live under contracts/openapi/".into(),
        })
    );
}

#[test]
fn rejects_version_drift_between_filename_and_info_version() {
    let invalid = VALID.replace("version: 1.0.0", "version: 2.0.0");
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &invalid,
        )]),
        Err(OpenApiSourceError::VersionSuffixMismatch {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            path_major: 1,
            info_major: 2,
        })
    );
}

#[test]
fn rejects_missing_operation_id() {
    let invalid = VALID.replace("      operationId: invokeCapability\n", "");
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &invalid,
        )]),
        Err(OpenApiSourceError::MissingOperationId {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            api_path: "/v1/capabilities/{capability_id}/invoke".into(),
            method: "post".into(),
        })
    );
}

#[test]
fn rejects_operations_without_response_statuses() {
    let invalid = replace_response_block("      responses:\n");
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &invalid,
        )]),
        Err(OpenApiSourceError::MissingResponses {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            api_path: "/v1/capabilities/{capability_id}/invoke".into(),
            method: "post".into(),
        })
    );
}
