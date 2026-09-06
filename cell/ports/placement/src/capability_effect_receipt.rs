//! Durable local effect receipt, its independently observed commitment, and
//! the read-only recovery authority that retrieves it.
//!
//! The order is durable unsigned commit, then independent reread, then
//! publication. A store that could return a signed or private-field verified
//! receipt would be signing its own evidence; the store here returns only what
//! it durably holds.

use crate::{
    BoxCellFuture, CapabilityEffectErrorV1, CapabilityEffectKeyV1, CapabilityEffectScopeV1,
    CapabilityPartitionRefV1, CellProofEnvelopeV1, Digest32, LocalCommitRevisionV1,
    LocalEffectCommitReceiptPayloadV1, LocalEffectCommitRequestV1, ProducerId,
    SignedLocalEffectCommitReceiptV1, VerifiedCapabilityReceiptRecoveryV1,
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
    fn observe_committed<'a>(
        &'a self,
        partition: &'a CapabilityPartitionRefV1,
        authority: &'a VerifiedCapabilityReceiptRecoveryV1,
        query: &'a CapabilityReceiptQueryV1,
    ) -> BoxCellFuture<'a, Result<CommittedLocalEffectReceiptClaimV1, CapabilityEffectErrorV1>>;
}

pub trait CapabilityEffectReceiptPublisherV1: Send + Sync {
    /// Accepts only a verified committed observation, so a precommit signed
    /// payload cannot be constructed through this API. Republication of the
    /// same immutable result under fresh recovery authority is allowed;
    /// signatures need not be byte-identical, payload identity must be.
    fn publish<'a>(
        &'a self,
        committed: &'a VerifiedCommittedLocalEffectReceiptV1,
    ) -> BoxCellFuture<'a, Result<SignedLocalEffectCommitReceiptV1, CapabilityEffectErrorV1>>;
}
