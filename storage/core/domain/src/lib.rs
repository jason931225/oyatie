//! Storage object/CAS domain and in-memory reference engine.
//!
//! Provider types are source-compatible re-exports from the owner-local draft
//! port.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use cell_location::{AzCode, CellId, RegionCode};
pub use compute_resource_metadata::{BucketTier, FilesystemTier};
use compute_resource_metadata::{CloudResourceError, PrincipalId, ResourceId, ResourceKind};
use data_classification::{Classified, DataClass, PrivacyDataClass};
pub use network_residency_policy::ResidencyClass;
use network_residency_policy::residency_class_allows_home_region_label;
use secrets_kms::{
    CiphertextRef, DestructionProofRef, KmsKeyId, KmsKeyOrigin, KmsPurpose, KmsUseEventId,
    MaterialRef,
};
pub use storage_provider_draft::{
    CloudStorageError, EncryptionMode, StorageBlockOperation, StorageObjectOperation,
    StorageProviderBlockCreateVolumeRequest, StorageProviderBlockError, StorageProviderBlockPort,
    StorageProviderBlockReceipt, StorageProviderKind, StorageProviderObjectError,
    StorageProviderObjectGetRequest, StorageProviderObjectPort, StorageProviderObjectPutRequest,
    StorageProviderObjectReceipt, VolumePerformance, VolumeTier,
};

include!(concat!(env!("OUT_DIR"), "/domain.generated.rs"));

/// Compatibility namespace for the former object-store kernel crate.
pub mod cas {
    use std::collections::BTreeMap;
    use std::fmt;
    use std::sync::{Mutex, MutexGuard};

    include!(concat!(env!("OUT_DIR"), "/cas.generated.rs"));

    #[cfg(test)]
    mod tests {
        include!(concat!(env!("OUT_DIR"), "/cas_tests.generated.rs"));
    }
}

pub use cas::*;

#[cfg(test)]
mod tests {
    include!(concat!(env!("OUT_DIR"), "/domain_tests.generated.rs"));
}
