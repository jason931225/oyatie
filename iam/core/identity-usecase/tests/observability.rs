#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use iam_identity_domain::IdentityError;
use iam_identity_usecase::{
    IDENTITY_TOKEN_ISSUE_SURFACE, IdentityApiAuthorization, IdentityApiBoundaryContext,
    IdentityApiPrincipal, IdentityScopeRef, IdentityTokenIssueApiError,
    IdentityTokenIssueApiRequest, IdentityTokenIssueIdempotencyLedger, IdentityTokenIssueRequest,
    IdentityTokenRotationRequest, issue_identity_token_from_app,
    observability::{
        IdentityTokenIssueEvent, OutcomeLabel, SURFACE, identity_token_issue_event_for_error,
        identity_token_issue_event_for_success, identity_token_rotate_event_for_error,
        identity_token_rotate_event_for_success,
    },
};

fn valid_request(request_id: &str, idempotency_key: &str) -> IdentityTokenIssueApiRequest {
    IdentityTokenIssueApiRequest {
        boundary: IdentityApiBoundaryContext {
            request_id: request_id.to_string(),
            tenant_id: "ten_alpha".to_string(),
            idempotency_key: idempotency_key.to_string(),
        },
        principal: IdentityApiPrincipal {
            tenant_id: "ten_alpha".to_string(),
            principal_id: "usr_admin".to_string(),
            principal_kind: "human".to_string(),
            owning_capability_id: None,
        },
        authorization: IdentityApiAuthorization {
            tenant_id: "ten_alpha".to_string(),
            principal_id: "usr_admin".to_string(),
            decision_id: "authz_obs_001".to_string(),
            allowed_surfaces: vec![IDENTITY_TOKEN_ISSUE_SURFACE.to_string()],
        },
        body: IdentityTokenIssueRequest {
            tenant_id: "ten_alpha".to_string(),
            subject_id: "usr_admin".to_string(),
            subject_kind: "human".to_string(),
            owning_capability_id: None,
            credential_kind: "sts".to_string(),
            purpose: "CapabilityInvocation".to_string(),
            ttl_seconds: 900,
            scopes: vec![IdentityScopeRef {
                value: "foundry.invoke".to_string(),
            }],
            issued_at_epoch_seconds: 1_700_000_000,
        },
    }
}

#[test]
fn observability_surface_constant_matches_app_surface() {
    assert_eq!(SURFACE, IDENTITY_TOKEN_ISSUE_SURFACE);
    assert_eq!(SURFACE, "identity.token.issue");
}

#[test]
fn outcome_label_strings_are_stable_low_cardinality() {
    assert_eq!(OutcomeLabel::Success.as_str(), "success");
    assert_eq!(OutcomeLabel::Failure.as_str(), "failure");
}

#[test]
fn success_event_has_no_error_code_and_success_outcome() {
    let request = valid_request("req_obs_success", "idem_obs_success");
    let event = identity_token_issue_event_for_success(&request);

    assert_eq!(event.surface, SURFACE);
    assert_eq!(event.outcome, OutcomeLabel::Success);
    assert_eq!(event.error_code, None);
    assert_eq!(event.purpose, Some("CapabilityInvocation"));
    assert_eq!(event.data_class, "AUDIT");
    assert!(event.tenant_id_hash > 0);
}

#[test]
fn tenant_id_hash_is_deterministic_and_not_raw_value() {
    let request = valid_request("req_obs_hash_a", "idem_obs_hash_a");
    let request_b = valid_request("req_obs_hash_b", "idem_obs_hash_b");

    let event_a = identity_token_issue_event_for_success(&request);
    let event_b = identity_token_issue_event_for_success(&request_b);

    assert_eq!(event_a.tenant_id_hash, event_b.tenant_id_hash);

    let mut other_tenant = valid_request("req_obs_hash_c", "idem_obs_hash_c");
    other_tenant.boundary.tenant_id = "ten_beta".to_string();
    let event_c = identity_token_issue_event_for_success(&other_tenant);
    assert_ne!(event_a.tenant_id_hash, event_c.tenant_id_hash);
}

