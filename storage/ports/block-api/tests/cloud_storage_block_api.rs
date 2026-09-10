#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use storage_block_api::{
    CloudStorageBlockApiAuthorization, CloudStorageBlockApiBoundaryContext,
    CloudStorageBlockApiError, CloudStorageBlockApiPrincipal, CloudStorageBlockCreateApiStatus,
    CloudStorageBlockCreateIdempotencyLedger, CloudStorageBlockVolumeCreateApiRequest,
    CloudStorageBlockVolumeCreateRequest, CloudStorageBlockVolumePerformance,
    STORAGE_BLOCK_CREATE_SURFACE, create_cloud_storage_block_volume_from_api,
};
use storage_domain::{CloudStorageCatalog, CloudStorageError};

const VOLUME_ID: &str = "oyatie:cloud:region-home:ten_alpha:volume:db-primary";

fn boundary_for(request_id: &str, idempotency_key: &str) -> CloudStorageBlockApiBoundaryContext {
    CloudStorageBlockApiBoundaryContext {
        request_id: request_id.to_string(),
        tenant_id: "ten_alpha".to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}

fn principal_for(principal_id: &str) -> CloudStorageBlockApiPrincipal {
    CloudStorageBlockApiPrincipal {
        tenant_id: "ten_alpha".to_string(),
        principal_id: principal_id.to_string(),
    }
}

fn authorization_for(principal_id: &str, surfaces: &[&str]) -> CloudStorageBlockApiAuthorization {
    CloudStorageBlockApiAuthorization {
        tenant_id: "ten_alpha".to_string(),
        principal_id: principal_id.to_string(),
        decision_id: format!("authz_decision_{principal_id}"),
        allowed_surfaces: surfaces
            .iter()
            .map(|surface| (*surface).to_string())
            .collect(),
    }
}

fn create_body(resource_id: &str) -> CloudStorageBlockVolumeCreateRequest {
    CloudStorageBlockVolumeCreateRequest {
        resource_id: resource_id.to_string(),
        tenant_id: "ten_alpha".to_string(),
        name: "db-primary".to_string(),
        region: "region-home".to_string(),
        az: "region-home-a".to_string(),
        cell_id: "cell-region-home-a-001".to_string(),
        residency: "strict_home_region".to_string(),
        tier: "provisioned_iops_ssd".to_string(),
        size_gib: 512,
        performance: CloudStorageBlockVolumePerformance {
            iops: 12_000,
            throughput_mbps: 750,
        },
        encryption: "byok".to_string(),
        kms_key: Some("byok/region-home/ten_alpha/db-key".to_string()),
        data_class: "PII_IDENTIFYING".to_string(),
        created_at_epoch_seconds: 1_700_000_000,
    }
}

fn create_request(
    request_id: &str,
    idempotency_key: &str,
) -> CloudStorageBlockVolumeCreateApiRequest {
    CloudStorageBlockVolumeCreateApiRequest {
        path_volume_id: VOLUME_ID.to_string(),
        boundary: boundary_for(request_id, idempotency_key),
        principal: principal_for("sp_storage_admin"),
        authorization: authorization_for("sp_storage_admin", &[STORAGE_BLOCK_CREATE_SURFACE]),
        body: create_body(VOLUME_ID),
    }
}

#[test]
fn openapi_runtime_binding_contracts_are_covered() {
    assert_eq!(STORAGE_BLOCK_CREATE_SURFACE, "storage.block.create");
    assert_eq!(CloudStorageBlockCreateApiStatus::Created.code(), 201);
    assert_eq!(CloudStorageBlockCreateApiStatus::BadRequest.code(), 400);
    assert_eq!(CloudStorageBlockCreateApiStatus::Forbidden.code(), 403);
    assert_eq!(CloudStorageBlockCreateApiStatus::NotFound.code(), 404);
    assert_eq!(CloudStorageBlockCreateApiStatus::Conflict.code(), 409);
    assert_eq!(
        CloudStorageBlockCreateApiStatus::UnprocessableEntity.code(),
        422
    );
}

#[test]
fn block_create_api_creates_volume_once_and_replays_same_idempotent_result() {
    let mut catalog = CloudStorageCatalog::default();
    let mut ledger = CloudStorageBlockCreateIdempotencyLedger::default();
    let request = create_request("req-storage-block-create", "idem-storage-block-create");

    let first =
        create_cloud_storage_block_volume_from_api(&mut catalog, &mut ledger, request.clone())
            .expect("authorized block volume create succeeds");
    let second = create_cloud_storage_block_volume_from_api(&mut catalog, &mut ledger, request)
        .expect("same idempotency fingerprint replays");

    assert_eq!(first, second);
    assert_eq!(ledger.len(), 1);
    assert_eq!(catalog.volumes().count(), 1);
    assert_eq!(first.metadata.request_id, "req-storage-block-create");
    assert_eq!(first.data.resource_id, VOLUME_ID);
    assert_eq!(first.data.tenant_id, "ten_alpha");
    assert_eq!(first.data.region, "region-home");
    assert_eq!(first.data.az, "region-home-a");
    assert_eq!(first.data.residency, "strict_home_region");
    assert_eq!(first.data.tier, "provisioned_iops_ssd");
    assert_eq!(first.data.size_gib, 512);
    assert_eq!(first.data.performance.iops, 12_000);
    assert_eq!(first.data.encryption, "byok");
    assert_eq!(
        first.data.kms_key.as_deref(),
        Some("byok/region-home/ten_alpha/db-key")
    );
    assert_eq!(first.data.data_class, "PII_IDENTIFYING");
    assert_eq!(first.data.state, "creating");
    assert_eq!(first.data.schema_version, 1);
}

#[test]
fn block_create_api_rejects_path_body_volume_drift_before_catalog_mutation() {
    let mut catalog = CloudStorageCatalog::default();
    let mut ledger = CloudStorageBlockCreateIdempotencyLedger::default();
    let mut request = create_request("req-storage-block-drift", "idem-storage-block-drift");
    request.body.resource_id = "oyatie:cloud:region-home:ten_alpha:volume:other".to_string();

    let error = create_cloud_storage_block_volume_from_api(&mut catalog, &mut ledger, request)
        .expect_err("path/body volume drift is rejected");

    assert_eq!(
        error,
        CloudStorageBlockApiError::VolumeIdMismatch {
            path_volume_id: VOLUME_ID.to_string(),
            body_resource_id: "oyatie:cloud:region-home:ten_alpha:volume:other".to_string(),
        }
    );
    assert_eq!(error.block_create_status_code(), 400);
    assert!(ledger.is_empty());
    assert_eq!(catalog.volumes().count(), 0);
}

#[test]
fn block_create_api_rejects_required_header_and_tenant_drift_before_ledger() {
    let mut catalog = CloudStorageCatalog::default();
    let mut ledger = CloudStorageBlockCreateIdempotencyLedger::default();
    let mut empty_request = create_request(" ", "idem-storage-block-empty-header");
    assert_eq!(
        create_cloud_storage_block_volume_from_api(
            &mut catalog,
            &mut ledger,
            empty_request.clone()
        ),
        Err(CloudStorageBlockApiError::EmptyRequestId)
    );

    empty_request.boundary.request_id = "req-storage-block-tenant-drift".to_string();
    empty_request.boundary.tenant_id = "ten_other".to_string();
    assert_eq!(
        create_cloud_storage_block_volume_from_api(&mut catalog, &mut ledger, empty_request),
        Err(CloudStorageBlockApiError::TenantMismatch {
            header_tenant_id: "ten_other".to_string(),
            principal_tenant_id: "ten_alpha".to_string(),
            body_tenant_id: "ten_alpha".to_string(),
        })
    );
    assert!(ledger.is_empty());
    assert_eq!(catalog.volumes().count(), 0);
}

#[test]
fn block_create_api_rejects_unauthorized_same_tenant_principal_before_ledger() {
    let mut catalog = CloudStorageCatalog::default();
    let mut ledger = CloudStorageBlockCreateIdempotencyLedger::default();
    let mut request = create_request("req-storage-block-authz", "idem-storage-block-authz");
    request.authorization.allowed_surfaces = vec!["cloud.storage.object.put".to_string()];

    let error = create_cloud_storage_block_volume_from_api(&mut catalog, &mut ledger, request)
        .expect_err("authorization decision does not allow block create");

    assert_eq!(
        error,
        CloudStorageBlockApiError::AuthorizationDenied {
            surface: STORAGE_BLOCK_CREATE_SURFACE.to_string(),
        }
    );
    assert_eq!(error.block_create_status_code(), 403);
    assert!(ledger.is_empty());
    assert_eq!(catalog.volumes().count(), 0);
}

#[test]
fn block_create_api_rejects_reused_idempotency_key_with_new_fingerprint() {
    let mut catalog = CloudStorageCatalog::default();
    let mut ledger = CloudStorageBlockCreateIdempotencyLedger::default();
    let request = create_request("req-storage-block-idem", "idem-storage-block-idem");
    create_cloud_storage_block_volume_from_api(&mut catalog, &mut ledger, request.clone())
        .expect("initial create succeeds");

    let mut drifted = request;
    drifted.body.size_gib = 1_024;
    assert_eq!(
        create_cloud_storage_block_volume_from_api(&mut catalog, &mut ledger, drifted),
        Err(CloudStorageBlockApiError::IdempotencyKeyReused {
            idempotency_key: "idem-storage-block-idem".to_string(),
        })
    );
    assert_eq!(ledger.len(), 1);
    assert_eq!(catalog.volumes().count(), 1);
}

#[test]
fn block_create_api_maps_duplicate_volume_to_conflict() {
    let mut catalog = CloudStorageCatalog::default();
    let mut ledger = CloudStorageBlockCreateIdempotencyLedger::default();
    create_cloud_storage_block_volume_from_api(
        &mut catalog,
        &mut ledger,
        create_request("req-storage-block-dup-1", "idem-storage-block-dup-1"),
    )
    .expect("first volume create succeeds");

    let error = create_cloud_storage_block_volume_from_api(
        &mut catalog,
        &mut ledger,
        create_request("req-storage-block-dup-2", "idem-storage-block-dup-2"),
    )
    .expect_err("same volume id through new idempotency key is a conflict");

    assert_eq!(
        error,
        CloudStorageBlockApiError::Storage(CloudStorageError::DuplicateVolume)
    );
    assert_eq!(error.block_create_status_code(), 409);
    assert_eq!(catalog.volumes().count(), 1);
}

#[test]
fn block_create_api_maps_invalid_data_class_without_catalog_mutation() {
    let mut catalog = CloudStorageCatalog::default();
    let mut ledger = CloudStorageBlockCreateIdempotencyLedger::default();
    let mut request = create_request("req-storage-block-class", "idem-storage-block-class");
    request.body.data_class = "SECRET".to_string();

    let error = create_cloud_storage_block_volume_from_api(&mut catalog, &mut ledger, request)
        .expect_err("unknown operational marker is not an API data class");

    assert_eq!(
        error,
        CloudStorageBlockApiError::InvalidDataClassLabel {
            data_class: "SECRET".to_string(),
        }
    );
    assert_eq!(error.block_create_status_code(), 400);
    assert_eq!(catalog.volumes().count(), 0);
}

#[test]
fn block_create_api_maps_invalid_tier_without_catalog_mutation() {
    let mut catalog = CloudStorageCatalog::default();
    let mut ledger = CloudStorageBlockCreateIdempotencyLedger::default();
    let mut request = create_request("req-storage-block-tier", "idem-storage-block-tier");
    request.body.tier = "magnetic".to_string();

    let error = create_cloud_storage_block_volume_from_api(&mut catalog, &mut ledger, request)
        .expect_err("unknown tier is rejected before storage mutation");

    assert_eq!(
        error,
        CloudStorageBlockApiError::InvalidVolumeTierLabel {
            tier: "magnetic".to_string(),
        }
    );
    assert_eq!(error.block_create_status_code(), 400);
    assert_eq!(catalog.volumes().count(), 0);
}

#[test]
fn block_create_api_maps_kms_key_region_mismatch_to_forbidden() {
    let mut catalog = CloudStorageCatalog::default();
    let mut ledger = CloudStorageBlockCreateIdempotencyLedger::default();
    let mut request = create_request("req-storage-block-kms", "idem-storage-block-kms");
    request.body.kms_key = Some("byok/failover-region/ten_alpha/db-key".to_string());

    let error = create_cloud_storage_block_volume_from_api(&mut catalog, &mut ledger, request)
        .expect_err("cross-region BYOK binding is denied");

    assert_eq!(
        error,
        CloudStorageBlockApiError::Storage(CloudStorageError::KmsKeyRegionMismatch)
    );
    assert_eq!(error.block_create_status_code(), 403);
    assert_eq!(catalog.volumes().count(), 0);
}
