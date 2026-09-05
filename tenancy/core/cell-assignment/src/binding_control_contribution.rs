use crate::{BindingDigest32, BindingProofVerificationError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionLimitsV1 {
    pub maximum_index_mutations: u32,
    pub maximum_drain_mutations: u32,
    pub maximum_destinations: u32,
    pub maximum_batch_contributions: u32,
    pub maximum_encoded_bytes: u64,
    pub maximum_proof_depth: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionTargetV1 {
    pub partition: crate::CellBindingIndexPartitionKey,
    pub cell_id: cell_placement::CellId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionDestinationV1 {
    pub target: BindingControlContributionTargetV1,
    pub previous_source_sequence: u64,
    pub previous_projection_digest: Option<BindingDigest32>,
    pub source_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionPayloadV1 {
    pub cell_index_mutations: crate::CellBindingIndexMutationSetPartsV1,
    pub drain_mutations: cell_placement::DrainContributorMutationSetPartsV1,
    pub destinations: Vec<BindingControlContributionDestinationV1>,
    pub encoded_bytes: u64,
    pub payload_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionCommitPayloadV1 {
    pub schema_version: u32,
    pub source_partition: crate::TenantControlPartitionRefV1,
    pub binding_revision: crate::BindingRevision,
    pub projection_digest: BindingDigest32,
    pub complete_payload_digest: BindingDigest32,
    pub committed_source_revision: u64,
    pub committed_transaction_digest: BindingDigest32,
    pub committed_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedBindingControlContributionCommitV1 {
    pub payload: BindingControlContributionCommitPayloadV1,
    pub envelope: crate::BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedBindingControlContributionClaimV1 {
    pub projection: crate::BindingControlContributionProjectionV1,
    pub attestation: SignedBindingControlContributionCommitV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionExpectationV1 {
    pub source_partition: crate::TenantControlPartitionRefV1,
    pub target: BindingControlContributionTargetV1,
    pub projection_digest: BindingDigest32,
    pub complete_payload_digest: BindingDigest32,
    pub committed_source_revision: u64,
    pub expected_source_producer: crate::BindingProducerId,
    pub expected_handoff_producer: crate::BindingProducerId,
    pub expected_audience: crate::BindingProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCommittedBindingControlContribution(CommittedBindingControlContributionClaimV1);

impl VerifiedCommittedBindingControlContribution {
    #[must_use]
    pub fn claim(&self) -> &CommittedBindingControlContributionClaimV1 {
        &self.0
    }
}

pub fn verify_committed_binding_control_contribution(
    _verifier: &dyn crate::BindingProofVerifier,
    _claim: CommittedBindingControlContributionClaimV1,
    _expected: &BindingControlContributionExpectationV1,
    _limits: &BindingControlContributionLimitsV1,
) -> Result<VerifiedCommittedBindingControlContribution, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedBindingControlContributionHandoffV1 {
    pub committed: CommittedBindingControlContributionClaimV1,
    pub target: BindingControlContributionTargetV1,
    pub envelope: crate::BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedBindingControlContributionHandoff(SignedBindingControlContributionHandoffV1);

impl VerifiedBindingControlContributionHandoff {
    #[must_use]
    pub fn signed(&self) -> &SignedBindingControlContributionHandoffV1 {
        &self.0
    }
}

pub fn verify_binding_control_contribution_handoff(
    _verifier: &dyn crate::BindingProofVerifier,
    _signed: SignedBindingControlContributionHandoffV1,
    _expected: &BindingControlContributionExpectationV1,
    _limits: &BindingControlContributionLimitsV1,
) -> Result<VerifiedBindingControlContributionHandoff, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingControlContributionDeliveryV1 {
    PendingPublication {
        target: BindingControlContributionTargetV1,
    },
    PublishedAwaitingAcknowledgment(Box<SignedBindingControlContributionHandoffV1>),
    Acknowledged(Box<crate::SignedBindingControlContributionAcknowledgmentV1>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionOutboxV1 {
    pub projection: crate::BindingControlContributionProjectionV1,
    pub deliveries: Vec<BindingControlContributionDeliveryV1>,
    pub minimum_retention_after_acknowledgment_seconds: u64,
    pub revision: u64,
    pub record_digest: BindingDigest32,
}