#[test]
fn every_api_error_variant_maps_to_stable_non_empty_error_code() {
    use IdentityTokenIssueApiError::*;

    let request = valid_request("req_obs_err", "idem_obs_err");

    let variants: &[IdentityTokenIssueApiError] = &[
        EmptyRequestId,
        EmptyTenantHeader,
        EmptyIdempotencyKey,
        EmptyPrincipalId,
        EmptyPrincipalKind,
        InvalidPrincipalKind {
            principal_kind: "bad".to_string(),
        },
        EmptySubjectId,
        InvalidSubjectKind {
            subject_kind: "bad".to_string(),
        },
        TenantMismatch {
            header_tenant_id: "ten_a".to_string(),
            principal_tenant_id: "ten_b".to_string(),
            body_tenant_id: "ten_c".to_string(),
        },
        PrincipalMismatch {
            principal_id: "usr_a".to_string(),
            subject_id: "usr_b".to_string(),
        },
        EmptyAuthorizationDecisionId,
        AuthorizationTenantMismatch {
            authorization_tenant_id: "ten_x".to_string(),
            principal_tenant_id: "ten_y".to_string(),
        },
        AuthorizationPrincipalMismatch {
            authorization_principal_id: "usr_x".to_string(),
            principal_id: "usr_y".to_string(),
        },
        AuthorizationDenied {
            surface: "identity.token.issue".to_string(),
        },
        EmptyCredentialKind,
        InvalidCredentialKind {
            credential_kind: "bad".to_string(),
        },
        InvalidPurpose {
            purpose: "bad".to_string(),
        },
        EmptyPreviousTokenFingerprint,
        PreviousTokenNotYetActive {
            previous_issued_at_epoch_seconds: 200,
            rotate_at_epoch_seconds: 100,
        },
        PreviousTokenExpired {
            previous_expires_at_epoch_seconds: 100,
            rotate_at_epoch_seconds: 200,
        },
        RotationBindingMismatch {
            previous_subject_id: "usr_a".to_string(),
            replacement_subject_id: "usr_b".to_string(),
        },
        RotationPurposeScopeMismatch,
        IdempotencyKeyReused {
            idempotency_key: "idem_x".to_string(),
        },
        Identity(IdentityError::InvalidTenantId),
        Identity(IdentityError::InvalidUserId),
        Identity(IdentityError::InvalidRegionPack),
        Identity(IdentityError::InvalidIdentityProviderId),
        Identity(IdentityError::EmptyExternalSubject),
        Identity(IdentityError::InvalidServicePrincipalId),
        Identity(IdentityError::InvalidCapabilityId),
        Identity(IdentityError::EmptyPrimaryIdentifier),
        Identity(IdentityError::TokenTtlTooLong),
        Identity(IdentityError::TokenTtlZero),
        Identity(IdentityError::MissingCredentialScope),
        Identity(IdentityError::LongLivedCredentialForbidden),
    ];

    for variant in variants {
        let event = identity_token_issue_event_for_error(&request, variant);
        let code = event
            .error_code
            .expect("every error variant must have a non-None error_code");
        assert!(
            !code.is_empty(),
            "error code must be non-empty for variant: {variant:?}"
        );
        assert_eq!(event.outcome, OutcomeLabel::Failure);
        assert_eq!(event.surface, SURFACE);
        assert_eq!(event.data_class, "AUDIT");
    }
}

#[test]
fn error_event_with_valid_purpose_in_request_extracts_purpose() {
    let request = valid_request("req_obs_purpose_valid", "idem_obs_purpose_valid");
    let error = IdentityTokenIssueApiError::EmptyCredentialKind;
    let event = identity_token_issue_event_for_error(&request, &error);
    assert_eq!(event.purpose, Some("CapabilityInvocation"));
}

#[test]
fn error_event_with_invalid_purpose_in_request_has_none_purpose() {
    let mut request = valid_request("req_obs_purpose_invalid", "idem_obs_purpose_invalid");
    request.body.purpose = "not-a-valid-purpose".to_string();
    let error = IdentityTokenIssueApiError::InvalidPurpose {
        purpose: "not-a-valid-purpose".to_string(),
    };
    let event = identity_token_issue_event_for_error(&request, &error);
    assert_eq!(event.purpose, None);
}

#[test]
fn rotate_success_event_uses_same_surface_and_has_no_error_code() {
    let mut idempotency = IdentityTokenIssueIdempotencyLedger::default();
    let original = issue_identity_token_from_app(
        &mut idempotency,
        valid_request("req_obs_rotate_orig", "idem_obs_rotate_orig"),
    )
    .expect("original issue succeeds");

    let mut replacement = valid_request("req_obs_rotate_new", "idem_obs_rotate_new");
    replacement.body.issued_at_epoch_seconds = original.data.issued_at_epoch_seconds + 600;

    let rotation = IdentityTokenRotationRequest {
        previous: original.data.clone(),
        replacement,
    };

    let event = identity_token_rotate_event_for_success(&rotation);
    assert_eq!(event.surface, SURFACE);
    assert_eq!(event.outcome, OutcomeLabel::Success);
    assert_eq!(event.error_code, None);
    assert_eq!(event.data_class, "AUDIT");
}

#[test]
fn rotate_error_event_has_stable_error_code_and_failure_outcome() {
    let replacement = valid_request("req_obs_rotate_err", "idem_obs_rotate_err");
    let previous = {
        let mut idempotency = IdentityTokenIssueIdempotencyLedger::default();
        issue_identity_token_from_app(
            &mut idempotency,
            valid_request("req_obs_rotate_prev", "idem_obs_rotate_prev"),
        )
        .expect("previous issue succeeds")
        .data
    };

    let rotation = IdentityTokenRotationRequest {
        previous,
        replacement,
    };
    let error = IdentityTokenIssueApiError::RotationPurposeScopeMismatch;
    let event = identity_token_rotate_event_for_error(&rotation, &error);

    assert_eq!(event.surface, SURFACE);
    assert_eq!(event.outcome, OutcomeLabel::Failure);
    let code = event.error_code.expect("rotation error must have a code");
    assert!(!code.is_empty());
    assert_eq!(event.data_class, "AUDIT");
}
