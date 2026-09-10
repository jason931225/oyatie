fn object_put_input(
    body: CloudStorageObjectPutRequest,
) -> Result<ObjectCreate, CloudStorageObjectApiError> {
    Ok(ObjectCreate {
        bucket_id: body.bucket_id,
        tenant_id: body.tenant_id,
        key: body.key,
        size_bytes: body.size_bytes,
        etag: body.etag,
        data_class: parse_api_data_class(body.data_class)?,
        encryption: ObjectEncryptionBindingCreate {
            kms_key: body.encryption.kms_key,
            kms_key_version: body.encryption.kms_key_version,
            material_ref: body.encryption.material_ref,
            ciphertext_ref: body.encryption.ciphertext_ref,
            kms_encrypt_event_id: body.encryption.kms_encrypt_event_id,
            purpose: parse_api_purpose(body.encryption.purpose)?,
            shred_proof_ref: body.encryption.shred_proof_ref,
        },
        stored_at_epoch_seconds: body.stored_at_epoch_seconds,
        last_accessed_at_epoch_seconds: body.last_accessed_at_epoch_seconds,
    })
}

fn parse_api_data_class(label: String) -> Result<DataClass, CloudStorageObjectApiError> {
    parse_data_class_label(&label)
        .ok_or(CloudStorageObjectApiError::InvalidDataClassLabel { data_class: label })
}

fn parse_api_purpose(label: String) -> Result<KmsPurpose, CloudStorageObjectApiError> {
    match label.as_str() {
        "cloud_object_storage" => Ok(KmsPurpose::CloudObjectStorage),
        "cloud_block_storage" => Ok(KmsPurpose::CloudBlockStorage),
        "cloud_file_storage" => Ok(KmsPurpose::CloudFileStorage),
        "cloud_archive_storage" => Ok(KmsPurpose::CloudArchiveStorage),
        "workspace_drive_object" => Ok(KmsPurpose::WorkspaceDriveObject),
        "workspace_recording" => Ok(KmsPurpose::WorkspaceRecording),
        "secret_provider" => Ok(KmsPurpose::SecretProvider),
        "cross_region_replication" => Ok(KmsPurpose::CrossRegionReplication),
        "database_backup" => Ok(KmsPurpose::DatabaseBackup),
        _ => Err(CloudStorageObjectApiError::InvalidKmsPurposeLabel { purpose: label }),
    }
}

fn purpose_label(purpose: KmsPurpose) -> &'static str {
    match purpose {
        KmsPurpose::CloudObjectStorage => "cloud_object_storage",
        KmsPurpose::CloudBlockStorage => "cloud_block_storage",
        KmsPurpose::CloudFileStorage => "cloud_file_storage",
        KmsPurpose::CloudArchiveStorage => "cloud_archive_storage",
        KmsPurpose::WorkspaceDriveObject => "workspace_drive_object",
        KmsPurpose::WorkspaceRecording => "workspace_recording",
        KmsPurpose::SecretProvider => "secret_provider",
        KmsPurpose::CrossRegionReplication => "cross_region_replication",
        KmsPurpose::DatabaseBackup => "database_backup",
    }
}

fn replay_outcome_for(
    entry: &CloudStorageObjectPutLedgerEntry,
    presented_fingerprint: &CloudStorageObjectRequestFingerprint,
    idempotency_key: &str,
) -> CloudStorageObjectReplayOutcome {
    if entry.fingerprint == *presented_fingerprint {
        match &entry.result {
            Ok(response) => CloudStorageObjectReplayOutcome::Replayed {
                response: Box::new(response.clone()),
            },
            Err(_) => CloudStorageObjectReplayOutcome::Conflict {
                idempotency_key: idempotency_key.to_string(),
            },
        }
    } else {
        CloudStorageObjectReplayOutcome::Conflict {
            idempotency_key: idempotency_key.to_string(),
        }
    }
}

