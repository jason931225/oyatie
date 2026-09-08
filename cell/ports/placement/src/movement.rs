use crate::{
    BindingOutcomeQueryRefV1, CellId, CellProofEnvelopeV1, Digest32, ForwardCompletionReserveV1,
    MovementBudgetGrantV1, MovementBudgetV1, PlacementPolicyGeneration, ProducerId,
    ProofConstructionError, TenantId,
};

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn parse(_value: impl Into<String>) -> Result<Self, ProofConstructionError> {
                Err(ProofConstructionError::NotImplemented)
            }
        }
    };
}

opaque_id!(MovementSchedulingPermitId);
opaque_id!(MovementWorkerId);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementWorkerLeaseV1 {
    pub worker_id: MovementWorkerId,
    pub lease_epoch: u64,
    pub not_before_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingParticipantManifestCommitmentPayloadV1 {
    pub schema_version: u32,
    pub tenant_id: TenantId,
    pub binding_operation: BindingOutcomeQueryRefV1,
    pub binding_attempt_digest: Digest32,
    pub participant_manifest_record_digest: Digest32,
    pub capability_inventory_snapshot_digest: Digest32,
    pub ordered_participant_root_digest: Digest32,
    pub participant_count: u64,
    pub required_writable_capability_root_digest: Digest32,
    pub required_writable_capability_count: u64,
    pub covered_writable_capability_root_digest: Digest32,
    pub covered_writable_capability_count: u64,
    pub coverage_proof_digest: Digest32,
    pub committed_at_unix_seconds: u64,
}

/// Tenancy's attestation that it committed a binding participant manifest.
///
/// THIS TYPE HAS NO PRODUCER IN THIS CRATE, AND MUST NOT BE GIVEN ONE. It is an
/// external arrival: a verifier and consumers live here, the producer lives in
/// the Tenancy binding coordinator. Cell has no participant-manifest store and
/// writes no participant manifest, so there is no local write for a local
/// observer to witness. Adding a producer port would invent a minting path for
/// evidence this cell does not generate, which is worse than the gap it appears
/// to close, because it would look like a fix.
///
/// Recorded because an automated sweep for the "signed, verified, consumed, but
/// never produced" defect species will keep selecting this type. That sweep keys
/// on payload fields named `committed_*`, and the only such field here is
/// `committed_at_unix_seconds`, a timestamp - the instrument cannot see WHOSE
/// state is attested, which is the fact that decides the case. Peers in the same
/// position: [`crate::SignedBindingOutcomeV1`] and
/// [`crate::SignedReservationCommitPermitV1`].
///
/// The rule that separates this from a real defect: a signed type whose subject
/// is a store's OWN committed row needs an independent local producer; one whose
/// subject is another party's state arrives from outside and needs none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedBindingParticipantManifestCommitmentV1 {
    pub payload: BindingParticipantManifestCommitmentPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingParticipantManifestCommitmentExpectationV1 {
    pub tenant_id: TenantId,
    pub binding_operation: BindingOutcomeQueryRefV1,
    pub binding_attempt_digest: Digest32,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedBindingParticipantManifestCommitment(
    SignedBindingParticipantManifestCommitmentV1,
);

impl VerifiedBindingParticipantManifestCommitment {
    #[must_use]
    pub fn signed(&self) -> &SignedBindingParticipantManifestCommitmentV1 {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellMovementPermitIntentV1 {
    pub schema_version: u32,
    pub tenant_id: TenantId,
    pub scheduling_operation: crate::PlacementOperationKey,
    pub binding_operation: BindingOutcomeQueryRefV1,
    pub source_cell_id: CellId,
    pub target_cell_id: CellId,
    pub placement_policy_generation: PlacementPolicyGeneration,
    pub assurance_requirements_digest: Digest32,
    pub assurance_evidence_digest: Digest32,
    pub recovery_evidence_digest: Digest32,
    pub placement_decision_digest: Digest32,
    pub binding_attempt_digest: Digest32,
    pub reservation_commit_permit_digest: Digest32,
    pub budget_request: crate::MovementBudgetRequestV1,
    pub scheduling_permit_id: MovementSchedulingPermitId,
    pub parent_deadline_unix_seconds: u64,
    pub worker_lease: MovementWorkerLeaseV1,
    pub ordinary_budget: MovementBudgetV1,
    pub forward_completion_reserve: ForwardCompletionReserveV1,
    pub forward_completion_coverage: crate::ForwardCompletionCoverageV1,
    pub budget_grant: MovementBudgetGrantV1,
    pub binding_participant_commitment_digest: Digest32,
    pub participant_manifest_record_digest: Digest32,
    pub capability_inventory_snapshot_digest: Digest32,
    pub ordered_participant_root_digest: Digest32,
    pub participant_count: u64,
    pub coverage_proof_digest: Digest32,
    pub intent_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellMovementPermitPayloadV1 {
    pub schema_version: u32,
    pub intent: CellMovementPermitIntentV1,
    pub issuance_revision: crate::MovementPermitIssuanceRevision,
    pub issuance_record_digest: Digest32,
    pub commit_observation: crate::SignedMovementPermitCommitObservationV1,
    pub permit_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedCellMovementPermitV1 {
    pub payload: CellMovementPermitPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CellMovementPermitExpectationV1 {
    pub tenant_id: TenantId,
    pub scheduling_operation: crate::PlacementOperationKey,
    pub binding_operation: BindingOutcomeQueryRefV1,
    pub source_cell_id: CellId,
    pub target_cell_id: CellId,
    pub placement_policy_generation: PlacementPolicyGeneration,
    pub assurance_requirements_digest: Digest32,
    pub assurance_evidence_digest: Digest32,
    pub recovery_evidence_digest: Digest32,
    pub placement_decision_digest: Digest32,
    pub binding_attempt_digest: Digest32,
    pub reservation_commit_permit_digest: Digest32,
    pub participant_manifest: VerifiedBindingParticipantManifestCommitment,
    pub budget_request: crate::MovementBudgetRequestV1,
    pub budget_lineage_digest: Digest32,
    pub leaf_state_revision_at_commit: crate::MovementBudgetAuthorityRevision,
    pub leaf_state_record_digest_at_commit: Digest32,
    pub issuance_revision: crate::MovementPermitIssuanceRevision,
    pub issuance_record_digest: Digest32,
    pub commit_context: crate::MovementPermitCommitContextV1,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub expected_store_producer: ProducerId,
    pub expected_store_audience: ProducerId,
    pub now_unix_seconds: u64,
}
