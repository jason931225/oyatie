use super::*;

#[test]
fn rejects_lowercase_openapi_response_range_keys() {
    let invalid = replace_response_block(
        "      responses:\n        '2xx':\n          description: Lowercase response range is not an OpenAPI 3.2 response key.\n",
    );
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

#[test]
fn rejects_missing_bearer_security_scheme_for_mutating_operation() {
    let invalid = VALID.replace(
        "  securitySchemes:\n    bearerAuth:\n      type: http\n      scheme: bearer\n      bearerFormat: STS\n",
        "",
    );
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &invalid,
        )]),
        Err(OpenApiSourceError::MissingBearerSecurityScheme {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
        })
    );
}

#[test]
fn rejects_missing_operation_bearer_security_for_mutating_operation() {
    let invalid = VALID.replace("      security:\n        - bearerAuth: []\n", "");
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &invalid,
        )]),
        Err(OpenApiSourceError::MissingOperationSecurity {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            api_path: "/v1/capabilities/{capability_id}/invoke".into(),
            method: "post".into(),
            scheme: "bearerAuth".into(),
        })
    );
}

#[test]
fn rejects_authorization_header_parameter_because_bearer_auth_uses_security_scheme() {
    let invalid = VALID.replace(
        "      parameters:\n",
        "      parameters:\n        - name: Authorization\n          in: header\n          required: true\n          schema:\n            type: string\n          x-oyatie-data-class: SECRET\n",
    );
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &invalid,
        )]),
        Err(OpenApiSourceError::ForbiddenAuthorizationParameter {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            api_path: "/v1/capabilities/{capability_id}/invoke".into(),
            method: "post".into(),
        })
    );
}

#[test]
fn rejects_mutating_operation_missing_required_ingress_header() {
    let invalid = VALID.replace(
        "        - name: Idempotency-Key\n          in: header\n          required: true\n          schema:\n            type: string\n            minLength: 1\n          x-oyatie-rust-type: String\n          x-oyatie-data-class: INTERNAL_ONLY\n",
        "",
    );
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &invalid,
        )]),
        Err(OpenApiSourceError::MissingRequiredHeaderParameter {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            api_path: "/v1/capabilities/{capability_id}/invoke".into(),
            method: "post".into(),
            header: "Idempotency-Key".into(),
        })
    );
}

#[test]
fn rejects_mutating_operation_ingress_header_without_min_length() {
    let invalid = VALID.replace(
        "            minLength: 1\n          x-oyatie-rust-type: String\n          x-oyatie-data-class: INTERNAL_ONLY\n        - name: X-Tenant-Id\n",
        "          x-oyatie-rust-type: String\n          x-oyatie-data-class: INTERNAL_ONLY\n        - name: X-Tenant-Id\n",
    );
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &invalid,
        )]),
        Err(OpenApiSourceError::InvalidHeaderParameter {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            api_path: "/v1/capabilities/{capability_id}/invoke".into(),
            method: "post".into(),
            header: "X-Request-Id".into(),
            reason: "header parameter schema must declare minLength: 1".into(),
        })
    );
}
