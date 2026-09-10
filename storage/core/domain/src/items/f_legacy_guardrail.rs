// Retirement-bound mixed storage-product aggregate.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTenantCellGuardrailCreate {
    pub tenant_id: String,
    pub region: String,
    pub primary_cell_id: String,
    pub bucket_id: String,
    pub object_key_prefix: String,
    pub object_versioning_enabled: bool,
    pub object_lock_enabled: bool,
    pub default_object_lock: ObjectLockPolicy,
    pub volume_id: String,
    pub volume_tier: VolumeTier,
    pub snapshot_required: bool,
    pub snapshot_evidence_ref: String,
    pub filesystem_id: String,
    pub filesystem_tier: FilesystemTier,
    pub filesystem_mount_policy_ref: String,
    pub provider_evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTenantCellGuardrail {
    pub tenant_id: Classified<String>,
    pub region: Classified<RegionCode>,
    pub primary_cell_id: Classified<CellId>,
    pub bucket_id: Classified<ResourceId>,
    pub object_key_prefix: Classified<ObjectKey>,
    pub object_versioning_enabled: Classified<bool>,
    pub object_lock_enabled: Classified<bool>,
    pub default_object_lock: Classified<ObjectLockPolicy>,
    pub volume_id: Classified<ResourceId>,
    pub volume_tier: Classified<VolumeTier>,
    pub snapshot_required: Classified<bool>,
    pub snapshot_evidence_ref: Classified<String>,
    pub filesystem_id: Classified<ResourceId>,
    pub filesystem_tier: Classified<FilesystemTier>,
    pub filesystem_mount_policy_ref: Classified<String>,
    pub provider_evidence_refs: Classified<Vec<String>>,
    pub schema_version: Classified<u32>,
}

impl StorageTenantCellGuardrail {
    pub fn new(input: StorageTenantCellGuardrailCreate) -> Result<Self, CloudStorageError> {
        let StorageTenantCellGuardrailCreate {
            tenant_id,
            region,
            primary_cell_id,
            bucket_id,
            object_key_prefix,
            object_versioning_enabled,
            object_lock_enabled,
            default_object_lock,
            volume_id,
            volume_tier,
            snapshot_required,
            snapshot_evidence_ref,
            filesystem_id,
            filesystem_tier,
            filesystem_mount_policy_ref,
            provider_evidence_refs,
        } = input;

        validate_tenant_id(&tenant_id)?;
        let region = RegionCode::new(region).map_err(|_| CloudStorageError::InvalidResourceId)?;
        let primary_cell_id =
            CellId::new(primary_cell_id).map_err(|_| CloudStorageError::InvalidCellId)?;
        validate_cell_location(&primary_cell_id, &region, None)?;

        let bucket_id = resource_id_for_kind_label(&bucket_id, &tenant_id, &region, "bucket")?;
        let volume_id = resource_id_for(
            &volume_id,
            &tenant_id,
            &region,
            ResourceKind::Volume(volume_tier),
        )?;
        let filesystem_id = resource_id_for(
            &filesystem_id,
            &tenant_id,
            &region,
            ResourceKind::Filesystem(filesystem_tier),
        )?;

        let object_key_prefix = ObjectKey::new(object_key_prefix)?;
        validate_object_key_prefix(&object_key_prefix, &tenant_id, &primary_cell_id)?;
        if !object_versioning_enabled {
            return Err(CloudStorageError::InvalidStorageNamespacePolicy);
        }
        if !object_lock_enabled {
            return Err(CloudStorageError::InvalidObjectLockPolicy);
        }
        validate_object_lock(Some(default_object_lock))?;
        if !snapshot_required {
            return Err(CloudStorageError::InvalidStorageNamespacePolicy);
        }
        let snapshot_evidence_ref =
            validate_metadata_ref(snapshot_evidence_ref, REF_SNAPSHOT_EVIDENCE_PREFIX)?;
        let filesystem_mount_policy_ref =
            validate_metadata_ref(filesystem_mount_policy_ref, REF_MOUNT_POLICY_PREFIX)?;
        let provider_evidence_refs =
            validate_metadata_refs(provider_evidence_refs, REF_EVIDENCE_PREFIX)?;

        Ok(Self {
            tenant_id: internal(tenant_id),
            region: public(region),
            primary_cell_id: public(primary_cell_id),
            bucket_id: internal(bucket_id),
            object_key_prefix: internal(object_key_prefix),
            object_versioning_enabled: internal(object_versioning_enabled),
            object_lock_enabled: internal(object_lock_enabled),
            default_object_lock: internal(default_object_lock),
            volume_id: internal(volume_id),
            volume_tier: public(volume_tier),
            snapshot_required: internal(snapshot_required),
            snapshot_evidence_ref: internal(snapshot_evidence_ref),
            filesystem_id: internal(filesystem_id),
            filesystem_tier: public(filesystem_tier),
            filesystem_mount_policy_ref: internal(filesystem_mount_policy_ref),
            provider_evidence_refs: internal(provider_evidence_refs),
            schema_version: public(STORAGE_SCHEMA_VERSION),
        })
    }
}
