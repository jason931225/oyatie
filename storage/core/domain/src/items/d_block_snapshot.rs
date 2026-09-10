
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SnapshotId {
    pub value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SnapshotState {
    Creating,
    Complete,
    Deleting,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotCreate {
    pub id: String,
    pub tenant_id: String,
    pub source_volume_id: String,
    pub region: String,
    pub data_class: DataClass,
    pub state: SnapshotState,
    pub created_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VolumeSnapshot {
    pub id: Classified<SnapshotId>,
    pub tenant_id: Classified<String>,
    pub source_volume_id: Classified<ResourceId>,
    pub region: Classified<RegionCode>,
    pub data_class: Classified<PrivacyDataClass>,
    pub state: Classified<SnapshotState>,
    pub created_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}

impl SnapshotId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudStorageError> {
        let value = value.into();
        if value.starts_with(SNAPSHOT_ID_PREFIX) && value.len() > SNAPSHOT_ID_PREFIX.len() {
            Ok(Self { value })
        } else {
            Err(CloudStorageError::InvalidSnapshotId)
        }
    }
}

impl VolumeSnapshot {
    pub fn new(
        source_volume: &BlockVolume,
        input: SnapshotCreate,
    ) -> Result<Self, CloudStorageError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != SnapshotState::Creating {
            return Err(CloudStorageError::InvalidInitialState);
        }
        let source_volume_id = ResourceId::new(input.source_volume_id)
            .map_err(|_| CloudStorageError::InvalidResourceId)?;
        if source_volume_id != source_volume.resource_id.value {
            return Err(CloudStorageError::UnknownVolume);
        }
        if input.tenant_id != source_volume.tenant_id.value {
            return Err(CloudStorageError::ResourceTenantMismatch);
        }
        let region =
            RegionCode::new(input.region).map_err(|_| CloudStorageError::InvalidResourceId)?;
        if region != source_volume.region.value {
            return Err(CloudStorageError::ResourceRegionMismatch);
        }
        let data_class = privacy_class(input.data_class)?;
        if data_class != source_volume.data_class.value {
            return Err(CloudStorageError::InvalidDataClass);
        }
        Ok(Self {
            id: internal(SnapshotId::new(input.id)?),
            tenant_id: internal(input.tenant_id),
            source_volume_id: internal(source_volume_id),
            region: public(region),
            data_class: internal(data_class),
            state: public(input.state),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(STORAGE_SCHEMA_VERSION),
        })
    }
}
