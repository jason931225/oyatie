use data_classification::DataClass;
use storage_domain::{
    BucketCreate, BucketState, BucketTier, CloudStorageCatalog, EncryptionMode, ObjectLockMode,
    ObjectLockPolicy, ReplicationPolicyCreate, StorageRepo,
};

use super::{
    CLOUD_STORAGE_OBJECT_PUT_SURFACE, CloudStorageObjectApiAuthorization,
    CloudStorageObjectApiError, CloudStorageObjectApiPrincipal,
    CloudStorageObjectEncryptionBindingRequest, CloudStorageObjectMutationBoundaryContext,
    CloudStorageObjectPutApiRequest, CloudStorageObjectPutIdempotencyLedger,
    CloudStorageObjectPutRequest, CloudStorageObjectReplayOutcome,
    put_cloud_storage_object_from_api,
};

// Region must be in the "region-home" family so StrictHomeRegion residency passes.
const BUCKET_ID: &str = "oyatie:cloud:region-home:ten_unit:bucket:unit-assets";
const OBJECT_KEY: &str = "unit/obj.bin";

fn bucket_create() -> BucketCreate {
    BucketCreate {
        resource_id: BUCKET_ID.to_string(),
        tenant_id: "ten_unit".to_string(),
        name: "unit-assets".to_string(),
        region: "region-home".to_string(),
        residency: network_residency_policy::ResidencyClass::StrictHomeRegion,
        tier: BucketTier::Standard,
        replication: ReplicationPolicyCreate::Regional,
        encryption: EncryptionMode::SseKms,
        kms_key: Some("kms/region-home/ten_unit/unit-key".to_string()),
        object_lock: Some(ObjectLockPolicy {
            mode: ObjectLockMode::Compliance,
            retain_until_epoch_seconds: 1_800_000_000,
            legal_hold: false,
        }),
        allowed_data_classes: vec![DataClass::Public, DataClass::PiiIdentifying],
        state: BucketState::Creating,
        created_at_epoch_seconds: 1_700_000_000,
    }
}

fn catalog_with_active_bucket() -> CloudStorageCatalog {
    let mut catalog = CloudStorageCatalog::default();
    catalog
        .create_bucket(bucket_create())
        .expect("unit bucket creates");
    catalog
        .activate_bucket(BUCKET_ID)
        .expect("unit bucket activates");
    catalog
}

fn encryption() -> CloudStorageObjectEncryptionBindingRequest {
    CloudStorageObjectEncryptionBindingRequest {
        kms_key: "kms/region-home/ten_unit/unit-key".to_string(),
        kms_key_version: 1,
        material_ref: "matref/ten_unit/unit/obj".to_string(),
        ciphertext_ref: "ct/ten_unit/unit/obj".to_string(),
        kms_encrypt_event_id: "kmsuse_unit_obj_001".to_string(),
        purpose: "cloud_object_storage".to_string(),
        shred_proof_ref: None,
    }
}

fn make_request(request_id: &str, idempotency_key: &str) -> CloudStorageObjectPutApiRequest {
    CloudStorageObjectPutApiRequest {
        path_bucket_id: BUCKET_ID.to_string(),
        path_object_key: OBJECT_KEY.to_string(),
        boundary: CloudStorageObjectMutationBoundaryContext {
            request_id: request_id.to_string(),
            tenant_id: "ten_unit".to_string(),
            idempotency_key: idempotency_key.to_string(),
        },
        principal: CloudStorageObjectApiPrincipal {
            tenant_id: "ten_unit".to_string(),
            principal_id: "sp_unit".to_string(),
        },
        authorization: CloudStorageObjectApiAuthorization {
            tenant_id: "ten_unit".to_string(),
            principal_id: "sp_unit".to_string(),
            decision_id: "authz_unit_001".to_string(),
            allowed_surfaces: vec![CLOUD_STORAGE_OBJECT_PUT_SURFACE.to_string()],
        },
        body: CloudStorageObjectPutRequest {
            bucket_id: BUCKET_ID.to_string(),
            tenant_id: "ten_unit".to_string(),
            key: OBJECT_KEY.to_string(),
            size_bytes: 16,
            etag: "aabbccddeeff00112233445566778899".to_string(),
            data_class: "PII_IDENTIFYING".to_string(),
            encryption: encryption(),
            stored_at_epoch_seconds: 1_700_000_100,
            last_accessed_at_epoch_seconds: None,
        },
    }
}

