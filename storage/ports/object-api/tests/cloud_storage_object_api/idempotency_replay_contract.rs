use super::common::*;

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

#[test]
fn shred_proof_ref_change_yields_idempotency_key_reused_conflict() {
    let mut catalog = catalog_with_active_bucket();
    let mut ledger = CloudStorageObjectPutIdempotencyLedger::default();

    let original = put_request("req-shred-1", "idem-shred");
    put_cloud_storage_object_from_api(&mut catalog, &mut ledger, original.clone())
        .expect("first PUT without shred_proof_ref succeeds");

    let mut drifted = original;
    drifted.boundary.request_id = "req-shred-2".to_string();
    drifted.body.encryption.shred_proof_ref = Some("proof/ten_alpha/shred/001".to_string());

    let err = put_cloud_storage_object_from_api(&mut catalog, &mut ledger, drifted)
        .expect_err("adding shred_proof_ref changes the fingerprint and triggers conflict");

    assert_eq!(
        err,
        CloudStorageObjectApiError::IdempotencyKeyReused {
            idempotency_key: "idem-shred".to_string(),
        },
        "shred_proof_ref must be part of the fingerprint canonical form"
    );
    assert_eq!(
        catalog.objects().count(),
        1,
        "catalog unchanged on conflict"
    );
}
