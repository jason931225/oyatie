const STORAGE_SCHEMA_VERSION: u32 = 1;
const TENANT_ID_PREFIX: &str = "ten_";
const SNAPSHOT_ID_PREFIX: &str = "snap_";
const REF_EVIDENCE_PREFIX: &str = "evidence/";
const REF_SNAPSHOT_EVIDENCE_PREFIX: &str = "snapshot-evidence/";
const REF_MOUNT_POLICY_PREFIX: &str = "mount-policy/";
const MAX_BUCKET_NAME_LEN: usize = 63;
const MAX_OBJECT_KEY_LEN: usize = 1024;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct BucketName {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ObjectKey {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ETag {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct VolumeName {
    pub value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ReplicationMode {
    None,
    Regional,
    CrossRegion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplicationPolicyCreate {
    None,
    Regional,
    CrossRegion { destination_regions: Vec<String> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplicationPolicy {
    None,
    Regional,
    CrossRegion {
        destination_regions: Vec<RegionCode>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ObjectLockMode {
    Governance,
    Compliance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ObjectLockPolicy {
    pub mode: ObjectLockMode,
    pub retain_until_epoch_seconds: u64,
    pub legal_hold: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum BucketState {
    Creating,
    Active,
    Suspended,
    Deleting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum VolumeState {
    Creating,
    Available,
    Attached,
    Deleting,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BucketCreate {
    pub resource_id: String,
    pub tenant_id: String,
    pub name: String,
    pub region: String,
    pub residency: ResidencyClass,
    pub tier: BucketTier,
    pub replication: ReplicationPolicyCreate,
    pub encryption: EncryptionMode,
    pub kms_key: Option<String>,
    pub object_lock: Option<ObjectLockPolicy>,
    pub allowed_data_classes: Vec<DataClass>,
    pub state: BucketState,
    pub created_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bucket {
    pub resource_id: Classified<ResourceId>,
    pub tenant_id: Classified<String>,
    pub name: Classified<BucketName>,
    pub region: Classified<RegionCode>,
    pub residency: Classified<ResidencyClass>,
    pub tier: Classified<BucketTier>,
    pub replication: Classified<ReplicationPolicy>,
    pub encryption: Classified<EncryptionMode>,
    pub kms_key: Classified<Option<KmsKeyId>>,
    pub object_lock: Classified<Option<ObjectLockPolicy>>,
    pub allowed_data_classes: Classified<BTreeSet<PrivacyDataClass>>,
    pub state: Classified<BucketState>,
    pub created_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectCreate {
    pub bucket_id: String,
    pub tenant_id: String,
    pub key: String,
    pub size_bytes: u64,
    pub etag: String,
    pub data_class: DataClass,
    pub encryption: ObjectEncryptionBindingCreate,
    pub stored_at_epoch_seconds: u64,
    pub last_accessed_at_epoch_seconds: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectEncryptionBindingCreate {
    pub kms_key: String,
    pub kms_key_version: u32,
    pub material_ref: String,
    pub ciphertext_ref: String,
    pub kms_encrypt_event_id: String,
    pub purpose: KmsPurpose,
    pub shred_proof_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectEncryptionBinding {
    pub kms_key: KmsKeyId,
    pub kms_key_version: u32,
    pub material_ref: MaterialRef,
    pub ciphertext_ref: CiphertextRef,
    pub kms_encrypt_event_id: KmsUseEventId,
    pub purpose: KmsPurpose,
    pub shred_proof_ref: Option<DestructionProofRef>,
}
