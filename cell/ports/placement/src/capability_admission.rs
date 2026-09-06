//! Deployment qualification of a capability adapter for the effect boundary.
//!
//! Acceptance is signed by an admitted qualification authority, never by the
//! adapter reporting on itself. A Rust type shape is not atomicity evidence;
//! only exercised conformance against the exact binary, configuration, schema
//! and transaction domain is.

use crate::{CapabilityEffectActionV1, CapabilityEffectScopeV1, CellProofEnvelopeV1, Digest32};

/// Which placement role an acceptance qualifies an adapter for.
///
/// The three are incomparable roles, not increasing scopes, and no ordering
/// is derived. A migration source acceptance does not subsume a migration
/// target acceptance in either direction: they cover different actions at
/// different transaction domains, and an initial binding covers neither.
/// Deriving an ordering would invite a future implementation to combine two
/// acceptances with `max()` and silently discard the one it needed.
/// Requirement is same-variant-else-reject. Declaration order and protobuf
/// tag order carry no rank.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CapabilityAdmissionUseV1 {
    InitialBinding,
    MigrationSource,
    MigrationTarget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityAdapterAcceptancePayloadV1 {
    pub scope: CapabilityEffectScopeV1,
    pub use_case: CapabilityAdmissionUseV1,
    pub adapter_binary_digest: Digest32,
    pub adapter_configuration_digest: Digest32,
    pub store_schema_digest: Digest32,
    pub transaction_domain_digest: Digest32,
    pub effect_schema_digest: Digest32,
    pub deployment_admission_epoch: u64,
    pub writable_inventory_digest: Digest32,
    pub conformance_suite_digest: Digest32,
    pub conformance_result_digest: Digest32,
    pub supported_actions: Vec<CapabilityEffectActionV1>,
    pub expires_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedCapabilityAdapterAcceptanceV1 {
    pub payload: CapabilityAdapterAcceptancePayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}
