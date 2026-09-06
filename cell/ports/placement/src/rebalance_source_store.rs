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
    fn close_action<'a>(
        &'a self,
        write: &'a RebalanceClosureWriteSetV1,
    ) -> BoxCellFuture<'a, Result<CommittedMovementActionClosureV1, PlacementContractError>>;
    fn load_closure<'a>(
        &'a self,
        authority: &'a CellControlReconciliationReadAuthorityV1,
        address: &'a RebalanceJobAddressV1,
        key: &'a PlacementBusinessActionKeyV1,
    ) -> BoxCellFuture<'a, Result<Option<CommittedMovementActionClosureV1>, PlacementContractError>>;
    fn observe_result<'a>(
        &'a self,
        write: &'a RebalanceObserveResultWriteSetV1,
    ) -> BoxCellFuture<'a, Result<RebalanceJobV1, PlacementContractError>>;
}
