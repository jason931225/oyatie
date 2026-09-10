//! Deprecated HTTP-shaped block metadata compatibility boundary.

use std::collections::BTreeMap;

use data_classification::{DataClass, parse_data_class_label};
use network_residency_policy::{ResidencyClass, parse_residency_class_label};
use storage_domain::{
    BlockVolume, CloudStorageCatalog, CloudStorageError, EncryptionMode, StorageRepo, VolumeCreate,
    VolumePerformance, VolumeState, VolumeTier,
};

include!(concat!(env!("OUT_DIR"), "/lib.generated.rs"));
