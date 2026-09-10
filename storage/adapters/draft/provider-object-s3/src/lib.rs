//! Draft S3 backend for the storage provider-object compatibility port.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use storage_provider_draft::{
    StorageProviderKind, StorageProviderObjectError, StorageProviderObjectGetRequest,
    StorageProviderObjectPort, StorageProviderObjectPutRequest, StorageProviderObjectReceipt,
};

include!(concat!(env!("OUT_DIR"), "/lib.generated.rs"));

#[cfg(test)]
mod tests {
    include!(concat!(env!("OUT_DIR"), "/tests.generated.rs"));
}
