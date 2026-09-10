use super::*;

#[test]
fn accepts_openapi_runtime_binding_when_source_and_test_cover_operation() {
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                RUNTIME_API
            )],
            [runtime_source(
                "crates/intelligence-api/tests/capability_invoke_api.rs",
                RUNTIME_API_TEST,
            )],
        ),
        Ok(OpenApiRuntimeParityReport {
            operations_checked: 1,
            bindings_checked: 1,
            sources_checked: 1,
            tests_checked: 1,
            response_statuses_checked: 3,
            response_schemas_checked: 3,
        })
    );
}

#[test]
fn accepts_openapi_runtime_binding_at_canonical_capability_face_paths() {
    // ADR-0562 canonical capability-face layout: `<capability>/<face>/<crate>`
    // with the package named `<capability>-<crate>` (intelligence/core/api
    // hosts intelligence-api). The shape validator must accept these paths
    // just like the legacy `crates/<crate>/` layout.
    let canonical = OpenApiRuntimeBinding {
        runtime_crate: "intelligence-api".into(),
        source_path: "intelligence/core/api/src/lib.rs".into(),
        test_path: "intelligence/core/api/tests/capability_invoke_api.rs".into(),
        ..runtime_binding()
    };
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [canonical],
            [runtime_source(
                "intelligence/core/api/src/lib.rs",
                RUNTIME_API
            )],
            [runtime_source(
                "intelligence/core/api/tests/capability_invoke_api.rs",
                RUNTIME_API_TEST,
            )],
        ),
        Ok(OpenApiRuntimeParityReport {
            operations_checked: 1,
            bindings_checked: 1,
            sources_checked: 1,
            tests_checked: 1,
            response_statuses_checked: 3,
            response_schemas_checked: 3,
        })
    );
}

#[test]
fn rejects_non_explicit_openapi_responses_for_typed_runtime_parity() {
    let with_range = VALID.replace("        '202':", "        '2XX':");
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                &with_range,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                RUNTIME_API
            )],
            [runtime_source(
                "crates/intelligence-api/tests/capability_invoke_api.rs",
                RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::NonExplicitRuntimeResponseKey {
            operation_id: "invokeCapability".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            response_key: "2XX".into(),
        })
    );

    let with_default = VALID.replace("        '403':", "        default:");
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                &with_default,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                RUNTIME_API
            )],
            [runtime_source(
                "crates/intelligence-api/tests/capability_invoke_api.rs",
                RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::NonExplicitRuntimeResponseKey {
            operation_id: "invokeCapability".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            response_key: "default".into(),
        })
    );
}

#[test]
fn rejects_runtime_response_status_without_schema_ref() {
    let invalid = VALID.replacen(
        "          content:\n            application/json:\n              schema:\n                $ref: '#/components/schemas/CapabilityInvokeApiErrorResponse'\n",
        "",
        1,
    );
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                &invalid,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                RUNTIME_API
            )],
            [runtime_source(
                "crates/intelligence-api/tests/capability_invoke_api.rs",
                RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::MissingRuntimeResponseSchema {
            operation_id: "invokeCapability".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            status: "400".into(),
        })
    );
}

#[test]
fn rejects_runtime_response_status_with_unexpected_schema_ref() {
    let invalid = VALID.replace(
        "                $ref: '#/components/schemas/CapabilityInvokeApiSuccessResponse'\n",
        "                $ref: '#/components/schemas/CapabilityInvokeApiErrorResponse'\n",
    );
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                &invalid,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                RUNTIME_API
            )],
            [runtime_source(
                "crates/intelligence-api/tests/capability_invoke_api.rs",
                RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::RuntimeResponseSchemaMismatch {
            operation_id: "invokeCapability".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            status: "202".into(),
            expected_schema: "CapabilityInvokeApiSuccessResponse".into(),
            actual_schema: "CapabilityInvokeApiErrorResponse".into(),
        })
    );
}

#[test]
fn rejects_openapi_operation_without_runtime_binding() {
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [],
            [],
            [],
        ),
        Err(OpenApiSourceError::MissingRuntimeBinding {
            operation_id: "invokeCapability".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
        })
    );
}

#[test]
fn rejects_runtime_binding_without_source_symbol_or_test_coverage() {
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                "pub fn other_symbol() { let _surface = \"foundry.capability.invoke\"; let _status = 202 + 400 + 403; }",
            )],
            [runtime_source(
                "crates/intelligence-api/tests/capability_invoke_api.rs",
                RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::MissingRuntimeSymbol {
            operation_id: "invokeCapability".into(),
            path: "crates/intelligence-api/src/lib.rs".into(),
            symbol: "invoke_capability_from_api".into(),
        })
    );
}

#[test]
fn rejects_runtime_binding_when_evidence_surface_is_not_public_constant() {
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                "const CAPABILITY_INVOKE_SURFACE: &str = \"foundry.capability.invoke\";\n\
pub enum CapabilityInvokeApiStatus { Accepted, BadRequest, Forbidden }\n\
impl CapabilityInvokeApiStatus { pub const fn code(self) -> u16 { match self { Self::Accepted => 202, Self::BadRequest => 400, Self::Forbidden => 403 } } }\n\
pub fn invoke_capability_from_api() { let _surface = CAPABILITY_INVOKE_SURFACE; }",
            )],
            [runtime_source(
                "crates/intelligence-api/tests/capability_invoke_api.rs",
                RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::MissingRuntimeEvidenceSurface {
            operation_id: "invokeCapability".into(),
            path: "crates/intelligence-api/src/lib.rs".into(),
            evidence_surface: "foundry.capability.invoke".into(),
        })
    );
}
