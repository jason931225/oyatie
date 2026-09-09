use crate::{
    CellId, CellLifecycleRevision, CellProofEnvelopeV1, CellProofVerifier, Digest32, DrainTermV1,
    ImmutableEvidenceRefV1, ProducerId, ProofVerificationError,
};

/// Who contributes state to a cell drain.
///
/// THIS ENUM IS THE DIVISION OF LABOUR, and it is the only place the division
/// was ever written down. Cell states the vocabulary and owns the shape of
/// [`crate::DrainContributorStateV1`]; each contributor writes ITS OWN
/// contributor row, in its own store, under its own error taxonomy. That is the
/// same division `capability_effect.rs` states in terms for the capability
/// boundary, and it was legible here only from the arm names.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DrainContributorKindV1 {
    /// Tenancy's bindings. The contributor is the `tenancy-cell-assignment`
    /// crate, whose write sets carry
    /// [`crate::DrainContributorStateMutationV1`] or
    /// [`crate::DrainContributorMutationSetV1`]. Every store that applies one
    /// returns `BindingStoreError`, so the refusal for a disagreeing proposal
    /// on this arm is `BindingStoreError PROPOSED_SUCCESSOR_MISMATCH` and NOT
    /// the [`crate::PlacementContractError`] the cell-side ownership rule head
    /// names — a Tenancy store cannot return that type at all. Stated on both
    /// sides: see the head of `cell/placement/v1/drain_seal.proto`.
    TenancyBindings,
    /// Cell's own reservations, proposed by
    /// [`crate::CellReservationWriteSetPartsV1::drain_mutations`].
    CellReservations,
    /// Cell's own placement operations.
    CellOperations,
    /// The capability plane's local write-authority state.
    CapabilityState,
    /// The routing projections a binding installs.
    RoutingProjections,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainContributorManifestPayloadV1 {
    pub schema_version: u32,
    pub cell_id: CellId,
    pub drain_term: DrainTermV1,
    pub lifecycle_revision: CellLifecycleRevision,
    pub contributor_identity_root_digest: Digest32,
    pub contributor_count: u64,
    pub compiler_version: String,
    pub inventory_snapshot: ImmutableEvidenceRefV1,
    pub source_snapshot_digest: Digest32,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainContributorManifestExpectationV1 {
    pub cell_id: CellId,
    pub drain_term: DrainTermV1,
    pub lifecycle_revision: CellLifecycleRevision,
    pub maximum_contributor_count: u64,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedDrainContributorManifestV1 {
    pub payload: DrainContributorManifestPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedDrainContributorManifest(SignedDrainContributorManifestV1);

impl VerifiedDrainContributorManifest {
    #[must_use]
    pub fn signed(&self) -> &SignedDrainContributorManifestV1 {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainContributorProofPayloadV1 {
    pub schema_version: u32,
    pub cell_id: CellId,
    pub drain_term: DrainTermV1,
    pub manifest_digest: Digest32,
    pub contributor_ordinal: u64,
    pub contributor_kind: DrainContributorKindV1,
    pub contributor_id: String,
    pub contributor_identity_digest: Digest32,
    pub contributor_identity_inclusion_path: Vec<Digest32>,
    pub contributor_seal_digest: Digest32,
    pub sealed_state_revision: crate::DrainContributorStateRevision,
    pub sealed_state_creation_high_water: u64,
    pub sealed_state_root_digest: Digest32,
    pub sealed_state_count: u64,
    pub partition_root_digest: Digest32,
    pub partition_count: u64,
    pub remaining_binding_count: u64,
    pub remaining_reservation_count: u64,
    pub remaining_operation_count: u64,
    pub remaining_capability_state_count: u64,
    pub remaining_projection_count: u64,
    pub observed_at_unix_seconds: u64,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedDrainContributorProofV1 {
    pub payload: DrainContributorProofPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedDrainContributorProof(SignedDrainContributorProofV1);

impl VerifiedDrainContributorProof {
    #[must_use]
    pub fn signed(&self) -> &SignedDrainContributorProofV1 {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DrainProofLedgerRevision(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainProofLedgerV1 {
    pub cell_id: CellId,
    pub drain_term: DrainTermV1,
    pub manifest_digest: Digest32,
    pub expected_contributor_count: u64,
    pub next_contributor_ordinal: u64,
    pub applied_identity_root_digest: Digest32,
    pub applied_proof_root_digest: Digest32,
    pub applied_seal_root_digest: Digest32,
    pub total_remaining_bindings: u64,
    pub total_remaining_reservations: u64,
    pub total_remaining_operations: u64,
    pub total_remaining_capability_states: u64,
    pub total_remaining_projections: u64,
    pub revision: DrainProofLedgerRevision,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellDrainCompletionPayloadV1 {
    pub schema_version: u32,
    pub cell_id: CellId,
    pub drain_term: DrainTermV1,
    pub lifecycle_revision: CellLifecycleRevision,
    pub contributor_manifest_digest: Digest32,
    pub contributor_identity_root_digest: Digest32,
    pub contributor_proof_root_digest: Digest32,
    pub contributor_seal_root_digest: Digest32,
    pub contributor_count: u64,
    pub ledger_revision: DrainProofLedgerRevision,
    pub zero_bindings: u64,
    pub zero_reservations: u64,
    pub zero_operations: u64,
    pub zero_capability_states: u64,
    pub zero_projections: u64,
    pub completed_at_unix_seconds: u64,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedCellDrainCompletionV1 {
    pub payload: CellDrainCompletionPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCellDrainCompletion(SignedCellDrainCompletionV1);

impl VerifiedCellDrainCompletion {
    #[must_use]
    pub fn signed(&self) -> &SignedCellDrainCompletionV1 {
        &self.0
    }
}

pub fn verify_drain_contributor_manifest(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedDrainContributorManifestV1,
    _expectation: &DrainContributorManifestExpectationV1,
) -> Result<VerifiedDrainContributorManifest, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

pub fn verify_drain_contributor_proof(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedDrainContributorProofV1,
    _manifest: &VerifiedDrainContributorManifest,
    _seal: &crate::VerifiedDrainContributorSeal,
    _maximum_inclusion_path_depth: u32,
    _now_unix_seconds: u64,
) -> Result<VerifiedDrainContributorProof, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

pub fn verify_cell_drain_completion(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedCellDrainCompletionV1,
    _manifest: &VerifiedDrainContributorManifest,
    _ledger: &DrainProofLedgerV1,
    _now_unix_seconds: u64,
) -> Result<VerifiedCellDrainCompletion, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}
