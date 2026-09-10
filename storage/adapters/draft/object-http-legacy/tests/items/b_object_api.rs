#[test]
fn put_object_api_maps_duplicate_object_to_conflict() {
    let mut catalog = catalog_with_active_bucket();
    let mut ledger = CloudStorageObjectPutIdempotencyLedger::default();
    put_cloud_storage_object_from_api(
        &mut catalog,
        &mut ledger,
        put_request("req-storage-object-dup-1", "idem-storage-object-dup-1"),
    )
    .expect("first object PUT succeeds");

    let error = put_cloud_storage_object_from_api(
        &mut catalog,
        &mut ledger,
        put_request("req-storage-object-dup-2", "idem-storage-object-dup-2"),
    )
    .expect_err("same object through new idempotency key is a conflict");

    assert_eq!(
        error,
        CloudStorageObjectApiError::Storage(CloudStorageError::DuplicateObject)
    );
    assert_eq!(error.object_status_code(), 409);
    assert_eq!(catalog.objects().count(), 1);
}

#[test]
fn put_object_api_maps_bucket_data_class_policy_denial_to_forbidden() {
    let mut catalog = catalog_with_active_bucket();
    let mut ledger = CloudStorageObjectPutIdempotencyLedger::default();
    let mut request = put_request("req-storage-object-class", "idem-storage-object-class");
    request.body.data_class = "PCI".to_string();

    let error = put_cloud_storage_object_from_api(&mut catalog, &mut ledger, request)
        .expect_err("bucket allowed data-class set denies PCI object metadata");

    assert_eq!(
        error,
        CloudStorageObjectApiError::Storage(CloudStorageError::ObjectDataClassDenied)
    );
    assert_eq!(error.object_status_code(), 403);
    assert_eq!(catalog.objects().count(), 0);
}

#[test]
fn put_object_api_maps_wrong_kms_purpose_without_masking() {
    let mut catalog = catalog_with_active_bucket();
    let mut ledger = CloudStorageObjectPutIdempotencyLedger::default();
    let mut request = put_request("req-storage-object-purpose", "idem-storage-object-purpose");
    request.body.encryption.purpose = "cloud_block_storage".to_string();

    let error = put_cloud_storage_object_from_api(&mut catalog, &mut ledger, request)
        .expect_err("object storage accepts only object KMS purpose at kernel boundary");

    assert_eq!(
        error,
        CloudStorageObjectApiError::Storage(CloudStorageError::InvalidKmsPurpose)
    );
    assert_eq!(error.object_status_code(), 400);
    assert_eq!(catalog.objects().count(), 0);
}

#[test]
fn get_object_api_projects_authorized_object_metadata() {
    let mut catalog = catalog_with_active_bucket();
    put_fixture_object(&mut catalog);

    let response = get_cloud_storage_object_from_api(&catalog, get_request(BUCKET_ID, OBJECT_KEY))
        .expect("authorized object GET succeeds");

    assert_eq!(response.metadata.request_id, "req-storage-object-get");
    assert_eq!(response.data.bucket_id, BUCKET_ID);
    assert_eq!(response.data.key, OBJECT_KEY);
    assert_eq!(
        response.data.encryption.ciphertext_ref,
        "ct/ten_alpha/object/report"
    );
    assert_eq!(response.data.schema_version, 1);
}

#[test]
fn get_object_api_rejects_authorization_before_existence_lookup() {
    let catalog = catalog_with_active_bucket();
    let mut request = get_request(BUCKET_ID, "workspace/missing.pdf");
    request.authorization.allowed_surfaces = vec![CLOUD_STORAGE_OBJECT_PUT_SURFACE.to_string()];

    let error = get_cloud_storage_object_from_api(&catalog, request)
        .expect_err("authorization denial must win over object existence checks");

    assert_eq!(
        error,
        CloudStorageObjectApiError::AuthorizationDenied {
            surface: CLOUD_STORAGE_OBJECT_GET_SURFACE.to_string(),
        }
    );
    assert_eq!(error.object_status_code(), 403);
}

