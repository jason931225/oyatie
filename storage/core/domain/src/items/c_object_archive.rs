
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ArchiveVaultName {
    pub value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ArchiveTier {
    Instant,
    Cold,
    DeepCold,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ArchiveVaultState {
    Creating,
    Active,
    Deleting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveVaultCreate {
    pub resource_id: String,
    pub tenant_id: String,
    pub name: String,
    pub region: String,
    pub residency: ResidencyClass,
    pub tier: ArchiveTier,
    pub encryption: EncryptionMode,
    pub kms_key: Option<String>,
    pub allowed_data_classes: Vec<DataClass>,
    pub state: ArchiveVaultState,
    pub created_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveVault {
    pub resource_id: Classified<ResourceId>,
    pub tenant_id: Classified<String>,
    pub name: Classified<ArchiveVaultName>,
    pub region: Classified<RegionCode>,
    pub residency: Classified<ResidencyClass>,
    pub tier: Classified<ArchiveTier>,
    pub encryption: Classified<EncryptionMode>,
    pub kms_key: Classified<Option<KmsKeyId>>,
    pub allowed_data_classes: Classified<BTreeSet<PrivacyDataClass>>,
    pub state: Classified<ArchiveVaultState>,
    pub created_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}

impl ArchiveVaultName {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudStorageError> {
        canonical_name(value.into(), CloudStorageError::InvalidResourceId)
            .map(|value| Self { value })
    }
}

impl ArchiveVault {
    pub fn new(input: ArchiveVaultCreate) -> Result<Self, CloudStorageError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != ArchiveVaultState::Creating {
            return Err(CloudStorageError::InvalidInitialState);
        }
        let region =
            RegionCode::new(input.region).map_err(|_| CloudStorageError::InvalidResourceId)?;
        validate_residency_allows_region(&input.residency, &region)?;
        let resource_id = resource_id_for(
            &input.resource_id,
            &input.tenant_id,
            &region,
            ResourceKind::ArchiveVault,
        )?;
        let kms_key = encryption_key(input.encryption, input.kms_key, &region, &input.tenant_id)?;
        Ok(Self {
            resource_id: internal(resource_id),
            tenant_id: internal(input.tenant_id),
            name: internal(ArchiveVaultName::new(input.name)?),
            region: public(region),
            residency: internal(input.residency),
            tier: public(input.tier),
            encryption: public(input.encryption),
            kms_key: internal(kms_key),
            allowed_data_classes: internal(privacy_class_set(input.allowed_data_classes)?),
            state: public(input.state),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(STORAGE_SCHEMA_VERSION),
        })
    }
}
