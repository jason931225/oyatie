use crate::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RebalanceClaimPreconditionV1 {
    Absent,
    Matches(RebalanceJobClaimV1),
}

#[derive(Debug, Eq, PartialEq)]
pub struct RebalanceClaimWriteSetPartsV1 {
    pub authority: CellControlReconciliationPersistenceAuthorityV1,
    pub address: RebalanceJobAddressV1,
    /// Compare-and-set on the rebalance job row, [`crate::RebalanceJobV1`],
    /// which `create_job` opened before any worker could claim it.
    pub expected_job_revision: u64,
    /// Compare-and-set on the rebalance job row, [`crate::RebalanceJobV1`]; the
    /// digest half of the pair above.
    pub expected_job_digest: Digest32,
    pub previous_claim: RebalanceClaimPreconditionV1,
    pub next_claim: RebalanceJobClaimV1,
    /// The instant the claim decision is judged against.
    ///
    /// Deciding whether `previous_claim` has lapsed requires a time, and the
    /// persistence authority carries only the CALLER'S
    /// `deadline_unix_seconds` - when the caller stops waiting, which is not a
    /// fact about the incumbent worker. Without this the store had no value
    /// against which `RebalanceJobClaimV1::expires_at_unix_seconds` meant
    /// anything, so "the previous holder is gone" was not decidable from the
    /// write set at all.
    ///
    /// It sits with the other compare-and-set inputs rather than on the method,
    /// because it IS one: `previous_claim` and this together state the
    /// precondition, and separating them would let a caller reason about
    /// expiry against one instant and have the store judge against another.
    pub now_unix_seconds: u64,
    pub audit_outbox: CellControlAuditRecordV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RebalanceEvaluationWriteSetPartsV1 {
    pub authority: CellControlReconciliationPersistenceAuthorityV1,
    pub precondition: RebalanceSourcePreconditionV1,
    pub evaluation: RebalanceEvaluationV1,
    pub requirements_action: RebalanceSelectedActionV1,
    pub next_job: RebalanceJobV1,
    pub idempotency: CellControlIdempotencyRecordV1,
    pub audit_outbox: CellControlAuditRecordV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RebalanceEvaluationOutcomeV1 {
    RequirementsRead(Box<RebalanceCandidateRequirementsV1>),
    Selected(Box<SignedPlacementDecisionV1>),
    NoCapacity(Box<SignedPlacementExhaustionV1>),
    Deferred(Box<SignedPlacementContinuationV1>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct RebalanceProgressWriteSetPartsV1 {
    pub authority: CellControlReconciliationPersistenceAuthorityV1,
    pub precondition: RebalanceSourcePreconditionV1,
    pub action_precondition: RebalanceActionPreconditionV1,
    pub outcome: RebalanceEvaluationOutcomeV1,
    pub completed_action: RebalanceSelectedActionV1,
    pub next_action: Option<RebalanceSelectedActionV1>,
    pub next_job: RebalanceJobV1,
    pub audit_outbox: CellControlAuditRecordV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RebalanceClosureWriteSetPartsV1 {
    pub authority: CellControlReconciliationPersistenceAuthorityV1,
    pub precondition: RebalanceSourcePreconditionV1,
    pub action_precondition: RebalanceActionPreconditionV1,
    pub closure: MovementActionClosureV1,
    pub next_action: RebalanceSelectedActionV1,
    pub next_job: RebalanceJobV1,
    pub audit_outbox: CellControlAuditRecordV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RebalanceLeafResultWriteSetPartsV1 {
    pub authority: CellControlReconciliationPersistenceAuthorityV1,
    pub precondition: RebalanceSourcePreconditionV1,
    pub action_precondition: RebalanceActionPreconditionV1,
    pub result: VerifiedMovementActionResult,
    pub next_action: RebalanceSelectedActionV1,
    pub next_job: RebalanceJobV1,
    pub audit_outbox: CellControlAuditRecordV1,
}

macro_rules! write_set {
    ($name:ident, $parts:ident) => {
        #[derive(Debug, Eq, PartialEq)]
        pub struct $name($parts);
        impl $name {
            pub fn assemble(_parts: $parts) -> Result<Self, PlacementContractError> {
                Err(PlacementContractError::NotImplemented)
            }
            #[must_use]
            pub fn parts(&self) -> &$parts {
                &self.0
            }
        }
    };
}

#[derive(Debug, Eq, PartialEq)]
pub struct RebalancePublicationWriteSetPartsV1 {
    pub authority: CellControlReconciliationPersistenceAuthorityV1,
    pub precondition: RebalanceSourcePreconditionV1,
    pub committed_issuance: VerifiedCommittedRebalanceIssuance,
    pub invocation: VerifiedPlacementInvocation,
    pub audit_outbox: CellControlAuditRecordV1,
}

write_set!(
    RebalancePublicationWriteSetV1,
    RebalancePublicationWriteSetPartsV1
);

write_set!(RebalanceClaimWriteSetV1, RebalanceClaimWriteSetPartsV1);
write_set!(
    RebalanceEvaluationWriteSetV1,
    RebalanceEvaluationWriteSetPartsV1
);
write_set!(
    RebalanceProgressWriteSetV1,
    RebalanceProgressWriteSetPartsV1
);
write_set!(RebalanceClosureWriteSetV1, RebalanceClosureWriteSetPartsV1);
write_set!(
    RebalanceLeafResultWriteSetV1,
    RebalanceLeafResultWriteSetPartsV1
);

pub trait RebalanceSourceStore: Send + Sync {
    fn publish<'a>(
        &'a self,
        write: &'a RebalancePublicationWriteSetV1,
    ) -> BoxCellFuture<'a, Result<SignedPlacementInvocationV1, PlacementContractError>>;
    /// Acquires or renews the job claim under the compare-and-set stated by
    /// [`RebalanceClaimWriteSetPartsV1`].
    ///
    /// Refused with
    /// [`PlacementContractError::JobClaimHeldByAnotherWorker`] when a live
    /// claim belongs to someone else; [`RebalanceSourceStore::read_claim`] is
    /// how the refused caller then finds out how long to wait.
    fn claim<'a>(
        &'a self,
        write: &'a RebalanceClaimWriteSetV1,
    ) -> BoxCellFuture<'a, Result<RebalanceJobClaimV1, PlacementContractError>>;
    /// Reads the claim currently held on one job WITHOUT acquiring it, so a
    /// worker refused by [`RebalanceSourceStore::claim`] can find out how long
    /// to wait, and so a job whose worker crashed is reachable at all.
    ///
    /// `None` means no live claim: attempt `claim` immediately. `Some` carries
    /// the `worker_id`, `epoch` and `expires_at_unix_seconds` that
    /// [`PlacementContractError::JobClaimHeldByAnotherWorker`] tells the caller
    /// to wait for. Before this existed, the only route to a job's claim was
    /// through [`RebalanceSourceIssuanceStore::load_issuance`] and the
    /// `claim` embedded in [`PlacementDispatchInputV1`] - keyed by issuance
    /// rather than by job, and a snapshot of whatever claim was live when that
    /// issuance was written, so a renewed or reassigned claim read stale and a
    /// job with no issuance yet was unreachable through the public API.
    ///
    /// READING A CLAIM IS NOT A STEP TOWARD HOLDING ONE. What comes back is the
    /// incumbent's record, not the reader's, and this port grants nothing: only
    /// `claim` moves the holder, and it does so under a compare-and-set that
    /// names the previous holder.
    ///
    /// THE EXPIRY IS A SNAPSHOT, NOT A PROMISE. A holder may renew, so a caller
    /// that wakes at the reported instant and is refused again reads again
    /// rather than concluding the claim must have lapsed.
    fn read_claim<'a>(
        &'a self,
        authority: &'a CellControlReconciliationReadAuthorityV1,
        address: &'a RebalanceJobAddressV1,
    ) -> BoxCellFuture<'a, Result<Option<RebalanceJobClaimV1>, PlacementContractError>>;
    fn persist_evaluation<'a>(
        &'a self,
        write: &'a RebalanceEvaluationWriteSetV1,
    ) -> BoxCellFuture<'a, Result<RebalanceEvaluationV1, PlacementContractError>>;
    fn get_evaluation<'a>(
        &'a self,
        authority: &'a CellControlReconciliationReadAuthorityV1,
        address: &'a RebalanceJobAddressV1,
        evaluation_id: &'a str,
    ) -> BoxCellFuture<'a, Result<Option<RebalanceEvaluationV1>, PlacementContractError>>;
    fn advance<'a>(
        &'a self,
        write: &'a RebalanceProgressWriteSetV1,
    ) -> BoxCellFuture<'a, Result<RebalanceJobV1, PlacementContractError>>;
    /// Durably closes the action and returns the closure record it now holds.
    /// It returns NO attestation and NO signature, which is clause (b) in the
    /// signature: a store must not witness its own write. The attestation comes
    /// from [`MovementActionClosureCommitObserver`], a separate port, and
    /// keeping the two roles in separate components is the deployment's job —
    /// see [`crate::MovementActionResultAuthority`].
    fn close_action<'a>(
        &'a self,
        write: &'a RebalanceClosureWriteSetV1,
    ) -> BoxCellFuture<'a, Result<MovementActionClosureV1, PlacementContractError>>;
    /// Re-reads one closure by explicit job address and action key. Absence
    /// distinguishes "never closed" from "closed, reply lost". Returns the
    /// durable record only.
    fn load_closure<'a>(
        &'a self,
        authority: &'a CellControlReconciliationReadAuthorityV1,
        address: &'a RebalanceJobAddressV1,
        key: &'a PlacementBusinessActionKeyV1,
    ) -> BoxCellFuture<'a, Result<Option<MovementActionClosureV1>, PlacementContractError>>;
    /// Durably folds a verified leaf result into the rebalance job and returns
    /// the advanced job record.
    ///
    /// THIS IS A MUTATION, and it was called `observe_result`. Every other
    /// `observe_*` in this wave is the law's independent observer: it takes a
    /// lookup key, re-reads, and signs what it found - including
    /// [`MovementActionClosureCommitObserver::observe_committed_closure`] in
    /// this same file. This one takes a write set and advances state, so the
    /// name put a writer in the namespace reserved for the ports that exist to
    /// check writers. It also defeated a sweep of this crate's own lookup
    /// surfaces, which matched it on the verb and had to discard it by hand.
    ///
    /// The leaf result it folds in has already been through the observer and
    /// the verifier; `RebalanceLeafResultWriteSetPartsV1` carries it as a
    /// private-field [`VerifiedMovementActionResult`].
    fn record_leaf_result<'a>(
        &'a self,
        write: &'a RebalanceLeafResultWriteSetV1,
    ) -> BoxCellFuture<'a, Result<RebalanceJobV1, PlacementContractError>>;
}

