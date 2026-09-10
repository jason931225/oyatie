use super::*;

#[test]
fn rejects_runtime_binding_when_source_or_test_only_contain_substring_markers() {
    let shadow_source = "pub const CAPABILITY_INVOKE_SURFACE: &str = \"foundry.capability.invoke\";\n\
    pub enum CapabilityInvokeApiStatus { Accepted, BadRequest, Forbidden }\n\
    impl CapabilityInvokeApiStatus { pub const fn code(self) -> u16 { match self { Self::Accepted => 202, Self::BadRequest => 400, Self::Forbidden => 403 } } }\n\
    pub fn invoke_capability_from_api_shadow() { let _surface = CAPABILITY_INVOKE_SURFACE; }";

    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                shadow_source,
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

    let shadow_test = "invoke_capability_from_api_shadow(foundation, request);\n\
        assert_eq!(CAPABILITY_INVOKE_SURFACE_SHADOW, \"foundry.capability.invoke.shadow\");\n\
        assert_eq!(CapabilityInvokeApiStatusShadow::Accepted.code(), 202);\n\
        assert_eq!(CapabilityInvokeApiStatusShadow::BadRequest.code(), 400);\n\
        assert_eq!(CapabilityInvokeApiStatusShadow::Forbidden.code(), 403);";

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
                shadow_test,
            )],
        ),
        Err(OpenApiSourceError::MissingRuntimeTestCoverage {
            operation_id: "invokeCapability".into(),
            test_path: "crates/intelligence-api/tests/capability_invoke_api.rs".into(),
            symbol: "invoke_capability_from_api".into(),
            evidence_surface: "foundry.capability.invoke".into(),
        })
    );
}

#[test]
fn rejects_runtime_binding_when_symbol_is_not_public_api_function() {
    let private_source = "pub const CAPABILITY_INVOKE_SURFACE: &str = \"foundry.capability.invoke\";\n\
    pub enum CapabilityInvokeApiStatus { Accepted, BadRequest, Forbidden }\n\
    impl CapabilityInvokeApiStatus { pub const fn code(self) -> u16 { match self { Self::Accepted => 202, Self::BadRequest => 400, Self::Forbidden => 403 } } }\n\
    fn invoke_capability_from_api() { let _surface = CAPABILITY_INVOKE_SURFACE; }";

    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                private_source,
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

    let restricted_source = "pub const CAPABILITY_INVOKE_SURFACE: &str = \"foundry.capability.invoke\";\n\
    pub enum CapabilityInvokeApiStatus { Accepted, BadRequest, Forbidden }\n\
    impl CapabilityInvokeApiStatus { pub const fn code(self) -> u16 { match self { Self::Accepted => 202, Self::BadRequest => 400, Self::Forbidden => 403 } } }\n\
    pub(crate) fn invoke_capability_from_api() { let _surface = CAPABILITY_INVOKE_SURFACE; }";

    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                restricted_source,
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
fn rejects_runtime_status_test_coverage_without_assertion_shape() {
    let weak_status_test = r#"
    invoke_capability_from_api(foundation, request);
    assert_eq!(CAPABILITY_INVOKE_SURFACE, "foundry.capability.invoke");
    let _accepted = (CapabilityInvokeApiStatus, 202);
    let _bad_request = (CapabilityInvokeApiStatus, 400);
    let _forbidden = (CapabilityInvokeApiStatus, 403);
    "#;

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
                weak_status_test,
            )],
        ),
        Err(OpenApiSourceError::MissingRuntimeTestResponseStatus {
            operation_id: "invokeCapability".into(),
            test_path: "crates/intelligence-api/tests/capability_invoke_api.rs".into(),
            status_type: "CapabilityInvokeApiStatus".into(),
            status: "202".into(),
        })
    );
}

#[test]
fn rejects_runtime_status_test_coverage_with_symbolic_status_constants() {
    let symbolic_status_test = r#"
    invoke_capability_from_api(foundation, request);
    assert_eq!(CAPABILITY_INVOKE_SURFACE, "foundry.capability.invoke");
    assert_eq!(CapabilityInvokeApiStatus::Accepted.code(), HTTP_202_ACCEPTED);
    assert_eq!(CapabilityInvokeApiStatus::BadRequest.code(), HTTP_400_BAD_REQUEST);
    assert_eq!(CapabilityInvokeApiStatus::Forbidden.code(), HTTP_403_FORBIDDEN);
    "#;

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
                symbolic_status_test,
            )],
        ),
        Err(OpenApiSourceError::MissingRuntimeTestResponseStatus {
            operation_id: "invokeCapability".into(),
            test_path: "crates/intelligence-api/tests/capability_invoke_api.rs".into(),
            status_type: "CapabilityInvokeApiStatus".into(),
            status: "202".into(),
        })
    );
}
