#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use iam_identity_api::{
    IDENTITY_USER_UPSERT_OPENAPI_CONTRACT, IDENTITY_USER_UPSERT_SURFACE,
    IdentityUserApiAuthorization, IdentityUserApiBoundaryContext, IdentityUserApiPrincipal,
    IdentityUserDirectory, IdentityUserRoleRef, IdentityUserUpsertApiError,
    IdentityUserUpsertApiRequest, IdentityUserUpsertApiStatus, IdentityUserUpsertIdempotencyLedger,
    IdentityUserUpsertRequest, upsert_identity_user_from_api,
};

const REQUEST_ID: &str = "req_identity_user_001";
const IDEMPOTENCY_KEY: &str = "idem_identity_user_001";
const TENANT_ID: &str = "ten_alpha";
const USER_ID: &str = "usr_admin";

#[test]
fn identity_user_upsert_contract_runtime_constants_are_covered() {
    assert_eq!(IDENTITY_USER_UPSERT_SURFACE, "identity.user.upsert");
    assert_eq!(
        IDENTITY_USER_UPSERT_OPENAPI_CONTRACT,
        "contracts/openapi/platform/platform-identity-user-v1.yaml"
    );
    assert_eq!(IdentityUserUpsertApiStatus::Ok.code(), 200);
    assert_eq!(IdentityUserUpsertApiStatus::BadRequest.code(), 400);
    assert_eq!(IdentityUserUpsertApiStatus::Unauthorized.code(), 401);
    assert_eq!(IdentityUserUpsertApiStatus::Forbidden.code(), 403);
    assert_eq!(IdentityUserUpsertApiStatus::Conflict.code(), 409);
    assert_eq!(IdentityUserUpsertApiStatus::UnprocessableEntity.code(), 422);
}

#[test]
fn identity_user_upsert_creates_and_replays_same_idempotent_result() {
    let mut directory = IdentityUserDirectory::default();
    let mut idempotency = IdentityUserUpsertIdempotencyLedger::default();
    let request = user_request(REQUEST_ID, IDEMPOTENCY_KEY, TENANT_ID, USER_ID);

    let first = upsert_identity_user_from_api(&mut directory, &mut idempotency, request.clone())
        .expect("first user upsert succeeds");
    let second = upsert_identity_user_from_api(&mut directory, &mut idempotency, request)
        .expect("same user upsert request replays");

    assert_eq!(first, second);
    assert_eq!(directory.len(), 1);
    assert_eq!(idempotency.len(), 1);
    assert_eq!(first.data.tenant_id, TENANT_ID);
    assert_eq!(first.data.user_id, USER_ID);
    assert_eq!(first.data.primary_identifier, "admin@kr.example");
    assert_eq!(first.data.display_name, "KR Admin");
    assert_eq!(first.data.roles[0].value, "tenant.admin");
    assert_eq!(first.data.region_pack, "pack-alpha");
    assert_eq!(first.data.identity_provider_id, "idp_kr_oidc");
    assert_eq!(first.data.external_subject, "oidc://kr.example/admin");
    assert_eq!(first.data.schema_version, 1);
    assert_eq!(first.metadata.result, "created");
    assert_eq!(first.metadata.request_id, REQUEST_ID);
    assert!(directory.get(TENANT_ID, USER_ID).is_some());
}

#[test]
fn identity_user_upsert_updates_existing_user_with_same_primary_identifier() {
    let mut directory = IdentityUserDirectory::default();
    let mut idempotency = IdentityUserUpsertIdempotencyLedger::default();
    upsert_identity_user_from_api(
        &mut directory,
        &mut idempotency,
        user_request(
            "req_identity_user_create",
            "idem_identity_user_create",
            TENANT_ID,
            USER_ID,
        ),
    )
    .expect("initial user upsert succeeds");

    let mut update = user_request(
        "req_identity_user_update",
        "idem_identity_user_update",
        TENANT_ID,
        USER_ID,
    );
    update.body.display_name = "KR Admin Updated".to_string();
    update.body.roles.push(IdentityUserRoleRef {
        value: "billing.admin".to_string(),
    });
    let response = upsert_identity_user_from_api(&mut directory, &mut idempotency, update)
        .expect("same primary identifier can update user profile");

    assert_eq!(response.metadata.result, "updated");
    assert_eq!(response.data.display_name, "KR Admin Updated");
    assert_eq!(response.data.roles.len(), 2);
    assert_eq!(directory.len(), 1);
}

#[test]
fn identity_user_upsert_rejects_path_body_and_tenant_drift_before_mutation() {
    let mut directory = IdentityUserDirectory::default();
    let mut idempotency = IdentityUserUpsertIdempotencyLedger::default();
    let mut user_drift = user_request(
        "req_identity_user_drift_user",
        "idem_identity_user_drift_user",
        TENANT_ID,
        USER_ID,
    );
    user_drift.body.user_id = "usr_other".to_string();

    let user_error = upsert_identity_user_from_api(&mut directory, &mut idempotency, user_drift)
        .expect_err("path/body user drift is rejected");
    assert!(matches!(
        user_error,
        IdentityUserUpsertApiError::UserPathBodyMismatch { .. }
    ));
    assert_eq!(user_error.identity_user_upsert_status_code(), 400);

    let mut tenant_drift = user_request(
        "req_identity_user_drift_tenant",
        "idem_identity_user_drift_tenant",
        TENANT_ID,
        USER_ID,
    );
    tenant_drift.boundary.tenant_id = "ten_other".to_string();
    let tenant_error =
        upsert_identity_user_from_api(&mut directory, &mut idempotency, tenant_drift)
            .expect_err("authenticated tenant drift is forbidden");
    assert!(matches!(
        tenant_error,
        IdentityUserUpsertApiError::TenantMismatch { .. }
    ));
    assert_eq!(
        tenant_error.identity_user_upsert_status(),
        IdentityUserUpsertApiStatus::Forbidden
    );
    assert!(directory.is_empty());
    assert!(idempotency.is_empty());
}

