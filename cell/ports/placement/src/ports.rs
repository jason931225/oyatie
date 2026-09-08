use core::future::Future;
use core::pin::Pin;

use crate::{
    BindingOutcomeQueryRefV1, Digest32, PlacementIdempotencyKey, PlacementOperationKey,
    PlacementOperationRevision, PlacementOperationV1, SignedPlacementContinuationV1,
    SignedPlacementDecisionV1, SignedPlacementExhaustionV1, VerifiedBindingOutcome,
    VerifiedBindingParticipantManifestCommitment, VerifiedCellPlacementDecision,
    VerifiedPlacementIntent, VerifiedPlacementInvocation, VerifiedPlacementRepairAuthority,
    VerifiedReservationArmIntent, VerifiedReservationArmReceipt, VerifiedReservationCommitPermit,
    VerifiedSourceReservationReleasePermit,
};

pub type BoxCellFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementSelectionOutcomeV1 {
    Selected(Box<SignedPlacementDecisionV1>),
    Deferred(Box<SignedPlacementContinuationV1>),
    NoCapacity(Box<SignedPlacementExhaustionV1>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlacementContractError {
    NotImplemented,
    InvalidRequest,
    IdempotencyKeyReuse,
    NotFoundOrNotAuthorized,
    Backpressure,
    DeadlineExceeded,
    DependencyUnavailable,
    VerificationFailed,
    ProofAlreadyApplied,
    AuthorizationScopeMismatch,
    /// A compare-and-set precondition did not hold: the record moved under the
    /// caller between read and write.
    ///
    /// IT ALSO COVERS EVERY STORE-DERIVED VALUE A PROPOSAL RESTATES, and the
    /// scope of that is a principle rather than a list.
    ///
    /// Write sets under this taxonomy carry the successor rows they expect the
    /// store to end up holding — `next_job`, `next_claim`, `next_action`,
    /// `next_ledger`, `next_state`, `next_resource`, `next_capacity` and any
    /// successor a later write set adds. Those rows are ASSERTIONS, not
    /// authority to write. The store MUST DERIVE every value it owns —
    /// revisions, record digests, epochs, fences, roots, counts, states and
    /// update instants — and refuse a proposal that disagrees, rather than
    /// persisting what it was handed.
    ///
    /// THE SCOPE IS EVERY OWNER-OWNED OR STORE-DERIVED VALUE A PROPOSAL
    /// RESTATES, WHETHER A FIELD, A NESTED RECORD OR A WHOLE ROW, AND NOT THE
    /// EXAMPLES ABOVE. A refusal scoped by enumeration leaves everything not
    /// enumerated bare, which is the defect the sibling taxonomies were
    /// rescoped out of; the field names here are illustrations of the shape and
    /// carry no boundary.
    ///
    /// Why it matters where it is least obvious:
    /// [`crate::RebalanceJobClaimV1::epoch`] is a fence this store owns —
    /// `claim` advances it and returns the advanced record — and
    /// [`crate::MovementActionClosureCommitObservationV1::claim_epoch_at_commit`]
    /// is later SIGNED OVER it by an independent observer. A proposed epoch the
    /// store does not recompute is a value a signed observation will
    /// subsequently vouch for.
    ///
    /// The sibling taxonomies state the same rule, and this one lacked it while
    /// six write sets under it carried proposed successors:
    /// [`crate::CapabilityEffectErrorV1::Conflict`] covers a proposed successor
    /// revision, and `CapabilityEffectErrorV1::AuthorityContextMismatch`
    /// covers a restated owner-assigned value. Note the split those two draw
    /// and that it applies here as well: a value the OWNER assigns is not the
    /// same defect as a value the STORE derives, and only the second is
    /// retryable.
    ///
    /// RECOVERY: re-read, rebuild the successor from what came back, retry. The
    /// reads that recovery names exist —
    /// [`crate::RebalanceSourceStore::read_claim`],
    /// [`crate::RebalanceSourceStore::get_evaluation`],
    /// [`crate::RebalanceSourceStore::load_closure`] and
    /// [`crate::RebalanceSourceIssuanceStore::load_issuance`]. Distinct from
    /// `JobClaimHeldByAnotherWorker`, which is not a lost race but a surface
    /// legitimately occupied, and where retrying at once is refused again for
    /// the same reason.
    Conflict,
    /// A live rebalance job claim is held by a different worker, so
    /// [`crate::RebalanceSourceStore::claim`] refused rather than stealing it.
    ///
    /// Distinct from `Conflict`, which is a compare-and-set that lost a race
    /// and is retryable at once. This one says the surface is legitimately
    /// occupied and retrying immediately will be refused again for the same
    /// reason.
    ///
    /// DELIBERATELY CARRIES NO PAYLOAD. The caller needs the holder and the
    /// expiry, and both are obtained from
    /// [`crate::RebalanceSourceStore::read_claim`]. An expiry embedded here
    /// would be a snapshot that the holder's renewal can invalidate before the
    /// caller acts on it, and naming a value this enum cannot carry is the
    /// failure the read exists to prevent: a recovery must only prescribe what
    /// a caller can actually perform.
    JobClaimHeldByAnotherWorker,
    TerminalOperation,
    ForwardRecoveryRequired,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SelectAndReserveRequestV1 {
    operation: PlacementOperationKey,
    intent: VerifiedPlacementIntent,
    idempotency_key: PlacementIdempotencyKey,
    canonical_request_digest: Digest32,
}

impl SelectAndReserveRequestV1 {
    pub fn assemble(
        _operation: PlacementOperationKey,
        _intent: VerifiedPlacementIntent,
        _idempotency_key: PlacementIdempotencyKey,
        _canonical_request_digest: Digest32,
    ) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn operation(&self) -> &PlacementOperationKey {
        &self.operation
    }

    #[must_use]
    pub fn intent(&self) -> &VerifiedPlacementIntent {
        &self.intent
    }

    #[must_use]
    pub fn idempotency_key(&self) -> &PlacementIdempotencyKey {
        &self.idempotency_key
    }

    #[must_use]
    pub fn canonical_request_digest(&self) -> Digest32 {
        self.canonical_request_digest
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ArmReservationRequestV1 {
    pub operation: PlacementOperationKey,
    pub intent: VerifiedReservationArmIntent,
    pub idempotency_key: PlacementIdempotencyKey,
    pub canonical_request_digest: Digest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ApplyBindingOutcomeRequestV1 {
    pub operation: PlacementOperationKey,
    pub outcome: VerifiedBindingOutcome,
    pub idempotency_key: PlacementIdempotencyKey,
    pub canonical_request_digest: Digest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ApplySourceReservationReleaseRequestV1 {
    pub operation: PlacementOperationKey,
    pub permit: VerifiedSourceReservationReleasePermit,
    pub idempotency_key: PlacementIdempotencyKey,
    pub canonical_request_digest: Digest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FinalizeReservationCommitPermitRequestV1 {
    pub operation: PlacementOperationKey,
    pub home_arm_receipt: VerifiedReservationArmReceipt,
    pub warm_recovery_arm_receipt: Option<VerifiedReservationArmReceipt>,
    pub idempotency_key: PlacementIdempotencyKey,
    pub canonical_request_digest: Digest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ScheduleMovementRequestV1 {
    pub operation: PlacementOperationKey,
    pub placement_decision: VerifiedCellPlacementDecision,
    pub reservation_commit_permit: VerifiedReservationCommitPermit,
    pub binding_operation: BindingOutcomeQueryRefV1,
    pub participant_manifest: VerifiedBindingParticipantManifestCommitment,
    pub budget_request: crate::MovementBudgetRequestV1,
    pub requested_deadline_unix_seconds: u64,
    pub idempotency_key: PlacementIdempotencyKey,
    pub canonical_request_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationMutationRequestV1 {
    pub operation: PlacementOperationKey,
    pub expected_revision: PlacementOperationRevision,
    pub idempotency_key: PlacementIdempotencyKey,
    pub canonical_request_digest: Digest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RepairPlacementOperationRequestV1 {
    pub repair_operation: PlacementOperationKey,
    pub target_operation: PlacementOperationKey,
    pub expected_target_revision: PlacementOperationRevision,
    pub authority: VerifiedPlacementRepairAuthority,
    pub idempotency_key: PlacementIdempotencyKey,
    pub canonical_request_digest: Digest32,
}

pub trait CellPlacementService: Send + Sync {
    fn select_and_reserve<'a>(
        &'a self,
        invocation: VerifiedPlacementInvocation,
        request: SelectAndReserveRequestV1,
    ) -> BoxCellFuture<'a, Result<PlacementOperationV1, PlacementContractError>>;

    fn arm_reservation<'a>(
        &'a self,
        invocation: VerifiedPlacementInvocation,
        request: ArmReservationRequestV1,
    ) -> BoxCellFuture<'a, Result<PlacementOperationV1, PlacementContractError>>;

    fn apply_binding_outcome<'a>(
        &'a self,
        invocation: VerifiedPlacementInvocation,
        request: ApplyBindingOutcomeRequestV1,
    ) -> BoxCellFuture<'a, Result<PlacementOperationV1, PlacementContractError>>;

    fn apply_source_reservation_release<'a>(
        &'a self,
        invocation: VerifiedPlacementInvocation,
        request: ApplySourceReservationReleaseRequestV1,
    ) -> BoxCellFuture<'a, Result<PlacementOperationV1, PlacementContractError>>;

    fn finalize_reservation_commit_permit<'a>(
        &'a self,
        invocation: VerifiedPlacementInvocation,
        request: FinalizeReservationCommitPermitRequestV1,
    ) -> BoxCellFuture<'a, Result<PlacementOperationV1, PlacementContractError>>;

    fn schedule_movement<'a>(
        &'a self,
        invocation: VerifiedPlacementInvocation,
        request: ScheduleMovementRequestV1,
    ) -> BoxCellFuture<'a, Result<PlacementOperationV1, PlacementContractError>>;

    fn get_operation<'a>(
        &'a self,
        invocation: VerifiedPlacementInvocation,
        operation: &'a PlacementOperationKey,
    ) -> BoxCellFuture<'a, Result<PlacementOperationV1, PlacementContractError>>;

    fn cancel_operation<'a>(
        &'a self,
        invocation: VerifiedPlacementInvocation,
        request: OperationMutationRequestV1,
    ) -> BoxCellFuture<'a, Result<PlacementOperationV1, PlacementContractError>>;

    fn repair_operation<'a>(
        &'a self,
        invocation: VerifiedPlacementInvocation,
        request: RepairPlacementOperationRequestV1,
    ) -> BoxCellFuture<'a, Result<PlacementOperationV1, PlacementContractError>>;
}
