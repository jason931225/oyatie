#![allow(clippy::expect_used, clippy::panic)]

use storage_domain::{
    CloudStorageError, FilesystemTier, ObjectLockMode, ObjectLockPolicy,
    StorageTenantCellGuardrail, StorageTenantCellGuardrailCreate, VolumeTier,
};

fn foundation() -> StorageTenantCellGuardrailCreate {
    StorageTenantCellGuardrailCreate {
        tenant_id: "ten_alpha".to_string(),
        region: "region-alpha1".to_string(),
        primary_cell_id: "cell-region-alpha1-a-001".to_string(),
        bucket_id: "oyatie:cloud:region-alpha1:ten_alpha:bucket:tenant-assets".to_string(),
        object_key_prefix: "ten_alpha/cell-region-alpha1-a-001/".to_string(),
        object_versioning_enabled: true,
        object_lock_enabled: true,
        default_object_lock: ObjectLockPolicy {
            mode: ObjectLockMode::Compliance,
            retain_until_epoch_seconds: 1_800_000_000,
            legal_hold: true,
        },
        volume_id: "oyatie:cloud:region-alpha1:ten_alpha:volume:db-primary".to_string(),
        volume_tier: VolumeTier::ProvisionedIopsSsd,
        snapshot_required: true,
        snapshot_evidence_ref: "snapshot-evidence/ten_alpha/cell-region-alpha1-a-001/db-primary"
            .to_string(),
        filesystem_id: "oyatie:cloud:region-alpha1:ten_alpha:filesystem:shared-docs".to_string(),
        filesystem_tier: FilesystemTier::ThroughputOptimized,
        filesystem_mount_policy_ref: "mount-policy/ten_alpha/cell-region-alpha1-a-001/shared-docs"
            .to_string(),
        provider_evidence_refs: vec![
            "evidence/storage/aws-s3/object-lock-versioning".to_string(),
            "evidence/storage/oci/block-snapshot".to_string(),
        ],
    }
}

#[test]
fn tenant_cell_guardrail_admits_object_block_and_file_metadata_only_namespace() {
    let guardrail = StorageTenantCellGuardrail::new(foundation()).expect("guardrail is valid");

    assert_eq!(guardrail.tenant_id.value, "ten_alpha");
    assert_eq!(guardrail.region.value.value, "region-alpha1");
    assert_eq!(
        guardrail.primary_cell_id.value.value,
        "cell-region-alpha1-a-001"
    );
    assert_eq!(
        guardrail.bucket_id.value.value,
        "oyatie:cloud:region-alpha1:ten_alpha:bucket:tenant-assets"
    );
    assert_eq!(
        guardrail.object_key_prefix.value.value,
        "ten_alpha/cell-region-alpha1-a-001/"
    );
    assert!(guardrail.object_versioning_enabled.value);
    assert!(guardrail.object_lock_enabled.value);
    assert!(guardrail.snapshot_required.value);
    assert_eq!(guardrail.provider_evidence_refs.value.len(), 2);
}

#[test]
fn tenant_cell_guardrail_rejects_object_prefix_outside_tenant_cell_namespace() {
    let error = StorageTenantCellGuardrail::new(StorageTenantCellGuardrailCreate {
        object_key_prefix: "workspace/reporting/".to_string(),
        ..foundation()
    })
    .expect_err("object keys must start with tenant/cell prefix");

    assert_eq!(error, CloudStorageError::InvalidStorageNamespacePolicy);
}

#[test]
fn tenant_cell_guardrail_requires_versioning_lock_and_snapshot_evidence() {
    let versioning_error = StorageTenantCellGuardrail::new(StorageTenantCellGuardrailCreate {
        object_versioning_enabled: false,
        ..foundation()
    })
    .expect_err("versioning is required before object-lock claims");
    assert_eq!(
        versioning_error,
        CloudStorageError::InvalidStorageNamespacePolicy
    );

    let lock_error = StorageTenantCellGuardrail::new(StorageTenantCellGuardrailCreate {
        object_lock_enabled: false,
        ..foundation()
    })
    .expect_err("object lock must be enabled for retention claims");
    assert_eq!(lock_error, CloudStorageError::InvalidObjectLockPolicy);

    let snapshot_error = StorageTenantCellGuardrail::new(StorageTenantCellGuardrailCreate {
        snapshot_required: false,
        ..foundation()
    })
    .expect_err("block storage guardrail requires snapshot evidence");
    assert_eq!(
        snapshot_error,
        CloudStorageError::InvalidStorageNamespacePolicy
    );
}

#[test]
fn tenant_cell_guardrail_rejects_secret_like_evidence_refs() {
    let error = StorageTenantCellGuardrail::new(StorageTenantCellGuardrailCreate {
        provider_evidence_refs: vec!["evidence/storage/aws/token=raw-secret".to_string()],
        ..foundation()
    })
    .expect_err("evidence refs must not carry credential material");

    assert_eq!(error, CloudStorageError::InvalidEvidenceRef);
}

#[test]
fn tenant_cell_guardrail_rejects_volume_and_filesystem_tenant_drift() {
    let volume_error = StorageTenantCellGuardrail::new(StorageTenantCellGuardrailCreate {
        volume_id: "oyatie:cloud:region-alpha1:ten_beta:volume:db-primary".to_string(),
        ..foundation()
    })
    .expect_err("volume tenant must match guardrail tenant");
    assert_eq!(volume_error, CloudStorageError::ResourceTenantMismatch);

    let filesystem_error = StorageTenantCellGuardrail::new(StorageTenantCellGuardrailCreate {
        filesystem_id: "oyatie:cloud:region-alpha1:ten_beta:filesystem:shared-docs".to_string(),
        ..foundation()
    })
    .expect_err("filesystem tenant must match guardrail tenant");
    assert_eq!(filesystem_error, CloudStorageError::ResourceTenantMismatch);
}
