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
    pub expected_job_revision: u64,
    pub expected_job_digest: Digest32,
    pub previous_claim: RebalanceClaimPreconditionV1,
    pub next_claim: RebalanceJobClaimV1,
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
pub struct RebalanceObserveResultWriteSetPartsV1 {
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
    RebalanceObserveResultWriteSetV1,
    RebalanceObserveResultWriteSetPartsV1
);

pub trait RebalanceSourceStore: Send + Sync {
    fn publish<'a>(
        &'a self,
        write: &'a RebalancePublicationWriteSetV1,
    ) -> BoxCellFuture<'a, Result<SignedPlacementInvocationV1, PlacementContractError>>;
    fn claim<'a>(
        &'a self,
        write: &'a RebalanceClaimWriteSetV1,
    ) -> BoxCellFuture<'a, Result<RebalanceJobClaimV1, PlacementContractError>>;
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
    /// It returns NO attestation and NO signature: a store cannot witness its
    /// own write. See [`MovementActionClosureCommitObserver`].
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
    fn observe_result<'a>(
        &'a self,
        write: &'a RebalanceObserveResultWriteSetV1,
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
pub trait MovementActionClosureCommitObserver: Send + Sync {
    fn observe_committed_closure<'a>(
        &'a self,
        authority: &'a CellControlReconciliationReadAuthorityV1,
        address: &'a RebalanceJobAddressV1,
        key: &'a PlacementBusinessActionKeyV1,
    ) -> BoxCellFuture<'a, Result<CommittedMovementActionClosureClaimV1, PlacementContractError>>;
}