#[test]
fn identity_user_upsert_separates_missing_principal_from_denied_authorization() {
    let mut directory = IdentityUserDirectory::default();
    let mut idempotency = IdentityUserUpsertIdempotencyLedger::default();
    let mut unauthenticated = user_request(
        "req_identity_user_authn",
        "idem_identity_user_authn",
        TENANT_ID,
        USER_ID,
    );
    unauthenticated.principal.principal_id.clear();

    let authn_error =
        upsert_identity_user_from_api(&mut directory, &mut idempotency, unauthenticated)
            .expect_err("missing principal is authentication failure");
    assert_eq!(
        authn_error.identity_user_upsert_status(),
        IdentityUserUpsertApiStatus::Unauthorized
    );

    let mut denied = user_request(
        "req_identity_user_authz",
        "idem_identity_user_authz",
        TENANT_ID,
        USER_ID,
    );
    denied.authorization.allowed_surfaces = vec!["identity.token.issue".to_string()];
    let authz_error = upsert_identity_user_from_api(&mut directory, &mut idempotency, denied)
        .expect_err("missing identity.user.upsert grant is authorization failure");
    assert!(
        matches!(authz_error, IdentityUserUpsertApiError::AuthorizationDenied { ref surface } if surface == IDENTITY_USER_UPSERT_SURFACE)
    );
    assert_eq!(
        authz_error.identity_user_upsert_status(),
        IdentityUserUpsertApiStatus::Forbidden
    );
    assert!(directory.is_empty());
    assert!(idempotency.is_empty());
}

#[test]
fn identity_user_upsert_rejects_primary_identifier_conflicts_invalid_idp_and_reused_idempotency() {
    let mut directory = IdentityUserDirectory::default();
    let mut idempotency = IdentityUserUpsertIdempotencyLedger::default();
    let mut request = user_request(
        "req_identity_user_first",
        "idem_identity_user_first",
        TENANT_ID,
        USER_ID,
    );
    upsert_identity_user_from_api(&mut directory, &mut idempotency, request.clone())
        .expect("first user upsert succeeds");

    let conflict = user_request(
        "req_identity_user_conflict",
        "idem_identity_user_conflict",
        TENANT_ID,
        "usr_other",
    );
    let conflict_error = upsert_identity_user_from_api(&mut directory, &mut idempotency, conflict)
        .expect_err("primary identifier is unique per tenant");
    assert!(matches!(
        conflict_error,
        IdentityUserUpsertApiError::PrimaryIdentifierConflict { .. }
    ));
    assert_eq!(
        conflict_error.identity_user_upsert_status(),
        IdentityUserUpsertApiStatus::Conflict
    );

    let mut invalid_idp = user_request(
        "req_identity_user_idp",
        "idem_identity_user_idp",
        TENANT_ID,
        "usr_idp",
    );
    invalid_idp.body.identity_provider_id = "provider".to_string();
    assert!(matches!(
        upsert_identity_user_from_api(&mut directory, &mut idempotency, invalid_idp),
        Err(IdentityUserUpsertApiError::InvalidIdentityProvider { .. })
    ));

    request.body.display_name = "Changed".to_string();
    let reused = upsert_identity_user_from_api(&mut directory, &mut idempotency, request)
        .expect_err("same idempotency key with changed body is rejected");
    assert_eq!(
        reused.identity_user_upsert_status(),
        IdentityUserUpsertApiStatus::UnprocessableEntity
    );
    assert_eq!(directory.len(), 1);
    assert_eq!(idempotency.len(), 1);
}

fn user_request(
    request_id: &str,
    idempotency_key: &str,
    tenant_id: &str,
    user_id: &str,
) -> IdentityUserUpsertApiRequest {
    IdentityUserUpsertApiRequest {
        path_tenant_id: tenant_id.to_string(),
        path_user_id: user_id.to_string(),
        boundary: IdentityUserApiBoundaryContext {
            request_id: request_id.to_string(),
            tenant_id: tenant_id.to_string(),
            idempotency_key: idempotency_key.to_string(),
        },
        principal: IdentityUserApiPrincipal {
            tenant_id: tenant_id.to_string(),
            principal_id: "usr_platform_admin".to_string(),
        },
        authorization: IdentityUserApiAuthorization {
            tenant_id: tenant_id.to_string(),
            principal_id: "usr_platform_admin".to_string(),
            decision_id: "authz_identity_user_upsert".to_string(),
            allowed_surfaces: vec![IDENTITY_USER_UPSERT_SURFACE.to_string()],
        },
        body: IdentityUserUpsertRequest {
            tenant_id: tenant_id.to_string(),
            user_id: user_id.to_string(),
            primary_identifier: "admin@kr.example".to_string(),
            display_name: "KR Admin".to_string(),
            roles: vec![IdentityUserRoleRef {
                value: "tenant.admin".to_string(),
            }],
            region_pack: "pack-alpha".to_string(),
            identity_provider_id: "idp_kr_oidc".to_string(),
            external_subject: "oidc://kr.example/admin".to_string(),
        },
    }
}