fn idempotency_key_for(
    boundary: &CloudStorageObjectMutationBoundaryContext,
    principal: &CloudStorageObjectApiPrincipal,
    surface: &str,
) -> CloudStorageObjectIdempotencyLedgerKey {
    CloudStorageObjectIdempotencyLedgerKey {
        tenant_id: boundary.tenant_id.clone(),
        principal_id: principal.principal_id.clone(),
        surface: surface.to_string(),
        idempotency_key: boundary.idempotency_key.clone(),
    }
}

fn put_fingerprint_for(
    request: &CloudStorageObjectPutApiRequest,
) -> CloudStorageObjectRequestFingerprint {
    CloudStorageObjectRequestFingerprint {
        canonical: [
            format!("path.bucket_id={}", request.path_bucket_id),
            format!("path.object_key={}", request.path_object_key),
            format!("header.tenant_id={}", request.boundary.tenant_id),
            format!("principal.tenant_id={}", request.principal.tenant_id),
            format!("principal.principal_id={}", request.principal.principal_id),
            format!(
                "authorization.tenant_id={}",
                request.authorization.tenant_id
            ),
            format!(
                "authorization.principal_id={}",
                request.authorization.principal_id
            ),
            format!(
                "authorization.decision_id={}",
                request.authorization.decision_id
            ),
            format!(
                "authorization.allowed_surfaces={}",
                request.authorization.allowed_surfaces.join(",")
            ),
            format!("body.bucket_id={}", request.body.bucket_id),
            format!("body.tenant_id={}", request.body.tenant_id),
            format!("body.key={}", request.body.key),
            format!("body.size_bytes={}", request.body.size_bytes),
            format!("body.etag={}", request.body.etag),
            format!("body.data_class={}", request.body.data_class),
            format!(
                "body.encryption.kms_key={}",
                request.body.encryption.kms_key
            ),
            format!(
                "body.encryption.kms_key_version={}",
                request.body.encryption.kms_key_version
            ),
            format!(
                "body.encryption.material_ref={}",
                request.body.encryption.material_ref
            ),
            format!(
                "body.encryption.ciphertext_ref={}",
                request.body.encryption.ciphertext_ref
            ),
            format!(
                "body.encryption.kms_encrypt_event_id={}",
                request.body.encryption.kms_encrypt_event_id
            ),
            format!(
                "body.encryption.purpose={}",
                request.body.encryption.purpose
            ),
            format!(
                "body.encryption.shred_proof_ref={:?}",
                request.body.encryption.shred_proof_ref
            ),
            format!(
                "body.stored_at_epoch_seconds={}",
                request.body.stored_at_epoch_seconds
            ),
            format!(
                "body.last_accessed_at_epoch_seconds={:?}",
                request.body.last_accessed_at_epoch_seconds
            ),
        ]
        .join("|"),
    }
}

fn object_record(object: StoredObject) -> CloudStorageObjectRecord {
    CloudStorageObjectRecord {
        bucket_id: object.bucket_id.value.value,
        tenant_id: object.tenant_id.value,
        key: object.key.value.value,
        size_bytes: object.size_bytes.value,
        etag: object.etag.value.value,
        data_class: object.data_class.value.label().to_string(),
        encryption: CloudStorageObjectEncryptionBindingRecord {
            kms_key: object.encryption.value.kms_key.value,
            kms_key_version: object.encryption.value.kms_key_version,
            material_ref: object.encryption.value.material_ref.value,
            ciphertext_ref: object.encryption.value.ciphertext_ref.value,
            kms_encrypt_event_id: object.encryption.value.kms_encrypt_event_id.value,
            purpose: purpose_label(object.encryption.value.purpose).to_string(),
            shred_proof_ref: object
                .encryption
                .value
                .shred_proof_ref
                .map(|proof| proof.value),
        },
        stored_at_epoch_seconds: object.stored_at_epoch_seconds.value,
        last_accessed_at_epoch_seconds: object.last_accessed_at_epoch_seconds.value,
        schema_version: object.schema_version.value,
    }
}
