//! Deprecated package identity for the combined OCI provider compatibility adapter.
//!
//! The implementation emits deterministic command and receipt projections only;
//! it has no credentialed network execution or durability authority.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use storage_provider_draft::{
    EncryptionMode, StorageProviderBlockCreateVolumeRequest, StorageProviderBlockError,
    StorageProviderBlockPort, StorageProviderBlockReceipt, StorageProviderKind,
    StorageProviderObjectError, StorageProviderObjectGetRequest, StorageProviderObjectPort,
    StorageProviderObjectPutRequest, StorageProviderObjectReceipt, VolumeTier,
};

include!(concat!(env!("OUT_DIR"), "/lib.generated.rs"));

#[cfg(test)]
mod tests {
    include!(concat!(env!("OUT_DIR"), "/tests.generated.rs"));
}
