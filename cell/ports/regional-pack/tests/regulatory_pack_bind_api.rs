#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cell_regional_pack_api::{
    REGULATORY_PACK_BIND_OPENAPI_CONTRACT, REGULATORY_PACK_BIND_SURFACE,
    RegulatoryPackApiAuthorization, RegulatoryPackApiBoundaryContext, RegulatoryPackApiPrincipal,
    RegulatoryPackBindApiError, RegulatoryPackBindApiRequest, RegulatoryPackBindApiStatus,
    RegulatoryPackBindIdempotencyLedger, RegulatoryPackBindRequest, RegulatoryPackBindingDirectory,
    RegulatoryPackBindingPackRef, RegulatoryPackControlRef, bind_regulatory_pack_from_api,
};

const REQUEST_ID: &str = "req_regulatory_pack_001";
const IDEMPOTENCY_KEY: &str = "idem_regulatory_pack_001";
const TENANT_ID: &str = "ten_regulatory_pack";
const PRIMARY_PACK_ID: &str = "pack-alpha";

#[test]
fn regulatory_pack_bind_contract_runtime_constants_are_covered() {
    assert_eq!(REGULATORY_PACK_BIND_SURFACE, "regulatory-pack.bind");
    assert_eq!(
        REGULATORY_PACK_BIND_OPENAPI_CONTRACT,
        "contracts/openapi/platform/platform-regulatory-pack-v1.yaml"
    );
    assert_eq!(RegulatoryPackBindApiStatus::Created.code(), 201);
    assert_eq!(RegulatoryPackBindApiStatus::BadRequest.code(), 400);
    assert_eq!(RegulatoryPackBindApiStatus::Unauthorized.code(), 401);
    assert_eq!(RegulatoryPackBindApiStatus::Forbidden.code(), 403);
    assert_eq!(RegulatoryPackBindApiStatus::Conflict.code(), 409);
    assert_eq!(RegulatoryPackBindApiStatus::UnprocessableEntity.code(), 422);
}

#[test]
fn regulatory_pack_bind_binds_once_and_replays_same_idempotent_result() {
    let mut directory = RegulatoryPackBindingDirectory::default();
    let mut idempotency = RegulatoryPackBindIdempotencyLedger::default();
    let request = bind_request(REQUEST_ID, IDEMPOTENCY_KEY, TENANT_ID);

    let first = bind_regulatory_pack_from_api(&mut directory, &mut idempotency, request.clone())
        .expect("first regulatory pack bind succeeds");
    let second = bind_regulatory_pack_from_api(&mut directory, &mut idempotency, request)
        .expect("same regulatory pack bind request replays");

    assert_eq!(first, second);
    assert_eq!(directory.len(), 1);
    assert_eq!(idempotency.len(), 1);
    assert_eq!(first.data.tenant_id, TENANT_ID);
    assert_eq!(first.data.primary_pack_id, PRIMARY_PACK_ID);
    assert_eq!(first.data.home_region, "region-home");
    assert_eq!(first.data.residency_class, "strict_home_region");
    assert_eq!(first.data.pack_refs.len(), 1);
    assert_eq!(first.data.schema_version, 1);
    assert_eq!(first.metadata.request_id, REQUEST_ID);
    assert!(directory.get(TENANT_ID).is_some());
}

#[test]
fn regulatory_pack_bind_supports_multi_pack_initial_binding() {
    let mut directory = RegulatoryPackBindingDirectory::default();
    let mut idempotency = RegulatoryPackBindIdempotencyLedger::default();
    let mut request = bind_request(
        "req_regulatory_pack_multi",
        "idem_regulatory_pack_multi",
        TENANT_ID,
    );
    request.body.pack_refs.push(RegulatoryPackBindingPackRef {
        pack_id: "pack-gamma".to_string(),
        region: "failover-region".to_string(),
        residency_class: "global".to_string(),
        controls: vec![
            RegulatoryPackControlRef {
                value: "FTC".to_string(),
            },
            RegulatoryPackControlRef {
                value: "CCPA_CPRA".to_string(),
            },
        ],
    });

    let response = bind_regulatory_pack_from_api(&mut directory, &mut idempotency, request)
        .expect("multi-pack tenant binding succeeds when the primary pack is included");

    assert_eq!(response.data.pack_refs.len(), 2);
    assert_eq!(response.data.pack_refs[1].pack_id, "pack-gamma");
    assert_eq!(response.data.primary_pack_id, PRIMARY_PACK_ID);
    assert_eq!(directory.len(), 1);
}

