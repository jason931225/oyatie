//! Unagreed object and block backend-provider contract.
//!
//! This owner-local draft isolates removable S3/OCI backend semantics
//! from the storage core. It is not the sold S3 facade and must not be
//! consumed by other owners.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use cell_location::{AzCode, CellId, RegionCode};
pub use compute_resource_metadata::VolumeTier;
use compute_resource_metadata::{CloudResourceError, PrincipalId, ResourceId, ResourceKind};
pub use data_classification::DataClass;
use data_classification::PrivacyDataClass;
pub use network_residency_policy::ResidencyClass;
use network_residency_policy::residency_class_allows_home_region_label;
use secrets_kms::{CiphertextRef, KmsKeyId, KmsKeyOrigin};

include!(concat!(env!("OUT_DIR"), "/provider.generated.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    include!(concat!(env!("OUT_DIR"), "/provider_tests.generated.rs"));
}