/// Independently re-reads a committed movement-action closure and signs what it
/// read.
///
/// The port accepts a lookup only - the same job address and action key
/// [`RebalanceSourceStore::load_closure`] takes - never a caller-supplied record
/// and never a caller's claim that a commit occurred. It deliberately does not
/// take a [`MovementActionClosureAddressV1`]: that address carries `closure_id`,
/// which is not known until the record has been read, so requiring it would
/// force the caller to supply part of the answer.
///
/// It returns the record it read together with its own attestation, for the same
/// reason as its issuance-side peer.
///
/// `None` means the observer looked and found NOTHING at that key. It is an
/// outcome, not a failure: the observer read under committed read isolation with
/// authority, and there was no committed row. That is precisely the fact
/// [`RebalanceSourceStore::load_closure`] exists to establish - it is what separates
/// "never durably committed" from "committed, reply lost" - so an observer that
/// could not say it would be unable to do its one job.
///
/// It is deliberately NOT `PlacementContractError::NotFoundOrNotAuthorized`.
/// That variant conflates absence with an authorization refusal, on purpose, so
/// that a lookup does not tell an unauthorized caller whether a record exists.
/// An observer already runs under authority, so for it the conflation destroys
/// exactly the distinction it is here to draw.
///
/// A `None` that DISAGREES with [`RebalanceSourceStore::load_closure`] reporting a record is a
/// REFUSAL, never a quiet fallback to "nothing was committed". The caller must
/// not proceed as if the write never happened on the strength of a store report
/// the observer could not corroborate.
pub trait MovementActionClosureCommitObserver: Send + Sync {
    fn observe_committed_closure<'a>(
        &'a self,
        authority: &'a CellControlReconciliationReadAuthorityV1,
        address: &'a RebalanceJobAddressV1,
        key: &'a PlacementBusinessActionKeyV1,
    ) -> BoxCellFuture<
        'a,
        Result<Option<CommittedMovementActionClosureClaimV1>, PlacementContractError>,
    >;
}