#[test]
fn regulatory_pack_bind_rejects_path_body_and_primary_pack_drift() {
    let mut directory = RegulatoryPackBindingDirectory::default();
    let mut idempotency = RegulatoryPackBindIdempotencyLedger::default();
    let mut tenant_drift = bind_request(
        "req_regulatory_pack_tenant_drift",
        "idem_regulatory_pack_tenant_drift",
        TENANT_ID,
    );
    tenant_drift.body.tenant_id = "ten_other".to_string();

    let drift_error = bind_regulatory_pack_from_api(&mut directory, &mut idempotency, tenant_drift)
        .expect_err("body tenant drift is rejected before mutation");
    assert!(matches!(
        drift_error,
        RegulatoryPackBindApiError::TenantPathBodyMismatch { .. }
    ));
    assert_eq!(drift_error.regulatory_pack_bind_status_code(), 400);

    let mut primary_drift = bind_request(
        "req_regulatory_pack_primary_drift",
        "idem_regulatory_pack_primary_drift",
        TENANT_ID,
    );
    primary_drift.body.primary_pack_id = "pack-secondary".to_string();
    let primary_error =
        bind_regulatory_pack_from_api(&mut directory, &mut idempotency, primary_drift)
            .expect_err("primary pack must be present in pack_refs");
    assert!(matches!(
        primary_error,
        RegulatoryPackBindApiError::PrimaryPackMissing { .. }
    ));
    assert!(directory.is_empty());
}

#[test]
fn regulatory_pack_bind_separates_authentication_and_authorization_errors() {
    let mut directory = RegulatoryPackBindingDirectory::default();
    let mut idempotency = RegulatoryPackBindIdempotencyLedger::default();
    let mut unauthenticated = bind_request(
        "req_regulatory_pack_authn",
        "idem_regulatory_pack_authn",
        TENANT_ID,
    );
    unauthenticated.principal.principal_id.clear();

    let authn_error =
        bind_regulatory_pack_from_api(&mut directory, &mut idempotency, unauthenticated)
            .expect_err("missing principal is authentication failure");
    assert_eq!(
        authn_error.regulatory_pack_bind_status(),
        RegulatoryPackBindApiStatus::Unauthorized
    );

    let mut denied = bind_request(
        "req_regulatory_pack_authz",
        "idem_regulatory_pack_authz",
        TENANT_ID,
    );
    denied.authorization.allowed_surfaces = vec!["tenant.create".to_string()];
    let authz_error = bind_regulatory_pack_from_api(&mut directory, &mut idempotency, denied)
        .expect_err("missing regulatory-pack.bind grant is authorization failure");
    assert!(matches!(
        authz_error,
        RegulatoryPackBindApiError::AuthorizationDenied { ref surface }
            if surface == REGULATORY_PACK_BIND_SURFACE
    ));
    assert_eq!(
        authz_error.regulatory_pack_bind_status(),
        RegulatoryPackBindApiStatus::Forbidden
    );
    assert!(directory.is_empty());
}

