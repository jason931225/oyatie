// Retirement-bound EFS-like compatibility model.

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FilesystemName {
    pub value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FilesystemState {
    Creating,
    Available,
    Mounted,
    Deleting,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilesystemCreate {
    pub resource_id: String,
    pub tenant_id: String,
    pub name: String,
    pub region: String,
    pub az: String,
    pub cell_id: String,
    pub residency: ResidencyClass,
    pub tier: FilesystemTier,
    pub size_gib: u64,
    pub throughput_mbps: u64,
    pub encryption: EncryptionMode,
    pub kms_key: Option<String>,
    pub data_class: DataClass,
    pub state: FilesystemState,
    pub created_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudFilesystem {
    pub resource_id: Classified<ResourceId>,
    pub tenant_id: Classified<String>,
    pub name: Classified<FilesystemName>,
    pub region: Classified<RegionCode>,
    pub az: Classified<AzCode>,
    pub cell_id: Classified<CellId>,
    pub residency: Classified<ResidencyClass>,
    pub tier: Classified<FilesystemTier>,
    pub size_gib: Classified<u64>,
    pub throughput_mbps: Classified<u64>,
    pub encryption: Classified<EncryptionMode>,
    pub kms_key: Classified<Option<KmsKeyId>>,
    pub data_class: Classified<PrivacyDataClass>,
    pub state: Classified<FilesystemState>,
    pub created_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}

impl FilesystemName {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudStorageError> {
        canonical_name(value.into(), CloudStorageError::InvalidResourceId)
            .map(|value| Self { value })
    }
}

impl CloudFilesystem {
    pub fn new(input: FilesystemCreate) -> Result<Self, CloudStorageError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != FilesystemState::Creating {
            return Err(CloudStorageError::InvalidInitialState);
        }
        let region =
            RegionCode::new(input.region).map_err(|_| CloudStorageError::InvalidResourceId)?;
        let az = AzCode::new(input.az).map_err(|_| CloudStorageError::InvalidAzCode)?;
        let cell_id = CellId::new(input.cell_id).map_err(|_| CloudStorageError::InvalidCellId)?;
        validate_az_region(&az, &region)?;
        validate_cell_location(&cell_id, &region, Some(&az))?;
        validate_residency_allows_region(&input.residency, &region)?;
        validate_size(input.size_gib)?;
        if input.throughput_mbps == 0 {
            return Err(CloudStorageError::InvalidPerformance);
        }
        let resource_id = resource_id_for(
            &input.resource_id,
            &input.tenant_id,
            &region,
            ResourceKind::Filesystem(input.tier),
        )?;
        let kms_key = encryption_key(input.encryption, input.kms_key, &region, &input.tenant_id)?;
        Ok(Self {
            resource_id: internal(resource_id),
            tenant_id: internal(input.tenant_id),
            name: internal(FilesystemName::new(input.name)?),
            region: public(region),
            az: public(az),
            cell_id: public(cell_id),
            residency: internal(input.residency),
            tier: public(input.tier),
            size_gib: internal(input.size_gib),
            throughput_mbps: public(input.throughput_mbps),
            encryption: public(input.encryption),
            kms_key: internal(kms_key),
            data_class: internal(privacy_class(input.data_class)?),
            state: public(input.state),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(STORAGE_SCHEMA_VERSION),
        })
    }
}
