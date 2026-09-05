use cell_placement::{AssuranceCompilerVersion, AssuranceGeneration, CellId, ReservationRefV1};

use crate::{
    BindingDigest32, BindingGeneration, BindingOperationKey, BindingProducerId,
    BindingProofEnvelopeV1, BindingProofVerificationError, BindingProofVerifier, BindingRevision,
    TenantId, WriteAuthorityEpoch, WriteFenceModeV1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationCommitSealPayloadV1 {
    pub schema_version: u32,
    pub operation: BindingOperationKey,
    pub tenant_id: TenantId,
    pub source_cell_id: CellId,
    pub target_cell_id: CellId,
    pub source_generation: BindingGeneration,
    pub source_revision: BindingRevision,
    pub source_binding_record_digest: BindingDigest32,
    pub source_home_reservation: ReservationRefV1,
    pub source_warm_recovery_reservation: Option<ReservationRefV1>,
    pub source_reservation_set_digest: BindingDigest32,
    pub successor_generation: BindingGeneration,
    pub assurance_generation: AssuranceGeneration,
    pub assurance_compiler_version: AssuranceCompilerVersion,
    pub assurance_requirements_digest: BindingDigest32,
    pub placement_assurance_evidence_digest: BindingDigest32,
    pub placement_recovery_evidence_digest: BindingDigest32,
    pub binding_attempt_digest: BindingDigest32,
    pub placement_decision_digest: BindingDigest32,
    pub home_reservation: ReservationRefV1,
    pub warm_recovery_reservation: Option<ReservationRefV1>,
    pub reservation_commit_permit_digest: BindingDigest32,
    pub transfer_authorization_set_digest: BindingDigest32,
    pub required_transfer_effect_manifest_digest: BindingDigest32,
    pub migration_fence_claim_digest: BindingDigest32,
    pub movement_permit_digest: BindingDigest32,
    pub forward_completion_coverage_digest: BindingDigest32,
    pub participant_manifest_digest: BindingDigest32,
    pub initial_copy_digest: BindingDigest32,
    pub final_delta_digest: BindingDigest32,
    pub write_fence_digest: BindingDigest32,
    pub source_fencing_completion_digest: BindingDigest32,
    pub source_authority: crate::ServingAuthorityInstanceV1,
    pub source_retirement: crate::ServingAuthorityRetirementEvidenceV1,
    pub source_authority_terminal_closure_digest: BindingDigest32,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub write_fence_mode: WriteFenceModeV1,
    pub target_readiness_digest: BindingDigest32,
    pub recovery_evidence_digest: BindingDigest32,
    pub target_activation_nonce_digest: BindingDigest32,
    pub rollback_window_policy: crate::RollbackWindowPolicyV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedMigrationCommitSealV1 {
    pub payload: MigrationCommitSealPayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationCommitSealExpectationV1 {
    pub operation: BindingOperationKey,
    pub tenant_id: TenantId,
    pub source_cell_id: CellId,
    pub target_cell_id: CellId,
    pub source_generation: BindingGeneration,
    pub source_revision: BindingRevision,
    pub source_binding_record_digest: BindingDigest32,
    pub source_home_reservation: ReservationRefV1,
    pub source_warm_recovery_reservation: Option<ReservationRefV1>,
    pub source_reservation_set_digest: BindingDigest32,
    pub successor_generation: BindingGeneration,
    pub assurance_generation: AssuranceGeneration,
    pub assurance_compiler_version: AssuranceCompilerVersion,
    pub assurance_requirements_digest: BindingDigest32,
    pub placement_assurance_evidence_digest: BindingDigest32,
    pub placement_recovery_evidence_digest: BindingDigest32,
    pub binding_attempt_digest: BindingDigest32,
    pub placement_decision_digest: BindingDigest32,
    pub home_reservation: ReservationRefV1,
    pub warm_recovery_reservation: Option<ReservationRefV1>,
    pub reservation_commit_permit_digest: BindingDigest32,
    pub transfer_authorization_set_digest: BindingDigest32,
    pub required_transfer_effect_manifest_digest: BindingDigest32,
    pub migration_fence_claim_digest: BindingDigest32,
    pub movement_permit_digest: BindingDigest32,
    pub forward_completion_coverage_digest: BindingDigest32,
    pub participant_manifest_digest: BindingDigest32,
    pub initial_copy_digest: BindingDigest32,
    pub final_delta_digest: BindingDigest32,
    pub write_fence_digest: BindingDigest32,
    pub source_fencing_completion_digest: BindingDigest32,
    pub source_authority: crate::ServingAuthorityInstanceV1,
    pub source_retirement: crate::ServingAuthorityRetirementEvidenceV1,
    pub source_authority_terminal_closure_digest: BindingDigest32,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub write_fence_mode: WriteFenceModeV1,
    pub target_readiness_digest: BindingDigest32,
    pub recovery_evidence_digest: BindingDigest32,
    pub target_activation_nonce_digest: BindingDigest32,
    pub rollback_window_policy: crate::RollbackWindowPolicyV1,
    pub expected_producer: BindingProducerId,
    pub expected_audience: BindingProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedMigrationCommitSeal(SignedMigrationCommitSealV1);

impl VerifiedMigrationCommitSeal {
    #[must_use]
    pub fn signed(&self) -> &SignedMigrationCommitSealV1 {
        &self.0
    }
}

pub fn verify_migration_commit_seal(
    _verifier: &dyn BindingProofVerifier,
    _signed: SignedMigrationCommitSealV1,
    _terminal_closure: &crate::VerifiedServingAuthorityTerminalClosure,
    _expectation: &MigrationCommitSealExpectationV1,
) -> Result<VerifiedMigrationCommitSeal, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}
