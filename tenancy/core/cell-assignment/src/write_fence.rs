use cell_placement::CellId;

use crate::{
    BindingDigest32, BindingGeneration, BindingOperationKey, BindingProducerId,
    BindingProofConstructionError, BindingProofEnvelopeV1, BindingProofVerificationError,
    BindingProofVerifier, BindingRevision, TenantId, VerifiedParticipantPhaseClosure,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WriteAuthorityEpoch(pub u64);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CapabilityParticipantId(String);

impl CapabilityParticipantId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(_value: impl Into<String>) -> Result<Self, BindingProofConstructionError> {
        Err(BindingProofConstructionError::NotImplemented)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum WriteFenceModeV1 {
    CooperativeSourceFence,
    RecoveryLeaseExpiry,
    RecoveryStonith,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFenceDirectivePayloadV1 {
    pub schema_version: u32,
    pub operation: BindingOperationKey,
    pub tenant_id: TenantId,
    pub source_cell_id: CellId,
    pub target_cell_id: CellId,
    pub source_generation: BindingGeneration,
    pub source_revision: BindingRevision,
    pub source_binding_record_digest: BindingDigest32,
    pub successor_generation: BindingGeneration,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub migration_fence_claim_digest: BindingDigest32,
    pub participant_id: CapabilityParticipantId,
    pub participant_membership_digest: BindingDigest32,
    pub participant_manifest_digest: BindingDigest32,
    pub source_authority: crate::ServingAuthorityInstanceV1,
    pub source_authority_freeze_result_digest: BindingDigest32,
    pub committed_source_horizon: crate::ServingAuthorityCommittedIssuanceHorizonV1,
    pub not_before_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedSourceFenceDirectiveV1 {
    pub payload: SourceFenceDirectivePayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFenceDirectiveExpectationV1 {
    pub operation: BindingOperationKey,
    pub tenant_id: TenantId,
    pub source_cell_id: CellId,
    pub target_cell_id: CellId,
    pub source_generation: BindingGeneration,
    pub source_revision: BindingRevision,
    pub source_binding_record_digest: BindingDigest32,
    pub successor_generation: BindingGeneration,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub migration_fence_claim_digest: BindingDigest32,
    pub participant_id: CapabilityParticipantId,
    pub participant_membership_digest: BindingDigest32,
    pub participant_manifest_digest: BindingDigest32,
    pub source_authority: crate::ServingAuthorityInstanceV1,
    pub source_authority_freeze_result_digest: BindingDigest32,
    pub committed_source_horizon: crate::ServingAuthorityCommittedIssuanceHorizonV1,
    pub expected_producer: BindingProducerId,
    pub expected_audience: BindingProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedSourceFenceDirective(SignedSourceFenceDirectiveV1);

impl VerifiedSourceFenceDirective {
    #[must_use]
    pub fn signed(&self) -> &SignedSourceFenceDirectiveV1 {
        &self.0
    }
}

pub fn verify_source_fence_directive(
    _verifier: &dyn BindingProofVerifier,
    _signed: SignedSourceFenceDirectiveV1,
    _freeze: &crate::VerifiedServingAuthorityFreezeResult,
    _expectation: &SourceFenceDirectiveExpectationV1,
) -> Result<VerifiedSourceFenceDirective, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverySourceFenceCompletionPayloadV1 {
    pub schema_version: u32,
    pub operation: BindingOperationKey,
    pub tenant_id: TenantId,
    pub source_cell_id: CellId,
    pub target_cell_id: CellId,
    pub source_generation: BindingGeneration,
    pub source_revision: BindingRevision,
    pub successor_generation: BindingGeneration,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub migration_fence_claim_digest: BindingDigest32,
    pub participant_manifest_digest: BindingDigest32,
    pub participant_root_digest: BindingDigest32,
    pub participant_count: u64,
    pub mode: WriteFenceModeV1,
    pub evidence_set_root_digest: BindingDigest32,
    pub evidence_count: u64,
    pub evidence_record: cell_placement::ImmutableEvidenceRefV1,
    pub source_authority: crate::ServingAuthorityInstanceV1,
    pub recovery_basis: crate::ServingAuthorityRecoveryBasisV1,
    pub completed_at_unix_seconds: u64,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedRecoverySourceFenceCompletionV1 {
    pub payload: RecoverySourceFenceCompletionPayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverySourceFenceCompletionExpectationV1 {
    pub operation: BindingOperationKey,
    pub tenant_id: TenantId,
    pub source_cell_id: CellId,
    pub target_cell_id: CellId,
    pub source_generation: BindingGeneration,
    pub source_revision: BindingRevision,
    pub successor_generation: BindingGeneration,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub migration_fence_claim_digest: BindingDigest32,
    pub participant_manifest_digest: BindingDigest32,
    pub participant_root_digest: BindingDigest32,
    pub participant_count: u64,
    pub mode: WriteFenceModeV1,
    pub source_authority: crate::ServingAuthorityInstanceV1,
    pub recovery_basis: crate::ServingAuthorityRecoveryBasisV1,
    pub expected_producer: BindingProducerId,
    pub expected_audience: BindingProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedRecoverySourceFenceCompletion(SignedRecoverySourceFenceCompletionV1);

impl VerifiedRecoverySourceFenceCompletion {
    #[must_use]
    pub fn signed(&self) -> &SignedRecoverySourceFenceCompletionV1 {
        &self.0
    }
}

pub fn verify_recovery_source_fence_completion(
    _verifier: &dyn BindingProofVerifier,
    _signed: SignedRecoverySourceFenceCompletionV1,
    _retirement: &crate::VerifiedServingAuthorityRetirementV1,
    _expectation: &RecoverySourceFenceCompletionExpectationV1,
) -> Result<VerifiedRecoverySourceFenceCompletion, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub enum VerifiedSourceFencingCompletionV1 {
    Cooperative(Box<VerifiedParticipantPhaseClosure>),
    Recovery(Box<VerifiedRecoverySourceFenceCompletion>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WriteFenceEvidenceV1 {
    CooperativeSourceFence {
        source_fence_directive_root_digest: BindingDigest32,
    },
    RecoveryLeaseExpiry {
        recovery_fence_completion_digest: BindingDigest32,
    },
    RecoveryStonith {
        recovery_fence_completion_digest: BindingDigest32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteFencePayloadV1 {
    pub schema_version: u32,
    pub operation: BindingOperationKey,
    pub tenant_id: TenantId,
    pub source_cell_id: CellId,
    pub target_cell_id: CellId,
    pub source_generation: BindingGeneration,
    pub source_revision: BindingRevision,
    pub successor_generation: BindingGeneration,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub evidence: WriteFenceEvidenceV1,
    pub source_binding_record_digest: BindingDigest32,
    pub binding_attempt_digest: BindingDigest32,
    pub migration_fence_claim_digest: BindingDigest32,
    pub movement_permit_digest: BindingDigest32,
    pub placement_decision_digest: BindingDigest32,
    pub reservation_commit_permit_digest: BindingDigest32,
    pub transfer_authorization_set_digest: BindingDigest32,
    pub required_transfer_effect_manifest_digest: BindingDigest32,
    pub participant_manifest_digest: BindingDigest32,
    pub source_fencing_completion_digest: BindingDigest32,
    pub source_authority: crate::ServingAuthorityInstanceV1,
    pub source_retirement: crate::ServingAuthorityRetirementEvidenceV1,
    pub source_fence_directive_ledger_digest: Option<BindingDigest32>,
    pub final_delta_digest: BindingDigest32,
    pub clock_uncertainty_bound_millis: u64,
    pub irreversible_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedWriteFenceV1 {
    pub payload: WriteFencePayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteFenceExpectationV1 {
    pub operation: BindingOperationKey,
    pub tenant_id: TenantId,
    pub source_cell_id: CellId,
    pub target_cell_id: CellId,
    pub source_generation: BindingGeneration,
    pub source_revision: BindingRevision,
    pub successor_generation: BindingGeneration,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub mode: WriteFenceModeV1,
    pub maximum_clock_uncertainty_bound_millis: u64,
    pub source_binding_record_digest: BindingDigest32,
    pub binding_attempt_digest: BindingDigest32,
    pub migration_fence_claim_digest: BindingDigest32,
    pub movement_permit_digest: BindingDigest32,
    pub placement_decision_digest: BindingDigest32,
    pub reservation_commit_permit_digest: BindingDigest32,
    pub transfer_authorization_set_digest: BindingDigest32,
    pub required_transfer_effect_manifest_digest: BindingDigest32,
    pub participant_manifest_digest: BindingDigest32,
    pub source_fencing_completion_digest: BindingDigest32,
    pub source_authority: crate::ServingAuthorityInstanceV1,
    pub source_retirement: crate::ServingAuthorityRetirementEvidenceV1,
    pub source_fence_directive_ledger_digest: Option<BindingDigest32>,
    pub final_delta_digest: BindingDigest32,
    pub expected_producer: BindingProducerId,
    pub expected_audience: BindingProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedWriteFence(SignedWriteFenceV1);

impl VerifiedWriteFence {
    #[must_use]
    pub fn signed(&self) -> &SignedWriteFenceV1 {
        &self.0
    }
}

pub fn verify_write_fence(
    _verifier: &dyn BindingProofVerifier,
    _signed: SignedWriteFenceV1,
    _retirement: &crate::VerifiedServingAuthorityRetirementV1,
    _expectation: &WriteFenceExpectationV1,
) -> Result<VerifiedWriteFence, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}
