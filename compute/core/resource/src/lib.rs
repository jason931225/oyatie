//! Cloud resource aggregate kernel.
//!
//! This crate owns the `CLOUD_RESOURCE_TYPE` contract.
// ADR-0083 Tier 3: tests legitimately use `.unwrap()` / `.expect()` /
// `panic!()` to assert invariants under the `cfg(test)` exemption.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod aggregate;
mod error;
mod identity;
mod kind;
mod lifecycle;
mod registry;

pub use aggregate::{Resource, ResourceCreate, resource_data_class_from_legacy};
pub use error::CloudResourceError;
pub use identity::{IamPolicyId, MeteringTag, PrincipalId, ResourceId, TagKey, TagValue};
pub use kind::{
    BareMetalFlavor, BucketTier, DatabaseEngine, FilesystemTier, FunctionRuntime, GpuFlavor,
    ImageKind, InstanceFlavor, K8sFlavor, LbProtocol, QueueEngine, ResourceKind, VolumeTier,
};
pub use lifecycle::ResourceState;
pub use registry::{ResourceRegistry, ResourceRepo};

#[cfg(test)]
mod tests;
