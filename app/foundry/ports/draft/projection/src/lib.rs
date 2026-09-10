//! Foundry projection port: the durable, indexed read plane the
//! projector mirrors each fold outcome into and every object read is
//! served from. Its executable meaning is [`conformance`].
#![forbid(unsafe_code)]
// ADR-0083 Tier 3: tests legitimately use `.unwrap()` / `.expect()` /
// `panic!()` to assert invariants under the `cfg(test)` exemption.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod conformance;
mod keys;
mod link_index;
mod memory;
mod predicate;
mod store;

pub use keys::KeyDesignations;
pub use memory::MemoryProjectionStore;
pub use predicate::{PredicateError, PropertyPredicate};
pub use store::{
    AppliedEntry, ApplyReceipt, EntryOutcome, Page, PageRequest, ProjectedLink, ProjectedObject,
    ProjectionCursor, ProjectionStore, ProjectionStoreError,
};
