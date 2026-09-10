use super::*;

#[test]
fn rejects_path_template_parameter_without_explicit_operation_parameter() {
    let missing = VALID.replace(
        "        - name: capability_id\n          in: path\n          required: true\n          schema:\n            type: string\n          x-oyatie-data-class: INTERNAL_ONLY\n",
        "",
    );
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &missing,
        )]),
        Err(OpenApiSourceError::MissingPathTemplateParameter {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            api_path: "/v1/capabilities/{capability_id}/invoke".into(),
            method: "post".into(),
            parameter: "capability_id".into(),
        })
    );

    let optional = VALID.replace(
        "        - name: capability_id\n          in: path\n          required: true\n          schema:\n            type: string\n          x-oyatie-data-class: INTERNAL_ONLY\n",
        "        - name: capability_id\n          in: path\n          required: false\n          schema:\n            type: string\n          x-oyatie-data-class: INTERNAL_ONLY\n",
    );
    assert_eq!(
        validate_openapi_documents([document(
            "contracts/openapi/foundry/capability-v1.yaml",
            &optional,
        )]),
        Err(OpenApiSourceError::InvalidPathTemplateParameter {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            api_path: "/v1/capabilities/{capability_id}/invoke".into(),
            method: "post".into(),
            parameter: "capability_id".into(),
            reason: "path template parameter must be required: true".into(),
        })
    );
}
