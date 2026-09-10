//! Deprecated HTTP-shaped object metadata compatibility boundary.

use std::collections::BTreeMap;

use compute_resource_metadata::ResourceId;
use data_classification::{DataClass, parse_data_class_label};
use secrets_kms::KmsPurpose;
use storage_domain::{
    CloudStorageCatalog, CloudStorageError, ObjectCreate, ObjectEncryptionBindingCreate, ObjectKey,
    StorageRepo, StoredObject,
};

include!(concat!(env!("OUT_DIR"), "/lib.generated.rs"));

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    include!(concat!(env!("OUT_DIR"), "/tests.generated.rs"));
}
