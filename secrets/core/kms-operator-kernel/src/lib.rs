//! Pure Cloud KMS operator reconcile kernel.
//!
//! This crate owns CRD-shaped desired/observed state and the pure reconcile
//! decision function. It intentionally has no kube-rs, k8s-openapi, async
//! runtime, or system-clock dependency.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub trait Clock {
    fn now_epoch_seconds(&self) -> u64;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyOrigin {
    OyatieManaged,
    Byok,
    Hyok,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyUsage {
    EncryptDecrypt,
    SignVerify,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HsmValidation {
    PackEnhancedFips1403Level3,
    Fips1403Level3,
    Cryptrec,
    CommonCriteriaEal4,
    PciHsm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResidencyMode {
    StrictHomeRegion,
    HomeWithRecoveryFailover,
    Global,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataClassLabel {
    Public,
    InternalOnly,
    PiiIdentifying,
    Phi,
    Pci,
    Secret,
    Audit,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KeyVersionRotationPolicy {
    pub rotate_after_seconds: u64,       // data_class: INTERNAL_ONLY
    pub decrypt_only_grace_seconds: u64, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KeyRing {
    pub name: String,                              // data_class: INTERNAL_ONLY
    pub tenant_id: String,                         // data_class: INTERNAL_ONLY
    pub region: String,                            // data_class: PUBLIC
    pub cell_id: String,                           // data_class: PUBLIC
    pub hsm_partition_ref: String,                 // data_class: INTERNAL_ONLY
    pub origin: KeyOrigin,                         // data_class: PUBLIC
    pub usage: KeyUsage,                           // data_class: PUBLIC
    pub hsm_validation: HsmValidation,             // data_class: PUBLIC
    pub residency: ResidencyMode,                  // data_class: INTERNAL_ONLY
    pub data_class: DataClassLabel,                // data_class: INTERNAL_ONLY
    pub rotation_policy: KeyVersionRotationPolicy, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SealingRoot {
    pub name: String,              // data_class: INTERNAL_ONLY
    pub tenant_id: String,         // data_class: INTERNAL_ONLY
    pub region: String,            // data_class: PUBLIC
    pub cell_id: String,           // data_class: PUBLIC
    pub root_ref: String,          // data_class: INTERNAL_ONLY
    pub active_version: u32,       // data_class: INTERNAL_ONLY
    pub rotate_after_seconds: u64, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DesiredState {
    pub key_rings: Vec<KeyRing>,         // data_class: INTERNAL_ONLY
    pub sealing_roots: Vec<SealingRoot>, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadConsistency {
    #[default]
    Complete,
    Partial,
    Ambiguous,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", content = "reason", rename_all = "snake_case")]
pub enum ObservedHealth {
    #[default]
    Healthy,
    Ambiguous(String),
    Compromised(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyVersionState {
    Pending,
    Active,
    DecryptOnly,
    Quarantined,
    Destroyed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservedKeyVersion {
    pub version: u32,                                  // data_class: INTERNAL_ONLY
    pub state: KeyVersionState,                        // data_class: PUBLIC
    pub created_at_epoch_seconds: u64,                 // data_class: INTERNAL_ONLY
    pub activated_at_epoch_seconds: u64,               // data_class: INTERNAL_ONLY
    pub decrypt_only_since_epoch_seconds: Option<u64>, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservedKeyRing {
    pub desired: KeyRing,                  // data_class: INTERNAL_ONLY
    pub versions: Vec<ObservedKeyVersion>, // data_class: INTERNAL_ONLY
    pub health: ObservedHealth,            // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservedSealingRoot {
    pub desired: SealingRoot,   // data_class: INTERNAL_ONLY
    pub observed_version: u32,  // data_class: INTERNAL_ONLY
    pub health: ObservedHealth, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservedState {
    pub read_consistency: ReadConsistency, // data_class: INTERNAL_ONLY
    pub key_rings: Vec<ObservedKeyRing>,   // data_class: INTERNAL_ONLY
    pub sealing_roots: Vec<ObservedSealingRoot>, // data_class: INTERNAL_ONLY
}

impl Default for ObservedState {
    fn default() -> Self {
        Self {
            read_consistency: ReadConsistency::Complete,
            key_rings: Vec::new(),
            sealing_roots: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    CreateSealingRoot {
        sealing_root: SealingRoot, // data_class: INTERNAL_ONLY
    },
    CreateKeyRing {
        key_ring: KeyRing,               // data_class: INTERNAL_ONLY
        requested_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
    },
    RotateKeyVersion {
        key_ring: KeyRing,               // data_class: INTERNAL_ONLY
        observed_active_version: u32,    // data_class: INTERNAL_ONLY
        reason: String,                  // data_class: INTERNAL_ONLY
        requested_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
    },
    DemoteKeyVersionToDecryptOnly {
        key_ring_name: String,           // data_class: INTERNAL_ONLY
        tenant_id: String,               // data_class: INTERNAL_ONLY
        version: u32,                    // data_class: INTERNAL_ONLY
        reason: String,                  // data_class: INTERNAL_ONLY
        effective_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
    },
    QuarantineKeyRing {
        key_ring_name: String,           // data_class: INTERNAL_ONLY
        tenant_id: String,               // data_class: INTERNAL_ONLY
        reason: String,                  // data_class: INTERNAL_ONLY
        effective_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
    },
    QuarantineObservedState {
        reason: String,                  // data_class: INTERNAL_ONLY
        effective_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
    },
}

mod reconcile;

pub use reconcile::reconcile;
