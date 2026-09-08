//! Durable local effect receipt, its independently observed commitment, and
//! the read-only recovery authority that retrieves it.
//!
//! The order is durable unsigned commit, then independent reread, then
//! publication. A store that could return a signed or private-field verified
//! receipt would be signing its own evidence; the store here returns only what
//! it durably holds.

use crate::{
    BoxCellFuture, CapabilityAuthorityRejectionHighWaterV1, CapabilityEffectErrorV1,
    CapabilityEffectKeyV1, CapabilityEffectScopeV1, CapabilityPartitionRefV1, CellProofEnvelopeV1,
    Digest32, LocalAuthorityStateV1, LocalCommitRevisionV1, LocalEffectCommitReceiptPayloadV1,
    LocalEffectCommitRequestV1, ProducerId, SignedLocalEffectCommitReceiptV1,
    VerifiedCapabilityEffectGrantV1, VerifiedCapabilityReceiptRecoveryV1,
    VerifiedCommittedLocalEffectReceiptV1,
};

/// One already-committed immutable result, addressed by partition, scope, key
/// and request. Nothing here authorizes a mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityReceiptQueryV1 {
    pub scope: CapabilityEffectScopeV1,
    pub effect_key: CapabilityEffectKeyV1,
    pub request_digest: Digest32,
}

/// Read-only recovery authority.
///
/// Its validity is independent of the original effect grant: the grant may
/// have expired or been fenced and the committed result is still readable,
/// while a revoked principal is refused. It carries its own freshly
/// authenticated window and can never refresh an effect grant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityReceiptRecoveryPayloadV1 {
    pub query: CapabilityReceiptQueryV1,
    pub principal_digest: Digest32,
    pub authorization_decision_digest: Digest32,
    pub expected_receipt_audience: ProducerId,
    pub not_before_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedCapabilityReceiptRecoveryV1 {
    pub payload: CapabilityReceiptRecoveryPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

/// The unsigned receipt the capability's physical transaction durably holds.
///
/// The transaction stores the full canonical payload alongside the actual
/// domain effect, the replayable idempotency result, the spent proof and
/// effect key, authority and rejection state, drain state and the audit
/// outbox record. It does not store only their hashes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DurableLocalEffectReceiptV1 {
    pub query: CapabilityReceiptQueryV1,
    pub payload: LocalEffectCommitReceiptPayloadV1,
    pub payload_digest: Digest32,
    pub local_record_revision: LocalCommitRevisionV1,
}

/// What a storage commit observer asserts it read, under committed read
/// isolation, from the admitted physical store.
///
/// Its signing identity is limited to storage commit observation and is
/// admitted for one adapter, configuration, partition and transaction domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalEffectReceiptCommitObservationV1 {
    pub query: CapabilityReceiptQueryV1,
    pub payload_digest: Digest32,
    pub local_record_revision: LocalCommitRevisionV1,
    pub authority_state_digest: Digest32,
    pub rejection_high_water_digest: Digest32,
    pub effect_result_digest: Digest32,
    pub proof_consumption_digest: Digest32,
    pub idempotency_record_digest: Digest32,
    pub drain_state_digest: Digest32,
    pub audit_record_digest: Digest32,
    pub adapter_acceptance_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedLocalEffectReceiptCommitObservationV1 {
    pub payload: LocalEffectReceiptCommitObservationV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

/// The durable record plus its attestation. This is a claim, not evidence:
/// only [`crate::verify_committed_local_effect_receipt`] turns it into the
/// private wrapper the publisher accepts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedLocalEffectReceiptClaimV1 {
    pub durable: DurableLocalEffectReceiptV1,
    pub observation: SignedLocalEffectReceiptCommitObservationV1,
}

pub trait CapabilityLocalEffectStoreV1: Send + Sync {
    type Effect: Send + Sync;

