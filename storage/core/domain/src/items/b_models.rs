#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredObject {
    pub bucket_id: Classified<ResourceId>,
    pub tenant_id: Classified<String>,
    pub key: Classified<ObjectKey>,
    pub size_bytes: Classified<u64>,
    pub etag: Classified<ETag>,
    pub data_class: Classified<PrivacyDataClass>,
    pub encryption: Classified<ObjectEncryptionBinding>,
    pub stored_at_epoch_seconds: Classified<u64>,
    pub last_accessed_at_epoch_seconds: Classified<Option<u64>>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VolumeCreate {
    pub resource_id: String,
    pub tenant_id: String,
    pub name: String,
    pub region: String,
    pub az: String,
    pub cell_id: String,
    pub residency: ResidencyClass,
    pub tier: VolumeTier,
    pub size_gib: u64,
    pub performance: VolumePerformance,
    pub encryption: EncryptionMode,
    pub kms_key: Option<String>,
    pub data_class: DataClass,
    pub state: VolumeState,
    pub created_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockVolume {
    pub resource_id: Classified<ResourceId>,
    pub tenant_id: Classified<String>,
    pub name: Classified<VolumeName>,
    pub region: Classified<RegionCode>,
    pub az: Classified<AzCode>,
    pub cell_id: Classified<CellId>,
    pub residency: Classified<ResidencyClass>,
    pub tier: Classified<VolumeTier>,
    pub size_gib: Classified<u64>,
    pub performance: Classified<VolumePerformance>,
    pub encryption: Classified<EncryptionMode>,
    pub kms_key: Classified<Option<KmsKeyId>>,
    pub data_class: Classified<PrivacyDataClass>,
    pub state: Classified<VolumeState>,
    pub created_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}
