use cell_placement::{AssuranceAuditPolicyV1, CellId};

use crate::{
    BindingDigest32, BindingGeneration, BindingOperationKey, BindingProducerId,
    BindingProofEnvelopeV1, BindingProofVerificationError, BindingProofVerifier, BindingRevision,
    CapabilityParticipantId, TenantId, WriteAuthorityEpoch,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitialWriteAuthorityBasisV1 {
    pub binding_attempt_digest: BindingDigest32,
    pub placement_decision_digest: BindingDigest32,
    pub reservation_commit_permit_digest: BindingDigest32,
    pub binding_outcome_digest: BindingDigest32,
    pub participant_preparation_closure_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationWriteAuthorityBasisV1 {
    pub migration_fence_claim_digest: BindingDigest32,
    pub movement_permit_digest: BindingDigest32,
    pub write_fence_digest: BindingDigest32,
    pub migration_commit_seal_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WriteAuthorityBasisV1 {
    Initial(InitialWriteAuthorityBasisV1),
    Migration(MigrationWriteAuthorityBasisV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseIntentV1 {
    pub schema_version: u32,
    pub operation: BindingOperationKey,
    pub instance: crate::ServingAuthorityInstanceV1,
    pub tenant_id: TenantId,
    pub cell_id: CellId,
    pub binding_generation: BindingGeneration,
    pub binding_revision: BindingRevision,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub participant_manifest_digest: BindingDigest32,
    pub binding_record_digest: BindingDigest32,
    pub basis: WriteAuthorityBasisV1,
    pub lease_policy_digest: BindingDigest32,
    pub maximum_participant_token_validity_seconds: u64,
    pub clock_authority_digest: BindingDigest32,
    pub maximum_clock_uncertainty_millis: u64,
    pub assurance_audit_policy: AssuranceAuditPolicyV1,
    pub not_before_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub lease_intent_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeasePayloadV1 {
    pub schema_version: u32,
    pub intent: WriteAuthorityLeaseIntentV1,
    pub issuance_revision: crate::WriteAuthorityLeaseIssuanceRevision,
    pub issuance_record_digest: BindingDigest32,
    pub commit_attestation: crate::SignedWriteAuthorityLeaseCommitAttestationV1,
    pub lease_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedWriteAuthorityLeaseV1 {
    pub payload: WriteAuthorityLeasePayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseExpectationV1 {
    pub expected_intent: WriteAuthorityLeaseIntentV1,
    pub expected_issuance: crate::WriteAuthorityLeaseIssuancePreconditionV1,
    pub expected_commit_attestation_digest: BindingDigest32,
    pub expected_producer: BindingProducerId,
    pub expected_audience: BindingProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedWriteAuthorityLease(SignedWriteAuthorityLeaseV1);

impl VerifiedWriteAuthorityLease {
    #[must_use]
    pub fn signed(&self) -> &SignedWriteAuthorityLeaseV1 {
        &self.0
    }
}

pub fn verify_write_authority_lease(
    _verifier: &dyn BindingProofVerifier,
    _signed: SignedWriteAuthorityLeaseV1,
    _expectation: &WriteAuthorityLeaseExpectationV1,
) -> Result<VerifiedWriteAuthorityLease, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityTokenPayloadV1 {
    pub schema_version: u32,
    pub operation: BindingOperationKey,
    pub instance: crate::ServingAuthorityInstanceV1,
    pub tenant_id: TenantId,
    pub cell_id: CellId,
    pub binding_generation: BindingGeneration,
    pub binding_revision: BindingRevision,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub participant_id: CapabilityParticipantId,
    pub participant_membership_digest: BindingDigest32,
    pub participant_manifest_digest: BindingDigest32,
    pub binding_record_digest: BindingDigest32,
    pub write_authority_lease_digest: BindingDigest32,
    pub assurance_audit_policy: AssuranceAuditPolicyV1,
    pub basis: WriteAuthorityBasisV1,
    pub not_before_unix_seconds: u64,
    pub authority_expires_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedWriteAuthorityTokenV1 {
    pub payload: WriteAuthorityTokenPayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityTokenExpectationV1 {
    pub operation: BindingOperationKey,
    pub instance: crate::ServingAuthorityInstanceV1,
    pub tenant_id: TenantId,
    pub cell_id: CellId,
    pub binding_generation: BindingGeneration,
    pub binding_revision: BindingRevision,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub participant_id: CapabilityParticipantId,
    pub participant_membership_digest: BindingDigest32,
    pub participant_manifest_digest: BindingDigest32,
    pub binding_record_digest: BindingDigest32,
    pub write_authority_lease_digest: BindingDigest32,
    pub assurance_audit_policy: AssuranceAuditPolicyV1,
    pub write_authority_lease_expires_at_unix_seconds: u64,
    pub maximum_participant_token_validity_seconds: u64,
    pub basis: WriteAuthorityBasisV1,
    pub authority_expires_at_unix_seconds: u64,
    pub expected_producer: BindingProducerId,
    pub expected_audience: BindingProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedWriteAuthorityToken(SignedWriteAuthorityTokenV1);

impl VerifiedWriteAuthorityToken {
    #[must_use]
    pub fn signed(&self) -> &SignedWriteAuthorityTokenV1 {
        &self.0
    }
}

pub fn verify_write_authority_token(
    _verifier: &dyn BindingProofVerifier,
    _signed: SignedWriteAuthorityTokenV1,
    _lease: &VerifiedWriteAuthorityLease,
    _participant: &crate::VerifiedParticipantManifestMember,
    _expectation: &WriteAuthorityTokenExpectationV1,
) -> Result<VerifiedWriteAuthorityToken, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}
