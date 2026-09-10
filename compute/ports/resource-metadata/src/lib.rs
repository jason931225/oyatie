//! Agreed cross-owner surface for cloud-resource identity and kind metadata.
//!
//! This port intentionally excludes resource state, registry/repository
//! behavior, policy attachments, tags, and metering.

#![forbid(unsafe_code)]

pub use cell_location::RegionCode;
pub use compute_resource::{
    BareMetalFlavor, BucketTier, CloudResourceError, DatabaseEngine, FilesystemTier,
    FunctionRuntime, GpuFlavor, ImageKind, InstanceFlavor, K8sFlavor, LbProtocol, PrincipalId,
    QueueEngine, ResourceId, ResourceKind, VolumeTier,
};