#[test]
fn regulatory_pack_bind_maps_invalid_pack_residency_duplicate_and_idempotency_errors() {
    let mut directory = RegulatoryPackBindingDirectory::default();
    let mut idempotency = RegulatoryPackBindIdempotencyLedger::default();
    bind_regulatory_pack_from_api(
        &mut directory,
        &mut idempotency,
        bind_request(
            "req_regulatory_pack_first",
            "idem_regulatory_pack_first",
            TENANT_ID,
        ),
    )
    .expect("initial regulatory pack bind succeeds");

    let duplicate = bind_regulatory_pack_from_api(
        &mut directory,
        &mut idempotency,
        bind_request(
            "req_regulatory_pack_duplicate",
            "idem_regulatory_pack_duplicate",
            TENANT_ID,
        ),
    )
    .expect_err("a tenant residency/regulatory pack binding is immutable");
    assert!(matches!(
        duplicate,
        RegulatoryPackBindApiError::DuplicateBinding { .. }
            | RegulatoryPackBindApiError::Residency(_)
    ));
    assert_eq!(
        duplicate.regulatory_pack_bind_status(),
        RegulatoryPackBindApiStatus::Conflict
    );

    let mut invalid_residency = bind_request(
        "req_regulatory_pack_bad_residency",
        "idem_regulatory_pack_bad_residency",
        "ten_bad_residency",
    );
    invalid_residency.body.residency_class = "moon_base".to_string();
    assert!(matches!(
        bind_regulatory_pack_from_api(&mut directory, &mut idempotency, invalid_residency),
        Err(RegulatoryPackBindApiError::InvalidResidencyClass { .. })
    ));

    let mut invalid_pack = bind_request(
        "req_regulatory_pack_bad_pack",
        "idem_regulatory_pack_bad_pack",
        "ten_bad_pack",
    );
    invalid_pack.body.primary_pack_id = "bad".to_string();
    invalid_pack.body.pack_refs[0].pack_id = "bad".to_string();
    assert!(matches!(
        bind_regulatory_pack_from_api(&mut directory, &mut idempotency, invalid_pack),
        Err(RegulatoryPackBindApiError::RegionalPack(_))
    ));

    let mut reused = bind_request(
        "req_regulatory_pack_reused",
        "idem_regulatory_pack_reused",
        "ten_reused",
    );
    bind_regulatory_pack_from_api(&mut directory, &mut idempotency, reused.clone())
        .expect("first idempotent binding succeeds");
    reused.body.evidence_ref = "evidence/changed".to_string();
    let error = bind_regulatory_pack_from_api(&mut directory, &mut idempotency, reused)
        .expect_err("same idempotency key with changed fingerprint fails");
    assert_eq!(
        error,
        RegulatoryPackBindApiError::IdempotencyKeyReused {
            idempotency_key: "idem_regulatory_pack_reused".to_string()
        }
    );
    assert_eq!(
        error.regulatory_pack_bind_status(),
        RegulatoryPackBindApiStatus::UnprocessableEntity
    );
}

fn bind_request(
    request_id: &str,
    idempotency_key: &str,
    tenant_id: &str,
) -> RegulatoryPackBindApiRequest {
    RegulatoryPackBindApiRequest {
        path_tenant_id: tenant_id.to_string(),
        boundary: RegulatoryPackApiBoundaryContext {
            request_id: request_id.to_string(),
            tenant_id: tenant_id.to_string(),
            idempotency_key: idempotency_key.to_string(),
        },
        principal: RegulatoryPackApiPrincipal {
            tenant_id: tenant_id.to_string(),
            principal_id: "usr_regulatory_operator".to_string(),
        },
        authorization: RegulatoryPackApiAuthorization {
            tenant_id: tenant_id.to_string(),
            principal_id: "usr_regulatory_operator".to_string(),
            decision_id: "authz_regulatory_pack_bind".to_string(),
            allowed_surfaces: vec![REGULATORY_PACK_BIND_SURFACE.to_string()],
        },
        body: RegulatoryPackBindRequest {
            tenant_id: tenant_id.to_string(),
            primary_pack_id: PRIMARY_PACK_ID.to_string(),
            home_region: "region-home".to_string(),
            cell_group_ref: "cellgrp_kr_seoul_001".to_string(),
            residency_class: "strict_home_region".to_string(),
            evidence_ref: "evidence/regulatory-pack/ten_regulatory_pack".to_string(),
            bound_at_epoch_seconds: 1_800_000_000,
            pack_refs: vec![RegulatoryPackBindingPackRef {
                pack_id: PRIMARY_PACK_ID.to_string(),
                region: "region-home".to_string(),
                residency_class: "strict_home_region".to_string(),
                controls: vec![
                    RegulatoryPackControlRef {
                        value: "CONTROL-ALPHA".to_string(),
                    },
                    RegulatoryPackControlRef {
                        value: "KISA".to_string(),
                    },
                ],
            }],
        },
    }
}
