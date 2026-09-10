#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageProviderObjectPutRequest {
    pub request_id: String,
    pub provider_bucket_ref: String,
    pub bucket_id: String,
    pub tenant_id: String,
    pub object_key: String,
    pub object_body_ref: String,
    pub size_bytes: u64,
    pub etag: String,
    pub data_class: DataClass,
    pub kms_key: String,
    pub ciphertext_ref: String,
    pub actor: String,
    pub idempotency_key: String,
    pub requested_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageProviderObjectGetRequest {
    pub request_id: String,
    pub provider_bucket_ref: String,
    pub bucket_id: String,
    pub tenant_id: String,
    pub object_key: String,
    pub result_body_ref: String,
    pub actor: String,
    pub requested_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageProviderObjectReceipt {
    pub provider: StorageProviderKind,
    pub operation: StorageObjectOperation,
    pub request_id: String,
    pub provider_request_id: String,
    pub provider_bucket_ref: String,
    pub bucket_id: String,
    pub tenant_id: String,
    pub object_key: String,
    pub object_body_ref: String,
    pub size_bytes: Option<u64>,
    pub etag: Option<String>,
    pub data_class: Option<DataClass>,
    pub kms_key: Option<String>,
    pub ciphertext_ref: Option<String>,
    pub actor: String,
    pub provider_evidence_ref: String,
    pub occurred_at_epoch_seconds: u64,
    pub schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageProviderObjectError {
    InvalidProviderBucketRef,
    InvalidProviderRequestId,
    InvalidProviderEvidenceRef,
    InvalidObjectBodyRef,
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

pub trait StorageProviderObjectPort {
    fn provider_kind(&self) -> StorageProviderKind;

    fn put_object(
        &self,
        input: StorageProviderObjectPutRequest,
    ) -> Result<StorageProviderObjectReceipt, StorageProviderObjectError>;

    fn get_object(
        &self,
        input: StorageProviderObjectGetRequest,
    ) -> Result<StorageProviderObjectReceipt, StorageProviderObjectError>;
}
