//! # port-engine-snapshot — bootstrap SourceModel snapshot admission.
#![forbid(unsafe_code)]

mod sources;
pub use sources::CRATE_SOURCES;

mod admit;
mod admitted;
mod error;
mod preimage;

pub use admit::{
    admit_embedded_fixture, admit_embedded_fixture_drift_after_v1,
    admit_embedded_fixture_drift_before_v1, admit_embedded_fixture_failure_v1,
    admit_embedded_fixture_interface_v1, admit_embedded_fixture_ownership_v1,
    admit_embedded_fixture_refused_v1, admit_embedded_fixture_v1, admit_reproducible_pair,
};
pub use admitted::AdmittedSnapshot;
pub use error::AdmitError;
pub use preimage::{snapshot_preimage, snapshot_preimage_v1};

pub(crate) const FIXTURE_SNAPSHOT_JSON: &str = include_str!("fixture-snapshot-v0.json");

pub(crate) const FIXTURE_SNAPSHOT_V1_JSON: &str = include_str!("fixture-snapshot-v1.json");

pub(crate) const FIXTURE_SNAPSHOT_REFUSED_V1_JSON: &str =
    include_str!("fixture-snapshot-refused-v1.json");

/// Embedded fixture for the corpus whose OWNERSHIP the engine must refuse. Separate from the
/// statement-refusal corpus so each class is proven rather than shadowed by whichever package
/// the transform reached first.
pub(crate) const FIXTURE_SNAPSHOT_OWNERSHIP_V1_JSON: &str =
    include_str!("fixture-snapshot-ownership-v1.json");

/// Embedded fixture for the corpus whose interface POSITIONS the engine must refuse: a trait has
/// no size in the target, and returning one by value needs an owner the pack has not declared.
pub(crate) const FIXTURE_SNAPSHOT_INTERFACE_V1_JSON: &str =
    include_str!("fixture-snapshot-interface-v1.json");

/// Embedded fixture for the corpus whose FAILURE returns the engine must refuse: the target's
/// failing return carries only the failure, so a source that returns a computed value beside one
/// has no shape to become.
pub(crate) const FIXTURE_SNAPSHOT_FAILURE_V1_JSON: &str =
    include_str!("fixture-snapshot-failure-v1.json");

/// The UPSTREAM DRIFT pair: one package at two versions, at the SAME unit id.
///
/// Not a refusal corpus and not a second corpus — a second EXTRACTION of the same one, which is
/// what a maintained port sees every time its upstream moves. The engine, the rules and the
/// toolchain are identical across the pair, so the only axis that may move is the one describing
/// the source.
pub(crate) const FIXTURE_SNAPSHOT_DRIFT_BEFORE_V1_JSON: &str =
    include_str!("fixture-snapshot-drift-before-v1.json");

/// See [`FIXTURE_SNAPSHOT_DRIFT_BEFORE_V1_JSON`].
pub(crate) const FIXTURE_SNAPSHOT_DRIFT_AFTER_V1_JSON: &str =
    include_str!("fixture-snapshot-drift-after-v1.json");

#[must_use]
pub const fn w0_ready() -> bool {
    true
}