#[test]
fn get_object_api_maps_not_found_and_tenant_drift_explicitly() {
    let catalog = catalog_with_active_bucket();
    let missing = get_cloud_storage_object_from_api(
        &catalog,
        get_request(BUCKET_ID, "workspace/missing.pdf"),
    )
    .expect_err("missing object maps to not found");
    assert_eq!(missing.object_status_code(), 404);
    assert!(matches!(
        missing,
        CloudStorageObjectApiError::ObjectNotFound { .. }
    ));

    let tenant_drift = get_cloud_storage_object_from_api(
        &catalog,
        get_request(
            "oyatie:cloud:region-home:ten_other:bucket:tenant-assets",
            OBJECT_KEY,
        ),
    )
    .expect_err("bucket tenant drift is rejected before catalog lookup");
    assert_eq!(tenant_drift.object_status_code(), 403);
    assert!(matches!(
        tenant_drift,
        CloudStorageObjectApiError::TenantMismatch { .. }
    ));
}


#[test]
fn replayed_response_preserves_first_request_id_not_second_callers() {
    let mut catalog = catalog_with_active_bucket();
    let mut ledger = CloudStorageObjectPutIdempotencyLedger::default();

    let first = put_cloud_storage_object_from_api(
        &mut catalog,
        &mut ledger,
        put_request("req-replay-first", "idem-replay-id-check"),
    )
    .expect("first PUT succeeds");
    assert_eq!(first.metadata.request_id, "req-replay-first");

    let replayed = put_cloud_storage_object_from_api(
        &mut catalog,
        &mut ledger,
        put_request("req-replay-second", "idem-replay-id-check"),
    )
    .expect("replay with same fingerprint succeeds");

    assert_eq!(
        replayed.metadata.request_id, "req-replay-first",
        "replay must return the stored response verbatim (S3/GCS deterministic-replay semantics)"
    );
    assert_eq!(first, replayed);
    assert_eq!(catalog.objects().count(), 1);
}

#[test]
fn composite_key_isolates_principal_scope() {
    let mut catalog = catalog_with_active_bucket();
    let mut ledger = CloudStorageObjectPutIdempotencyLedger::default();

    let alpha_request = CloudStorageObjectPutApiRequest {
        path_bucket_id: BUCKET_ID.to_string(),
        path_object_key: "workspace/alpha.pdf".to_string(),
        boundary: CloudStorageObjectMutationBoundaryContext {
            request_id: "req-alpha".to_string(),
            tenant_id: "ten_alpha".to_string(),
            idempotency_key: "idem-shared-key".to_string(),
        },
        principal: principal_for("sp_alpha"),
        authorization: authorization_for("sp_alpha", &[CLOUD_STORAGE_OBJECT_PUT_SURFACE]),
        body: put_body(BUCKET_ID, "workspace/alpha.pdf"),
    };

    let beta_request = CloudStorageObjectPutApiRequest {
        path_bucket_id: BUCKET_ID.to_string(),
        path_object_key: "workspace/beta.pdf".to_string(),
        boundary: CloudStorageObjectMutationBoundaryContext {
            request_id: "req-beta".to_string(),
            tenant_id: "ten_alpha".to_string(),
            idempotency_key: "idem-shared-key".to_string(),
        },
        principal: principal_for("sp_beta"),
        authorization: authorization_for("sp_beta", &[CLOUD_STORAGE_OBJECT_PUT_SURFACE]),
        body: put_body(BUCKET_ID, "workspace/beta.pdf"),
    };

    put_cloud_storage_object_from_api(&mut catalog, &mut ledger, alpha_request)
        .expect("sp_alpha PUT succeeds");
    put_cloud_storage_object_from_api(&mut catalog, &mut ledger, beta_request)
        .expect("sp_beta PUT with same idempotency key string but different principal succeeds");

    assert_eq!(
        ledger.len(),
        2,
        "distinct (principal, surface, idempotency_key) composite keys are independent entries"
    );
    assert_eq!(catalog.objects().count(), 2);

    let alpha_entry = ledger
        .peek(
            "ten_alpha",
            "sp_alpha",
            CLOUD_STORAGE_OBJECT_PUT_SURFACE,
            "idem-shared-key",
        )
        .expect("sp_alpha peek returns Some");
    let beta_entry = ledger
        .peek(
            "ten_alpha",
            "sp_beta",
            CLOUD_STORAGE_OBJECT_PUT_SURFACE,
            "idem-shared-key",
        )
        .expect("sp_beta peek returns Some");

    assert!(matches!(
        alpha_entry.outcome,
        CloudStorageObjectReplayOutcome::Replayed { .. }
    ));
    assert!(matches!(
        beta_entry.outcome,
        CloudStorageObjectReplayOutcome::Replayed { .. }
    ));
}
