use super::*;

#[test]
fn rejects_runtime_binding_without_response_status_coverage() {
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                "pub const CAPABILITY_INVOKE_SURFACE: &str = \"foundry.capability.invoke\";\n\
pub enum CapabilityInvokeApiStatus { Accepted, BadRequest }\n\
impl CapabilityInvokeApiStatus { pub const fn code(self) -> u16 { match self { Self::Accepted => 202, Self::BadRequest => 400 } } }\n\
pub fn invoke_capability_from_api() { let _surface = CAPABILITY_INVOKE_SURFACE; }",
            )],
            [runtime_source(
                "crates/intelligence-api/tests/capability_invoke_api.rs",
                RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::MissingRuntimeResponseStatus {
            operation_id: "invokeCapability".into(),
            path: "crates/intelligence-api/src/lib.rs".into(),
            status_type: "CapabilityInvokeApiStatus".into(),
            status: "403".into(),
        })
    );

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
                "invoke_capability_from_api(foundation, request); assert_eq!(surface, \"foundry.capability.invoke\"); assert_eq!(CapabilityInvokeApiStatus::Accepted.code(), 202); assert_eq!(CapabilityInvokeApiStatus::BadRequest.code(), 400);",
            )],
        ),
        Err(OpenApiSourceError::MissingRuntimeTestResponseStatus {
            operation_id: "invokeCapability".into(),
            test_path: "crates/intelligence-api/tests/capability_invoke_api.rs".into(),
            status_type: "CapabilityInvokeApiStatus".into(),
            status: "403".into(),
        })
    );
}

#[test]
fn rejects_runtime_binding_without_typed_status_source() {
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                "pub const CAPABILITY_INVOKE_SURFACE: &str = \"foundry.capability.invoke\";\n\
pub fn invoke_capability_from_api() { let _accepted = 202; let _bad_request = 400; let _forbidden = 403; }",
            )],
            [runtime_source(
                "crates/intelligence-api/tests/capability_invoke_api.rs",
                RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::MissingRuntimeStatusType {
            operation_id: "invokeCapability".into(),
            path: "crates/intelligence-api/src/lib.rs".into(),
            status_type: "CapabilityInvokeApiStatus".into(),
        })
    );
}

#[test]
fn rejects_runtime_binding_when_status_type_is_not_public_api_enum() {
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                "pub const CAPABILITY_INVOKE_SURFACE: &str = \"foundry.capability.invoke\";\n\
enum CapabilityInvokeApiStatus { Accepted, BadRequest, Forbidden }\n\
impl CapabilityInvokeApiStatus { pub const fn code(self) -> u16 { match self { Self::Accepted => 202, Self::BadRequest => 400, Self::Forbidden => 403 } } }\n\
pub fn invoke_capability_from_api() { let _surface = CAPABILITY_INVOKE_SURFACE; }",
            )],
            [runtime_source(
                "crates/intelligence-api/tests/capability_invoke_api.rs",
                RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::MissingRuntimeStatusType {
            operation_id: "invokeCapability".into(),
            path: "crates/intelligence-api/src/lib.rs".into(),
            status_type: "CapabilityInvokeApiStatus".into(),
        })
    );
}

#[test]
fn rejects_runtime_status_codes_not_documented_by_openapi() {
    assert_eq!(
        validate_openapi_runtime_parity(
            [document(
                "contracts/openapi/foundry/capability-v1.yaml",
                VALID,
            )],
            [runtime_binding()],
            [runtime_source(
                "crates/intelligence-api/src/lib.rs",
                "pub const CAPABILITY_INVOKE_SURFACE: &str = \"foundry.capability.invoke\";\n\
	pub enum CapabilityInvokeApiStatus { Accepted, BadRequest, Forbidden, NotFound }\n\
	impl CapabilityInvokeApiStatus { pub const fn code(self) -> u16 { match self { Self::Accepted => 202, Self::BadRequest => 400, Self::Forbidden => 403, Self::NotFound => 404 } } }\n\
pub fn invoke_capability_from_api() { let _surface = CAPABILITY_INVOKE_SURFACE; }",
            )],
            [runtime_source(
                "crates/intelligence-api/tests/capability_invoke_api.rs",
                RUNTIME_API_TEST,
            )],
        ),
        Err(OpenApiSourceError::UndocumentedRuntimeResponseStatus {
            operation_id: "invokeCapability".into(),
            path: "crates/intelligence-api/src/lib.rs".into(),
            status_type: "CapabilityInvokeApiStatus".into(),
            status: "404".into(),
        })
    );
}

#[test]
fn rejects_invalid_runtime_status_type_mappings() {
    let undeclared_variant = "pub const CAPABILITY_INVOKE_SURFACE: &str = \"foundry.capability.invoke\";\n\
pub enum CapabilityInvokeApiStatus { Accepted, BadRequest, Forbidden }\n\
impl CapabilityInvokeApiStatus { pub const fn code(self) -> u16 { match self { Self::Accepted => 202, Self::BadRequest => 400, Self::Teapot => 418 } } }\n\
pub fn invoke_capability_from_api() { let _surface = CAPABILITY_INVOKE_SURFACE; }";
    assert_eq!(
        invalid_runtime_status_type_reason(undeclared_variant),
        "CapabilityInvokeApiStatus maps undeclared variant Teapot"
    );

    let missing_variant = "pub const CAPABILITY_INVOKE_SURFACE: &str = \"foundry.capability.invoke\";\n\
pub enum CapabilityInvokeApiStatus { Accepted, BadRequest, Forbidden, NotFound }\n\
impl CapabilityInvokeApiStatus { pub const fn code(self) -> u16 { match self { Self::Accepted => 202, Self::BadRequest => 400, Self::Forbidden => 403 } } }\n\
pub fn invoke_capability_from_api() { let _surface = CAPABILITY_INVOKE_SURFACE; }";
    assert_eq!(
        invalid_runtime_status_type_reason(missing_variant),
        "CapabilityInvokeApiStatus code mappings do not cover enum variants: missing=[\"NotFound\"], extra=[]"
    );

    let wildcard_arm = "pub const CAPABILITY_INVOKE_SURFACE: &str = \"foundry.capability.invoke\";\n\
pub enum CapabilityInvokeApiStatus { Accepted, BadRequest, Forbidden }\n\
impl CapabilityInvokeApiStatus { pub const fn code(self) -> u16 { match self { Self::Accepted => 202, Self::BadRequest => 400, _ => 403 } } }\n\
pub fn invoke_capability_from_api() { let _surface = CAPABILITY_INVOKE_SURFACE; }";
    assert_eq!(
        invalid_runtime_status_type_reason(wildcard_arm),
        "status code mappings must use explicit Self::Variant arms"
    );

    let non_literal_status = "pub const CAPABILITY_INVOKE_SURFACE: &str = \"foundry.capability.invoke\";\n\
pub enum CapabilityInvokeApiStatus { Accepted, BadRequest, Forbidden }\n\
impl CapabilityInvokeApiStatus { pub const fn code(self) -> u16 { match self { Self::Accepted => 202, Self::BadRequest => 400, Self::Forbidden => FORBIDDEN } } }\n\
pub fn invoke_capability_from_api() { let _surface = CAPABILITY_INVOKE_SURFACE; }";
    assert_eq!(
        invalid_runtime_status_type_reason(non_literal_status),
        "status code mappings must return explicit three-digit numeric literals"
    );
}
