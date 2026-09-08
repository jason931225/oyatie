use cell_placement::{
    SignedSourceReservationReleasePermitV1, SourceReservationReleaseIntentV1,
    VerifiedSourceReservationReleasePermit,
};

use crate::{
    BindingAuditRecordV1, BindingDigest32, BindingIdempotencyRecordV1, BindingOperationKey,
    BindingPersistenceAuthorityV1, BindingProducerId, BindingProofConsumptionV1,
    BindingProofEnvelopeV1, BindingProofVerificationError, BindingProofVerifier,
    BindingReadAuthorityV1, BindingReconciliationLeaseV1,
    BindingReconciliationPersistenceAuthorityV1, BindingStoreError, BoxTenancyFuture,
    TenantCellBinding, TenantId, VerifiedRollbackWindowElapsed,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceReservationReleaseIssuanceRevision(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SourceReservationReleaseIssuanceStatusV1 {
    PendingSignature,
    Published,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceReleaseCommitContextV1 {
    Request,
    Reconciliation {
        candidate_digest: BindingDigest32,
        lease_epoch: u64,
        lease_digest: BindingDigest32,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum SourceReleaseClaimContextV1 {
    Request,
    Reconciliation(BindingReconciliationLeaseV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceReservationReleaseIssuanceRecordV1 {
    intent: SourceReservationReleaseIntentV1,
    status: SourceReservationReleaseIssuanceStatusV1,
    signed_permit: Option<SignedSourceReservationReleasePermitV1>,
    revision: SourceReservationReleaseIssuanceRevision,
    record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceReservationReleaseIssuanceRecordPartsV1 {
    pub intent: SourceReservationReleaseIntentV1,
    pub status: SourceReservationReleaseIssuanceStatusV1,
    pub signed_permit: Option<SignedSourceReservationReleasePermitV1>,
    pub revision: SourceReservationReleaseIssuanceRevision,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SourceReservationReleaseIssuanceConstructionErrorV1 {
    NotImplemented,
    InvalidRevision,
    InvalidStatus,
    InvalidPermitRelation,
    RecordDigestMismatch,
}

impl SourceReservationReleaseIssuanceRecordV1 {
    pub fn rehydrate(
        _parts: SourceReservationReleaseIssuanceRecordPartsV1,
    ) -> Result<Self, SourceReservationReleaseIssuanceConstructionErrorV1> {
        Err(SourceReservationReleaseIssuanceConstructionErrorV1::NotImplemented)
    }

    #[must_use]
    pub fn intent(&self) -> &SourceReservationReleaseIntentV1 {
        &self.intent
    }

    #[must_use]
    pub fn status(&self) -> SourceReservationReleaseIssuanceStatusV1 {
        self.status
    }

    #[must_use]
    pub fn signed_permit(&self) -> Option<&SignedSourceReservationReleasePermitV1> {
        self.signed_permit.as_ref()
    }

    #[must_use]
    pub fn revision(&self) -> SourceReservationReleaseIssuanceRevision {
        self.revision
    }

    #[must_use]
    pub fn record_digest(&self) -> BindingDigest32 {
        self.record_digest
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceReservationReleaseIssuancePreconditionV1 {
    pub revision: SourceReservationReleaseIssuanceRevision,
    pub status: SourceReservationReleaseIssuanceStatusV1,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceReleaseCommitObservationV1 {
    pub schema_version: u32,
    pub tenant_id: TenantId,
    pub operation: BindingOperationKey,
    pub successor_binding_generation: crate::BindingGeneration,
    pub successor_binding_revision: crate::BindingRevision,
    pub successor_binding_record_digest: BindingDigest32,
    pub issuance_revision: SourceReservationReleaseIssuanceRevision,
    pub issuance_record_digest: BindingDigest32,
    pub rollback_window_elapsed_digest: BindingDigest32,
    pub context: SourceReleaseCommitContextV1,
    pub committed_transaction_digest: BindingDigest32,
    /// Read out of the durable record.
    pub committed_at_unix_seconds: u64,
    /// When the observer itself looked.
    pub observed_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedSourceReleaseCommitObservationV1 {
    pub payload: SourceReleaseCommitObservationV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CommittedSourceReservationReleaseIssuanceClaimV1 {
    pub successor_binding: TenantCellBinding,
    pub issuance: SourceReservationReleaseIssuanceRecordV1,
    pub rollback_window: VerifiedRollbackWindowElapsed,
    pub context: SourceReleaseClaimContextV1,
    pub observation: SignedSourceReleaseCommitObservationV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceReleaseCommitObservationExpectationV1 {
    pub tenant_id: TenantId,
    pub operation: BindingOperationKey,
    pub successor_binding_generation: crate::BindingGeneration,
    pub successor_binding_revision: crate::BindingRevision,
    pub successor_binding_record_digest: BindingDigest32,
    pub issuance_revision: SourceReservationReleaseIssuanceRevision,
    pub issuance_record_digest: BindingDigest32,
    pub rollback_window_elapsed_digest: BindingDigest32,
    pub context: SourceReleaseCommitContextV1,
    pub expected_producer: BindingProducerId,
    pub expected_audience: BindingProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCommittedSourceReservationReleaseIssuance(
    CommittedSourceReservationReleaseIssuanceClaimV1,
);

impl VerifiedCommittedSourceReservationReleaseIssuance {
    #[must_use]
    pub fn claim(&self) -> &CommittedSourceReservationReleaseIssuanceClaimV1 {
        &self.0
    }
}

pub fn verify_committed_source_release_issuance(
    _verifier: &dyn BindingProofVerifier,
    _claim: CommittedSourceReservationReleaseIssuanceClaimV1,
    _expectation: &SourceReleaseCommitObservationExpectationV1,
) -> Result<VerifiedCommittedSourceReservationReleaseIssuance, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub enum SourceReleasePublicationAuthorityV1 {
    Request(BindingPersistenceAuthorityV1),
    Reconciler(BindingReconciliationPersistenceAuthorityV1),
}

#[derive(Debug, Eq, PartialEq)]
pub struct SourceReleasePublicationWriteSetV1 {
    parts: SourceReleasePublicationWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SourceReleasePublicationWriteSetPartsV1 {
    pub authority: SourceReleasePublicationAuthorityV1,
    pub issuance_precondition: SourceReservationReleaseIssuancePreconditionV1,
    pub committed_issuance: VerifiedCommittedSourceReservationReleaseIssuance,
    pub permit: VerifiedSourceReservationReleasePermit,
    pub published_issuance: SourceReservationReleaseIssuanceRecordV1,
    pub idempotency: BindingIdempotencyRecordV1,
    pub audit_outbox: BindingAuditRecordV1,
    pub proof_consumptions: Vec<BindingProofConsumptionV1>,
}

impl SourceReleasePublicationWriteSetV1 {
    pub fn assemble(
        _parts: SourceReleasePublicationWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &SourceReleasePublicationWriteSetPartsV1 {
        &self.parts
    }
}

/// Independently re-reads a committed source-reservation release issuance and
/// signs what it read.
///
/// WHY THIS PORT EXISTS. A store that signs an observation of its own write
/// vouches for itself, and no amount of downstream signature checking recovers
/// what that destroys. [`MigrationReleaseStore::commit_release_issuance`] used
/// to return a [`CommittedSourceReservationReleaseIssuanceClaimV1`] — the
/// durable record AND a signature over the transaction the same call had just
/// performed. [`SourceReleaseCommitObservationV1`] carries
/// `committed_transaction_digest` and `committed_at_unix_seconds`, facts only
/// the committing store holds at the moment it commits, so it could never have
/// been an external arrival needing no local producer: the committer was the
/// producer.
///
/// The port accepts a read authority and a lookup key only. It is not given,
/// and cannot be given, a caller-supplied record or a caller's claim that a
/// commit occurred: every field of the emitted observation describes what the
/// observer itself read.
///
/// It returns the RECORD TOGETHER WITH its observation rather than the
/// observation alone. Handing back a lone signature would put the caller in
/// charge of pairing it with a record, which reopens a narrower version of the
/// same steering hazard. That is the shape
/// `PromotionEconomicsCheckpointCommitObserver` settled on in the cell crate.
///
/// `None` means the observer looked and found NO committed issuance for that
/// operation. It is an outcome, not a failure, and it is the fact that
/// separates "never durably committed" from "committed, reply lost". A `None`
/// that DISAGREES with [`MigrationReleaseStore::load_release_issuance`]
/// reporting a record is a REFUSAL, never a quiet fallback to "nothing was
/// committed".
///
/// THE PROOF DOMAIN IS NEW, NOT RENAMED. `BindingProofDomainV1` tag 26 named a
/// signed statement the STORE made about its own write. This is a different
/// producer making a different trust claim, so reusing that tag would let a
/// signature produced under the old self-attesting semantics validate as an
/// independent observation. Tag 26 is reserved by number and by name in
/// `tenancy/binding/v1/proof.proto` and replaced by
/// `BINDING_PROOF_DOMAIN_V1_SOURCE_RELEASE_COMMIT_OBSERVATION`, named rather
/// than numbered so the pointer survives a renumber.
///
/// Separate from the store on purpose. Whether the deployed observer is in fact
/// a different party from the deployed store is A DEPLOYMENT OBLIGATION, NOT A
/// TYPE-LEVEL REFUSAL — one process may implement both traits. What the types
/// do is remove the shape in which self-observation was the ONLY implementable
/// one; see [`crate::TransferExecutionCommitObserver`], the lane this is
/// modelled on.
pub trait SourceReleaseCommitObserver: Send + Sync {
    fn observe_committed_release_issuance<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        operation: &'a BindingOperationKey,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<CommittedSourceReservationReleaseIssuanceClaimV1>, BindingStoreError>,
    >;
}

pub trait MigrationReleaseStore: Send + Sync {
    /// Durably commits the source-reservation release issuance and returns THE
    /// DURABLE RECORD ALONE.
    ///
    /// It never returns a signature, because a signature here would be the
    /// store attesting to its own write. The commit signature comes from
    /// [`SourceReleaseCommitObserver::observe_committed_release_issuance`],
    /// which re-reads the committed row by lookup key under an ordinary read
    /// authority and returns
    /// [`CommittedSourceReservationReleaseIssuanceClaimV1`] — the record
    /// together with its observation.
    fn commit_release_issuance<'a>(
        &'a self,
        write_set: &'a crate::MigrationReleaseWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<SourceReservationReleaseIssuanceRecordV1, BindingStoreError>>;

    /// Re-reads the source-reservation release issuance row by operation key,
    /// under the SAME ordinary read authority a
    /// [`BindingPersistenceAuthorityV1`] holder derives from
    /// [`BindingPersistenceAuthorityV1::read_authority`].
    ///
    /// `None` MEANS THE STORE LOOKED AND FOUND NO ISSUANCE ROW for that
    /// operation: no release has been durably committed. That is the fact that
    /// separates "never durably issued" from "issued, reply lost", and it is
    /// not an authorization refusal — that is on the error channel.
    ///
    /// WHY IT IS NOT THE OBSERVER. `SourceReleasePublicationWriteSetPartsV1::issuance_precondition`
    /// is required by value, and the `Request` arm holds a
    /// [`BindingPersistenceAuthorityV1`]. Before this read the only surfaces
    /// yielding the row were the write's own return — gone after a lost reply —
    /// and [`MigrationReleaseStore::load_committed_release_issuance`], gated on
    /// a reconciliation authority AND a reconciliation lease the request path
    /// must not hold. The commit observer is not a substitute: like the
    /// transfer lane's, its claim carries the values AS OF THE COMMIT, and the
    /// publication compare-and-set is on the row's CURRENT revision, status and
    /// record digest. That is the same reason
    /// [`crate::TransferExecutionStore::load_item`] was added beside
    /// [`crate::TransferExecutionCommitObserver`] rather than instead of it.
    fn load_release_issuance<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        operation: &'a BindingOperationKey,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<SourceReservationReleaseIssuanceRecordV1>, BindingStoreError>,
    >;

    fn load_committed_release_issuance<'a>(
        &'a self,
        authority: &'a BindingReconciliationPersistenceAuthorityV1,
        operation: &'a BindingOperationKey,
        reconciliation_lease: &'a BindingReconciliationLeaseV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<CommittedSourceReservationReleaseIssuanceClaimV1>, BindingStoreError>,
    >;

    fn publish_release_permit<'a>(
        &'a self,
        write_set: &'a SourceReleasePublicationWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<SignedSourceReservationReleasePermitV1, BindingStoreError>>;
}

pub trait SourceReservationReleaseAuthority: Send + Sync {
    fn sign_committed<'a>(
        &'a self,
        issuance: &'a VerifiedCommittedSourceReservationReleaseIssuance,
    ) -> BoxTenancyFuture<'a, Result<VerifiedSourceReservationReleasePermit, BindingStoreError>>;
}
