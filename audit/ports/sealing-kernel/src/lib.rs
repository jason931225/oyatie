#![allow(dead_code)]

/// Reference to a signing key handle held inside an HSM.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SigningKeyRef {
    pub key_id: String, // data_class: INTERNAL_ONLY
}

/// Pack-scoped key epoch covering the half-open period range `[period_lo, period_hi)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackEpoch {
    pub pack: String,                        // data_class: PUBLIC
    pub tenant_partition: String,            // data_class: INTERNAL_ONLY
    pub period_lo: String,                   // data_class: INTERNAL_ONLY
    pub period_hi: String,                   // data_class: INTERNAL_ONLY
    pub active_key: SigningKeyRef,           // data_class: INTERNAL_ONLY
    pub retiring_key: Option<SigningKeyRef>, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SealStatus {
    Accepted,
    Unsealed,
    Sealed,
    Published,
    Verified,
    Redacted,
    Retained,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealRecord {
    pub pack: String,               // data_class: PUBLIC
    pub tenant_partition: String,   // data_class: INTERNAL_ONLY
    pub period_id: String,          // data_class: INTERNAL_ONLY
    pub leaf_count: u64,            // data_class: PUBLIC
    pub merkle_root: String,        // data_class: INTERNAL_ONLY
    pub prior_root: Option<String>, // data_class: INTERNAL_ONLY
    pub signing_key: SigningKeyRef, // data_class: INTERNAL_ONLY
    pub status: SealStatus,         // data_class: PUBLIC
}

pub trait MerkleEngine {
    type Leaf;
    type Root;
    type Error;
    fn root(&self, leaves: &[Self::Leaf]) -> Result<Self::Root, Self::Error>;
}

pub trait SignerPort {
    type Root;
    type Signature;
    type Error;
    fn sign(&self, root: &Self::Root, epoch: &PackEpoch) -> Result<Self::Signature, Self::Error>;
}

pub trait RootPublisher {
    type Root;
    type Reference;
    type Error;
    fn publish(&self, root: &Self::Root) -> Result<Self::Reference, Self::Error>;
}

pub trait IndexWriter {
    type Error;
    fn insert(&self, record: &SealRecord) -> Result<(), Self::Error>;
}

pub trait ObjectStoreWriter {
    type Blob;
    type Error;
    fn put(&self, blob: &Self::Blob) -> Result<(), Self::Error>;
}
