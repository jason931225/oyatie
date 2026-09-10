const PROVIDER_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloudStorageError {
    InvalidTenantId,
    InvalidResourceId,
    ResourceTenantMismatch,
    ResourceRegionMismatch,
    ResourceKindMismatch,
    InvalidBucketName,
    InvalidObjectKey,
    InvalidEtag,
    InvalidKmsKeyId,
    InvalidKmsKeyVersion,
    InvalidKmsUseEventId,
    InvalidMaterialRef,
    InvalidCiphertextRef,
    InvalidKmsPurpose,
    InvalidDestructionProofRef,
    MissingKmsKey,
    UnexpectedKmsKey,
    KmsKeyModeMismatch,
    KmsKeyTenantMismatch,
    KmsKeyRegionMismatch,
    InvalidReplicationPolicy,
    DuplicateReplicationRegion,
    ReplicationResidencyDenied,
    EmptyAllowedDataClassSet,
    DuplicateDataClass,
    InvalidDataClass,
    ObjectDataClassDenied,
    InvalidObjectLockPolicy,
    InvalidSize,
    InvalidPerformance,
    InvalidAzCode,
    InvalidCellId,
    AzRegionMismatch,
    CellLocationMismatch,
    InvalidSnapshotId,
    InvalidInitialState,
    InvalidTimeOrder,
    InvalidStorageNamespacePolicy,
    InvalidEvidenceRef,
    DuplicateBucket,
    UnknownBucket,
    DuplicateObject,
    DuplicateVolume,
    UnknownVolume,
    DuplicateFilesystem,
    DuplicateArchiveVault,
    DuplicateSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum EncryptionMode {
    Sse,
    SseKms,
    Byok,
    Hyok,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum StorageProviderKind {
    OciObjectStorage,
    OciBlockStorage,
    S3ObjectStorage,
}

impl StorageProviderKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::OciObjectStorage => "oci_object_storage",
            Self::OciBlockStorage => "oci_block_storage",
            Self::S3ObjectStorage => "s3_object_storage",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum StorageObjectOperation {
    PutObject,
    GetObject,
}

impl StorageObjectOperation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::PutObject => "put_object",
            Self::GetObject => "get_object",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum StorageBlockOperation {
    CreateVolume,
}

impl StorageBlockOperation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CreateVolume => "create_volume",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct VolumePerformance {
    pub iops: u64,
    pub throughput_mbps: u64,
}
