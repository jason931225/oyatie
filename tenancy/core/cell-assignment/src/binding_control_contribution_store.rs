use crate::{BindingDigest32, BoxTenancyFuture};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingControlContributionError {
    NotImplemented,
    Unavailable,
    MissingRetainedPayload,
    MissingCommitAttestation,
    RetentionExpired,
    IncompletePayload,
    PayloadDigestMismatch,
    ScopeMismatch,
    CheckpointConflict,
    OutOfOrderContribution,
    MissingPredecessor,
    CountLimitExceeded,
    EncodedBytesLimitExceeded,
    ProofDepthLimitExceeded,
    IdempotencyKeyReuse,
    UncommittedContribution,
    MissingAcknowledgment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionQueryV1 {
    pub source_partition: crate::TenantControlPartitionRefV1,
    pub projection_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BindingControlContributionDeliveryEvidenceV1 {
    Published(Box<crate::VerifiedBindingControlContributionHandoff>),
    Applied(Box<crate::VerifiedBindingControlContributionAcknowledgment>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingControlContributionDeliveryWriteSetV1 {
    parts: BindingControlContributionDeliveryWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingControlContributionDeliveryWriteSetPartsV1 {
    pub authority: crate::BindingReconciliationPersistenceAuthorityV1,
    pub query: BindingControlContributionQueryV1,
    pub expected_outbox_revision: u64,
    pub expected_outbox_digest: BindingDigest32,
    pub evidence: BindingControlContributionDeliveryEvidenceV1,
    pub next_outbox: crate::BindingControlContributionOutboxV1,
    pub limits: crate::BindingControlContributionLimitsV1,
    pub idempotency: crate::BindingIdempotencyRecordV1,
    pub proof_consumptions: Vec<crate::BindingProofConsumptionV1>,
    pub audit_outbox: crate::BindingAuditRecordV1,
}

impl BindingControlContributionDeliveryWriteSetV1 {
    pub fn assemble(
        _parts: BindingControlContributionDeliveryWriteSetPartsV1,
    ) -> Result<Self, BindingControlContributionError> {
        Err(BindingControlContributionError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &BindingControlContributionDeliveryWriteSetPartsV1 {
        &self.parts
    }
}

pub trait BindingControlContributionSourceStore: Send + Sync {
    fn load_committed<'a>(
        &'a self,
        authority: &'a crate::BindingReconciliationReadAuthorityV1,
        query: &'a BindingControlContributionQueryV1,
        limits: &'a crate::BindingControlContributionLimitsV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::CommittedBindingControlContributionClaimV1, BindingControlContributionError>,
    >;

    fn checkpoint_delivery<'a>(
        &'a self,
        write_set: &'a BindingControlContributionDeliveryWriteSetV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::BindingControlContributionOutboxV1, BindingControlContributionError>,
    >;
}

pub trait BindingControlContributionIssuer: Send + Sync {
    fn sign_committed<'a>(
        &'a self,
        committed: &'a crate::VerifiedCommittedBindingControlContribution,
        target: &'a crate::BindingControlContributionTargetV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::SignedBindingControlContributionHandoffV1, BindingControlContributionError>,
    >;
}
