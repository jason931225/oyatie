use crate::{BindingDigest32, BoxTenancyFuture};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingControlContributionError {
    /// No implementation exists yet. Every method of this contract returns this
    /// today; it is not a runtime condition and says nothing about durable
    /// state.
    NotImplemented,
    /// The store could not be reached or could not answer. The caller learns
    /// NOTHING about whether the write took effect. Every other variant below
    /// is a definite answer; this one is the absence of one.
    Unavailable,
    /// The contribution's retained payload is absent where the contract
    /// requires it. Contrast [`Self::IncompletePayload`] (present but partial),
    /// [`Self::RetentionExpired`] (present until its retention window lapsed)
    /// and [`Self::MissingCommitAttestation`] (a different member is missing).
    MissingRetainedPayload,
    /// The committed-contribution claim carries no commit attestation, so
    /// nothing independently observed the commit. Distinct from
    /// [`Self::UncommittedContribution`]: there the commit did not happen; here
    /// it may have happened with no attestation to prove it.
    MissingCommitAttestation,
    /// The payload existed but its retention window
    /// (`minimum_retention_after_acknowledgment_seconds`) has lapsed and it was
    /// legitimately discarded. Distinct from [`Self::MissingRetainedPayload`]:
    /// this is expected ageing, not a contract breach, and callers may treat it
    /// as terminal rather than as corruption.
    RetentionExpired,
    /// The payload is present but structurally short of what the contract
    /// requires -- fewer members than declared. Contrast
    /// [`Self::PayloadDigestMismatch`], where the payload is complete and
    /// wrong.
    IncompletePayload,
    /// The payload is complete but does not hash to its declared digest.
    /// Integrity failure, not a completeness failure.
    PayloadDigestMismatch,
    /// The request addresses a source partition or target the invocation does
    /// not cover. Evaluated before any ordering or limit check, so it never
    /// competes with [`Self::CheckpointConflict`].
    ScopeMismatch,
    /// The delivery checkpoint moved under the caller: a compare-and-set
    /// failure on checkpoint state. Retrying after a fresh read may succeed.
    /// Contrast [`Self::OutOfOrderContribution`], where re-reading changes
    /// nothing because the request itself is misordered.
    CheckpointConflict,
    /// The contribution's ordinal is behind or ahead of the checkpoint's next
    /// expected position. The request is misordered relative to a checkpoint
    /// the store CAN see. Contrast [`Self::MissingPredecessor`], where the
    /// predecessor is not present at all.
    OutOfOrderContribution,
    /// The contribution names a predecessor the store does not hold, so the
    /// chain cannot be extended. Contrast [`Self::OutOfOrderContribution`]: the
    /// position is knowable there and unknowable here.
    MissingPredecessor,
    /// The batch exceeds the permitted number of contributions. One of three
    /// disjoint limit variants -- this one counts records.
    CountLimitExceeded,
    /// The batch exceeds `maximum_encoded_bytes`. Counts wire size, not
    /// records, so it can fire on a batch that satisfies
    /// [`Self::CountLimitExceeded`].
    EncodedBytesLimitExceeded,
    /// A contribution's proof chain is deeper than `maximum_proof_depth`.
    /// Counts nesting, independent of both record count and encoded size.
    ProofDepthLimitExceeded,
    /// An idempotency key was reused carrying a different request digest. Same
    /// key, different request.
    IdempotencyKeyReuse,
    /// The operation requires a contribution that is durably committed and this
    /// one is not. A definite negative about durable state; contrast
    /// [`Self::Unavailable`], which asserts nothing.
    UncommittedContribution,
    /// No acknowledgment exists for a contribution where one is required. The
    /// target side has not attested application. Distinct from the three
    /// `Missing*` variants above, which concern the source side's payload,
    /// attestation and predecessor respectively.
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
    /// Reads back the UNSIGNED durable contribution projection, or `None` when
    /// nothing was committed for that query.
    ///
    /// This returns the projection rather than
    /// [`crate::CommittedBindingControlContributionClaimV1`] for the same reason
    /// the serving-authority loaders return records: the claim's other member is
    /// a signed commit attestation, and no write path on this store carries one
    /// in. Returning the claim would have obliged the store to produce a
    /// signature only it could have forged.
    ///
    /// The caller pairs this projection with a fresh attestation from
    /// [`BindingControlContributionCommitObserver::observe_contribution_commit`],
    /// assembles the claim and passes it through
    /// `verify_committed_binding_control_contribution`.
    fn load_committed<'a>(
        &'a self,
        authority: &'a crate::BindingReconciliationReadAuthorityV1,
        query: &'a BindingControlContributionQueryV1,
        limits: &'a crate::BindingControlContributionLimitsV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<
            Option<crate::BindingControlContributionProjectionV1>,
            BindingControlContributionError,
        >,
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

/// Independently observes a committed binding control contribution and signs
/// what it read.
///
/// `SignedBindingControlContributionCommitV1` had a proof domain, a proto tag, a
/// verifier, an expectation and a `VerifiedBindingProofRefV1` arm -- everything
/// but a producer. This port is that producer, and it is the fourth instance of
/// the same shape on this lane.
///
/// The type now occurs only at: its own definition, the `attestation`
/// member of [`crate::CommittedBindingControlContributionClaimV1`], and this
/// port's return. It is NOT reachable through
/// [`BindingControlContributionSourceStore::load_committed`], which returns the
/// projection and never the claim -- see that method's own documentation for
/// why. An earlier version of this comment said the opposite, describing the
/// pre-fix shape; the two were written in the same commit and the doc was not
/// updated when the loader was.
///
/// Separate from the source store on purpose: the party that committed the
/// contribution must not be the party that attests it committed. Accepts a
/// reconciliation read authority and the same
/// [`BindingControlContributionQueryV1`] lookup key the loader takes -- never a
/// caller's claim about the committed row. Returns `None` when there is nothing
/// committed to observe.
///
/// No new proof domain is required; the existing one was always intended for
/// this artifact.
pub trait BindingControlContributionCommitObserver: Send + Sync {
    fn observe_contribution_commit<'a>(
        &'a self,
        authority: &'a crate::BindingReconciliationReadAuthorityV1,
        query: &'a BindingControlContributionQueryV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<
            Option<crate::SignedBindingControlContributionCommitV1>,
            BindingControlContributionError,
        >,
    >;
}
