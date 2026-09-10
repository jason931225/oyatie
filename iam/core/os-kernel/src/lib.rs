#![cfg_attr(not(test), no_std)]
#![allow(
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::missing_errors_doc
)]
//! Machine primitives ported from the `siderolabs/talos` `machinery` package:
//! machine types, RBAC roles, semantic versions, node addressing, resource
//! identifiers, platforms, and the traits used to run services.
//!
//! It has no dependencies of its own.

extern crate alloc;

pub mod address;
pub mod cel;
pub mod error;
pub mod id;
pub mod machine_type;
pub mod os;
pub mod platform;
pub mod primitives;
pub mod resource;
pub mod role;
pub mod traits;
pub mod version;

pub use address::{Cidr, Hostname, NodeAddress, Port, ResourceId};
pub use cel::{
    DiskLocator, evaluate_disk_locator_bool_expression, validate_disk_locator_bool_expression,
    validate_volume_locator_bool_expression,
};
pub use error::{Error, Result};
pub use id::{Fingerprint, IdGenerator};
pub use machine_type::MachineType;
pub use os::{
    Clock, CommandExecutor, CommandOutput, FileSystem, InMemorySyscalls, ManualClock, MemoryFs,
    MockExecutor, MountEntry, PowerAction, SyscallProvider,
};
pub use platform::Platform;
pub use resource::{Metadata, Namespace, Phase, ResourceKind, ResourcePointer};
pub use role::{PREFIX as ROLE_PREFIX, Role, RoleSet};
pub use traits::Runnable;
pub use version::{Version, VersionRange};