    /// Reads back the durable authority record for one scope, or `None` when
    /// the participant has none in this partition.
    ///
    /// Every mutation on this store takes a
    /// [`crate::LocalAuthorityPreconditionV1`] as its compare-and-set, and
    /// `commit_effect` additionally takes a `next_state` built from what that
    /// read returned -- and until now nothing returned one. The record was
    /// write-only, which made a participant's first `Prepare` unassemblable
    /// and left [`CapabilityEffectErrorV1::Conflict`]'s stated recovery,
    /// "retryable after re-reading", naming a read that did not exist.
    ///
    /// The authority is the caller's verified effect grant rather than a new
    /// read-authority type. A new one would have had no producer, which is the
    /// defect this wave has found repeatedly on the argument side of a port;
    /// and the existing [`crate::CellControlReadAuthorityV1`] would put a
    /// control-plane invocation on a path that must survive a global outage.
    fn get_authority_state<'a>(
        &'a self,
        partition: &'a CapabilityPartitionRefV1,
        authority: &'a VerifiedCapabilityEffectGrantV1,
        scope: &'a CapabilityEffectScopeV1,
    ) -> BoxCellFuture<'a, Result<Option<LocalAuthorityStateV1>, CapabilityEffectErrorV1>>;

    /// Reads back the durable rejection membership record for one scope, or
    /// `None` when the store holds no membership row for it at all.
    ///
    /// This is the row [`CapabilityEffectErrorV1::StaleIncarnation`] depends
    /// on: without it a replaced incarnation carrying the same generation and
    /// the same numeric fence as its replacement is indistinguishable from it.
    /// It is required by both arms of
    /// [`crate::LocalAuthorityPreconditionV1`] and supplied again as
    /// `next_rejection_high_water`, so it was required twice per commit and
    /// readable never.
    ///
    /// `None` means the store looked and found no row, which is legal only for
    /// a participant never installed anywhere. Finding an authority record
    /// without a membership row is restored or truncated state and returns
    /// [`CapabilityEffectErrorV1::RestoreEvidenceRequired`].
    fn get_rejection_high_water<'a>(
        &'a self,
        partition: &'a CapabilityPartitionRefV1,
        authority: &'a VerifiedCapabilityEffectGrantV1,
        scope: &'a CapabilityEffectScopeV1,
    ) -> BoxCellFuture<
        'a,
        Result<Option<CapabilityAuthorityRejectionHighWaterV1>, CapabilityEffectErrorV1>,
    >;

    /// Commits the capability's own effect, authority comparison, dedup, proof
    /// consumption, drain mutation, audit outbox and receipt payload at one
    /// linearization point, and returns the durable unsigned receipt. It never
    /// returns a publication signature.
    fn commit_effect<'a>(
        &'a self,
        partition: &'a CapabilityPartitionRefV1,
        request: &'a LocalEffectCommitRequestV1<Self::Effect>,
    ) -> BoxCellFuture<'a, Result<DurableLocalEffectReceiptV1, CapabilityEffectErrorV1>>;

    /// Read-only lookup of one already-committed immutable result under fresh
    /// recovery authority. A different partition, request or effect refuses
    /// before storage access; retention loss is
    /// [`CapabilityEffectErrorV1::RetainedEvidenceUnavailable`], never a
    /// silent absence.
    fn recover_receipt<'a>(
        &'a self,
        partition: &'a CapabilityPartitionRefV1,
        authority: &'a VerifiedCapabilityReceiptRecoveryV1,
        query: &'a CapabilityReceiptQueryV1,
    ) -> BoxCellFuture<'a, Result<Option<DurableLocalEffectReceiptV1>, CapabilityEffectErrorV1>>;
}

pub trait CapabilityEffectReceiptCommitObserverV1: Send + Sync {
    /// Independently rereads by explicit partition and query and signs what it
    /// actually found. It accepts a lookup, never a caller's claimed commit
    /// transaction or receipt.
    ///
    /// `None` MEANS THE OBSERVER LOOKED AND FOUND NO COMMITTED ROW, or found
    /// one not yet visible under committed read isolation. That is a definite
    /// negative observation and a normal answer, not a refusal: an observer
    /// signs what it read, and where there is nothing to read there is nothing
    /// to sign. It is emphatically not
    /// [`CapabilityEffectErrorV1::OutcomeUnknown`], which says the caller
    /// learned nothing at all; here the caller learned that no receipt is
    /// committed under this key.
    ///
    /// This used to be a bare claim with absence folded into
    /// `CapabilityEffectErrorV1::UncommittedReceipt`, and the split ran
    /// through this very file: [`CapabilityLocalEffectStoreV1::recover_receipt`]
    /// reads THE SAME ROW under the IDENTICAL
    /// [`VerifiedCapabilityReceiptRecoveryV1`] and the IDENTICAL
    /// [`CapabilityReceiptQueryV1`] and answers `None`. Same authority, same
    /// key, same row, two answers, and neither doc acknowledged the other. The
    /// rule this wave states in absolute terms elsewhere — absence is a normal,
    /// expressible answer and must not be folded into an error — now holds here
    /// as well, alongside the sibling observers that already obeyed it:
    /// [`crate::PromotionEconomicsCheckpointCommitObserver::observe_committed_checkpoint`],
    /// [`crate::MovementActionResultCommitObserver::observe_committed_result`],
    /// [`crate::MovementActionClosureCommitObserver::observe_committed_closure`]
    /// and
    /// [`crate::RebalanceInvocationIssuanceCommitObserver::observe_committed_issuance`].
    /// `UncommittedReceipt` is gone rather than left as a variant nothing can
    /// raise.
    ///
    /// RETENTION LOSS IS STILL AN ERROR, and it is
    /// [`CapabilityEffectErrorV1::RetainedEvidenceUnavailable`]: "the result
    /// existed and its evidence is gone" is a different claim from "nothing was
    /// committed", and a caller that reads the second where the first is true
    /// concludes the effect never happened.
    fn observe_committed<'a>(
        &'a self,
        partition: &'a CapabilityPartitionRefV1,
        authority: &'a VerifiedCapabilityReceiptRecoveryV1,
        query: &'a CapabilityReceiptQueryV1,
    ) -> BoxCellFuture<
        'a,
        Result<Option<CommittedLocalEffectReceiptClaimV1>, CapabilityEffectErrorV1>,
    >;
}

pub trait CapabilityEffectReceiptPublisherV1: Send + Sync {
    /// Accepts only a verified committed observation, so this API cannot be
    /// called with a precommit payload: the argument type has no other
    /// inhabitant. WHERE THE WRAPPER CAME FROM is a separate question and a
    /// deployment obligation — see [`crate::MovementActionResultAuthority`].
    /// Republication of the
    /// same immutable result under fresh recovery authority is allowed;
    /// signatures need not be byte-identical, payload identity must be.
    fn publish<'a>(
        &'a self,
        committed: &'a VerifiedCommittedLocalEffectReceiptV1,
    ) -> BoxCellFuture<'a, Result<SignedLocalEffectCommitReceiptV1, CapabilityEffectErrorV1>>;
}
