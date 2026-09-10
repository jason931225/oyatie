//! Pins the serialized JSON shape of the workload-identity DTOs: camelCase
//! keys and stable enum strings are a wire contract, so a rename here breaks
//! every deployed client.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use iam_identity_workload_api::{
    ApiErrorEnvelope, AuthorizeResponse, AuthorizeWithTokenRequest, BatchAuthorizeResponse,
    ClaimValueDto, DecisionReasonDto, EffectDto, PrincipalLifecycleResponse, ResourceDto,
    ValidateTokenResponse,
};
use iam_identity_workload_domain::{AuthorizationDecision, ClaimValue, WorkloadState};
use serde_json::json;

#[test]
fn authorize_with_token_request_uses_camel_case_and_typed_claims() {
    let mut context = BTreeMap::new();
    context.insert("mfa".to_owned(), ClaimValueDto::Bool(true));
    let request = AuthorizeWithTokenRequest {
        token: "header.payload.sig".to_owned(),
        action: "cloud.kms.Decrypt".to_owned(),
        resource: ResourceDto {
            resource_type: "Secret".to_owned(),
            resource_id: "db-password".to_owned(),
            attributes: BTreeMap::new(),
        },
        context,
    };
    let body = serde_json::to_value(&request).expect("serialize");
    assert_eq!(body["token"], "header.payload.sig");
    assert_eq!(body["action"], "cloud.kms.Decrypt");
    assert_eq!(body["resource"]["resourceType"], "Secret");
    assert_eq!(body["resource"]["resourceId"], "db-password");
    // Tag/content encoding is what makes the typed claim round-trip losslessly.
    assert_eq!(body["context"]["mfa"]["kind"], "bool");
    assert_eq!(body["context"]["mfa"]["value"], true);

    assert_eq!(request.action().as_str(), "cloud.kms.Decrypt");
    assert!(request.context_domain().contains_key("mfa"));
}

#[test]
fn resource_attributes_round_trip_into_domain_resource() {
    let request: AuthorizeWithTokenRequest = serde_json::from_value(json!({
        "token": "header.payload.sig",
        "action": "quota:Read",
        "resource": {
            "resourceType": "QuotaRecord",
            "resourceId": "ten_acme",
            "attributes": {
                "tenant_id": { "kind": "text", "value": "ten_acme" }
            }
        },
        "context": {}
    }))
    .expect("deserialize authorize request");

    let domain = request.resource.into_domain();

    assert_eq!(
        domain.attributes().get("tenant_id"),
        Some(&ClaimValue::Text("ten_acme".to_owned())),
        "resource tenant_id must reach the PDP domain resource for same-tenant Cedar policies"
    );
}

#[test]
fn authorize_response_deny_carries_reason_not_a_bare_bool() {
    let response = AuthorizeResponse::from(&AuthorizationDecision::forbid("freeze-checkout"));
    let body = serde_json::to_value(&response).expect("serialize");
    assert_eq!(body["effect"], "DENY");
    assert_eq!(body["reason"]["kind"], "explicitForbid");
    assert_eq!(body["reason"]["policyId"], "freeze-checkout");
}

#[test]
fn batch_authorize_response_preserves_order() {
    let response = BatchAuthorizeResponse {
        decisions: vec![
            AuthorizeResponse::from(&AuthorizationDecision::permit("p1")),
            AuthorizeResponse::from(&AuthorizationDecision::default_deny()),
        ],
    };
    let body = serde_json::to_value(&response).expect("serialize");
    assert_eq!(body["decisions"][0]["effect"], "ALLOW");
    assert_eq!(body["decisions"][1]["effect"], "DENY");
    assert_eq!(body["decisions"][1]["reason"]["kind"], "defaultDeny");
}

#[test]
fn validate_token_response_exposes_trust_domain_and_state() {
    let response = ValidateTokenResponse {
        tenant_id: "ten_acme".to_owned(),
        workload_id: "wl_secrets_sync".to_owned(),
        owning_capability: "cap.cloud.kms".to_owned(),
        trust_domain: "spiffe://ten_acme".to_owned(),
        state: "active".to_owned(),
        scopes: vec!["cloud.kms.decrypt".to_owned()],
    };
    let body = serde_json::to_value(&response).expect("serialize");
    assert_eq!(body["tenantId"], "ten_acme");
    assert_eq!(body["workloadId"], "wl_secrets_sync");
    assert_eq!(body["owningCapability"], "cap.cloud.kms");
    assert_eq!(body["trustDomain"], "spiffe://ten_acme");
    assert_eq!(body["state"], "active");
    assert_eq!(body["scopes"][0], "cloud.kms.decrypt");
}

#[test]
fn lifecycle_response_round_trips() {
    let response = PrincipalLifecycleResponse::new("wl_secrets_sync", WorkloadState::Retired);
    let body = serde_json::to_value(&response).expect("serialize");
    assert_eq!(
        body,
        json!({ "workloadId": "wl_secrets_sync", "state": "retired" })
    );
    let back: PrincipalLifecycleResponse = serde_json::from_value(body).expect("deserialize");
    assert_eq!(back, response);
}

#[test]
fn error_envelopes_have_stable_codes() {
    assert_eq!(
        serde_json::to_value(ApiErrorEnvelope::token_invalid(None)).expect("serialize")["error"]["code"],
        "TOKEN_INVALID"
    );
    assert_eq!(
        serde_json::to_value(ApiErrorEnvelope::forbidden(None)).expect("serialize")["error"]["code"],
        "FORBIDDEN"
    );
    assert_eq!(
        serde_json::to_value(ApiErrorEnvelope::dependency_unavailable(None)).expect("serialize")["error"]
            ["code"],
        "DEPENDENCY_UNAVAILABLE"
    );
}

#[test]
fn effect_dto_serializes_screaming_snake() {
    assert_eq!(
        serde_json::to_value(EffectDto::Allow).expect("serialize"),
        json!("ALLOW")
    );
    assert_eq!(
        serde_json::to_value(EffectDto::Deny).expect("serialize"),
        json!("DENY")
    );
}

#[test]
fn decision_reason_dto_default_deny_is_tagged() {
    let reason = DecisionReasonDto::DefaultDeny;
    assert_eq!(
        serde_json::to_value(reason).expect("serialize"),
        json!({ "kind": "defaultDeny" })
    );
}
