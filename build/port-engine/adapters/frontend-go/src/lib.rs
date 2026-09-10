//! # port-engine-frontend-go — Go SourceModel snapshot consumer.
#![forbid(unsafe_code)]

mod sources;
pub use sources::CRATE_SOURCES;

mod convert;
mod error;
mod model;
mod vocabulary;
mod wire;

pub use error::SnapshotError;
pub use model::GoSourceModel;
pub use vocabulary::{
    ATTR_GO_NODE, ATTR_LIT_KIND, ATTR_OP, ATTR_REF, ATTR_VALUE, KNOWN_ATTR_KEYS,
    KNOWN_DECLARATION_KINDS, KNOWN_FLAGS, KNOWN_MEMBER_KINDS, KNOWN_TYPE_KINDS,
    PRODUCER_BOOTSTRAP_GO, PRODUCER_OWNED_RUST, SCHEMA_VERSION_DECLARATIONS,
    SCHEMA_VERSION_FLAT_TYPES, SCHEMA_VERSION_IDENTITY_ONLY,
};

#[must_use]
pub const fn w0_ready() -> bool {
    true
}
