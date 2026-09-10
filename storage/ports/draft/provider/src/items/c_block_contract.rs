#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageProviderBlockCreateVolumeRequest {
    pub request_id: String,
    pub provider_volume_ref: String,
    pub volume_id: String,
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
    pub actor: String,
    pub idempotency_key: String,
    pub requested_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageProviderBlockReceipt {
    pub provider: StorageProviderKind,
    pub operation: StorageBlockOperation,
    pub request_id: String,
    pub provider_request_id: String,
    pub provider_volume_ref: String,
    pub volume_id: String,
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
    pub actor: String,
    pub idempotency_key: String,
    pub provider_evidence_ref: String,
    pub occurred_at_epoch_seconds: u64,
    pub schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageProviderBlockError {
    InvalidProviderVolumeRef,
    InvalidProviderRequestId,
    InvalidProviderEvidenceRef,
    InvalidIdempotencyKey,
    InvalidActorRef,
    InvalidRequestShape(CloudStorageError),
    ProviderRejected {
        provider: StorageProviderKind,
        reason: String,
    },
    ProviderUnavailable {
        provider: StorageProviderKind,
        reason: String,
    },
}

pub trait StorageProviderBlockPort {
    fn provider_kind(&self) -> StorageProviderKind;

    fn create_volume(
        &self,
        input: StorageProviderBlockCreateVolumeRequest,
    ) -> Result<StorageProviderBlockReceipt, StorageProviderBlockError>;
}
