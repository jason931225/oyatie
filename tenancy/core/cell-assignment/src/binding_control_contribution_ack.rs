use crate::{BindingDigest32, BindingProofVerificationError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionCheckpointV1 {
    pub source_partition: crate::TenantControlPartitionRefV1,
    pub target: crate::BindingControlContributionTargetV1,
    pub source_sequence: u64,
    pub projection_digest: Option<BindingDigest32>,
    pub revision: u64,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionApplicationIntentV1 {
    pub schema_version: u32,
    pub source_partition: crate::TenantControlPartitionRefV1,
    pub target: crate::BindingControlContributionTargetV1,
    pub projection_digest: BindingDigest32,
    pub complete_payload_digest: BindingDigest32,
    pub committed_source_revision: u64,
    pub previous_checkpoint: BindingControlContributionCheckpointV1,
    pub applied_checkpoint: BindingControlContributionCheckpointV1,
    pub target_projection_revision: u64,
    pub target_projection_digest: BindingDigest32,
    pub intent_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionAcknowledgmentPayloadV1 {
    pub schema_version: u32,
    pub intent: BindingControlContributionApplicationIntentV1,
    pub target_committed_transaction_digest: BindingDigest32,
    pub acknowledgment_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedBindingControlContributionAcknowledgmentV1 {
    pub payload: BindingControlContributionAcknowledgmentPayloadV1,
    pub envelope: crate::BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedBindingControlContributionAcknowledgment(
    SignedBindingControlContributionAcknowledgmentV1,
);

impl VerifiedBindingControlContributionAcknowledgment {
    #[must_use]
    pub fn signed(&self) -> &SignedBindingControlContributionAcknowledgmentV1 {
        &self.0
    }
}

pub fn verify_binding_control_contribution_acknowledgment(
    _verifier: &dyn crate::BindingProofVerifier,
    _signed: SignedBindingControlContributionAcknowledgmentV1,
    _expected: &crate::BindingControlContributionExpectationV1,
    _expected_target_producer: &crate::BindingProducerId,
) -> Result<VerifiedBindingControlContributionAcknowledgment, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingCellIndexProjectionResultV1 {
    pub snapshot: crate::CellBindingIndexSnapshotV1,
    pub acknowledgments: Vec<SignedBindingControlContributionAcknowledgmentV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionAcknowledgmentQueryV1 {
    pub source_partition: crate::TenantControlPartitionRefV1,
    pub target: crate::BindingControlContributionTargetV1,
    pub projection_digest: BindingDigest32,
}