#[test]
fn first_put_records_and_returns_created() {
    let mut catalog = catalog_with_active_bucket();
    let mut ledger = CloudStorageObjectPutIdempotencyLedger::default();

    let response = put_cloud_storage_object_from_api(
        &mut catalog,
        &mut ledger,
        make_request("req-u1", "idem-u1"),
    )
    .expect("first PUT succeeds");

    assert_eq!(ledger.len(), 1);
    assert_eq!(catalog.objects().count(), 1);
    assert_eq!(response.data.key, OBJECT_KEY);
    assert_eq!(response.metadata.request_id, "req-u1");
}

#[test]
fn replay_same_fingerprint_no_catalog_mutation() {
    let mut catalog = catalog_with_active_bucket();
    let mut ledger = CloudStorageObjectPutIdempotencyLedger::default();
    let request = make_request("req-u2", "idem-u2");

    let first = put_cloud_storage_object_from_api(&mut catalog, &mut ledger, request.clone())
        .expect("first PUT succeeds");
    let second = put_cloud_storage_object_from_api(&mut catalog, &mut ledger, request)
        .expect("replay succeeds");

    assert_eq!(first, second);
    assert_eq!(
        catalog.objects().count(),
        1,
        "catalog not mutated on replay"
    );
    assert_eq!(ledger.len(), 1, "ledger has exactly one entry");
}

#[test]
fn conflict_different_fingerprint_yields_idempotency_key_reused() {
    let mut catalog = catalog_with_active_bucket();
    let mut ledger = CloudStorageObjectPutIdempotencyLedger::default();
    let original = make_request("req-u3", "idem-u3");
    put_cloud_storage_object_from_api(&mut catalog, &mut ledger, original.clone())
        .expect("first PUT succeeds");

    let mut drifted = original;
    drifted.body.etag = "00000000000000000000000000000000".to_string();
    // etag also lives in path-body binding check as separate field; update body only
    let err = put_cloud_storage_object_from_api(&mut catalog, &mut ledger, drifted)
        .expect_err("different fingerprint yields conflict");

    assert_eq!(
        err,
        CloudStorageObjectApiError::IdempotencyKeyReused {
            idempotency_key: "idem-u3".to_string(),
        }
    );
    assert_eq!(
        catalog.objects().count(),
        1,
        "catalog not mutated on conflict"
    );
}

#[test]
fn peek_reflects_each_state() {
    let mut catalog = catalog_with_active_bucket();
    let mut ledger = CloudStorageObjectPutIdempotencyLedger::default();

    assert!(
        ledger
            .peek(
                "ten_unit",
                "sp_unit",
                CLOUD_STORAGE_OBJECT_PUT_SURFACE,
                "idem-u4"
            )
            .is_none(),
        "peek returns None before recording"
    );

    put_cloud_storage_object_from_api(&mut catalog, &mut ledger, make_request("req-u4", "idem-u4"))
        .expect("first PUT succeeds");

    let entry = ledger
        .peek(
            "ten_unit",
            "sp_unit",
            CLOUD_STORAGE_OBJECT_PUT_SURFACE,
            "idem-u4",
        )
        .expect("peek returns Some after record");
    assert_eq!(entry.idempotency_key, "idem-u4");
    assert!(
        matches!(
            entry.outcome,
            CloudStorageObjectReplayOutcome::Replayed { .. }
        ),
        "outcome is Replayed after successful PUT"
    );

    assert!(
        ledger
            .peek(
                "ten_unit",
                "sp_unit",
                CLOUD_STORAGE_OBJECT_PUT_SURFACE,
                "idem-unknown"
            )
            .is_none(),
        "peek returns None for unknown key"
    );
}
