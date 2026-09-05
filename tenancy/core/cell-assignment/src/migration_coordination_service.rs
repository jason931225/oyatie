use cell_placement::{VerifiedCellMovementPermit, VerifiedCellPlacementDecision};

use crate::{
    BindingContractError, BindingDigest32, BindingIdempotencyKey, BindingOperationKey,
    BindingOperationRevision, BindingReservationAttemptRevision, PutParticipantManifestResultV1,
    SignedParticipantPhaseClosureV1, SignedResidencyTransferAuthorizationSetV1,
    SignedTransferExecutionOutcomeV1, SignedTransferExecutionPermitV1,
    SourceFenceDirectiveIssueResultV1, SourceFenceDirectiveLedgerRevision,
    TransferAuthorizationJournalRevision, TransferExecutionLedgerRevision,
    VerifiedBindingInvocation, VerifiedParticipantManifest, VerifiedParticipantPhaseClosure,
    VerifiedParticipantReceipt, VerifiedResidencyTransferAuthorization,
    VerifiedResidencyTransferAuthorizationSet, VerifiedTransferEffectManifest,
    VerifiedTransferExecutionOutcome,
};

#[derive(Debug, Eq, PartialEq)]
pub struct PutParticipantManifestRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_attempt_revision: BindingReservationAttemptRevision,
    pub expected_operation_revision: BindingOperationRevision,
    pub placement_decision: VerifiedCellPlacementDecision,
    pub manifest: VerifiedParticipantManifest,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AppendParticipantReceiptRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_operation_revision: BindingOperationRevision,
    pub expected_ledger_revision: crate::ParticipantReceiptLedgerRevision,
    pub receipt: VerifiedParticipantReceipt,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CloseParticipantPhaseRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_operation_revision: BindingOperationRevision,
    pub expected_ledger_revision: crate::ParticipantReceiptLedgerRevision,
    pub closure: VerifiedParticipantPhaseClosure,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AppendTransferAuthorizationRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_operation_revision: BindingOperationRevision,
    pub expected_journal_revision: TransferAuthorizationJournalRevision,
    pub manifest: VerifiedTransferEffectManifest,
    pub movement_permit: VerifiedCellMovementPermit,
    pub authorization: VerifiedResidencyTransferAuthorization,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SealTransferAuthorizationSetRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_operation_revision: BindingOperationRevision,
    pub expected_journal_revision: TransferAuthorizationJournalRevision,
    pub manifest: VerifiedTransferEffectManifest,
    pub set: VerifiedResidencyTransferAuthorizationSet,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct IssueTransferExecutionPermitRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_operation_revision: BindingOperationRevision,
    pub expected_ledger_revision: TransferExecutionLedgerRevision,
    pub authorization: VerifiedResidencyTransferAuthorization,
    pub authorization_set: VerifiedResidencyTransferAuthorizationSet,
    pub participant: crate::VerifiedParticipantManifestMember,
    pub movement_permit: VerifiedCellMovementPermit,
    pub requested_budget_pool: crate::MovementBudgetPoolV1,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RecordTransferExecutionOutcomeRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_operation_revision: BindingOperationRevision,
    pub expected_ledger_revision: TransferExecutionLedgerRevision,
    pub outcome: VerifiedTransferExecutionOutcome,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct IssueSourceFenceDirectiveRequestV1 {
    pub source_authority_freeze: crate::VerifiedServingAuthorityFreezeResult,
    pub operation: BindingOperationKey,
    pub expected_operation_revision: BindingOperationRevision,
    pub migration_fence_claim_digest: BindingDigest32,
    pub expected_ledger_revision: SourceFenceDirectiveLedgerRevision,
    pub participant: crate::VerifiedParticipantManifestMember,
    pub requested_validity_seconds: u64,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

pub trait TenancyMigrationCoordinationService: Send + Sync {
    fn put_participant_manifest<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: PutParticipantManifestRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<PutParticipantManifestResultV1, BindingContractError>>;

    fn append_participant_receipt<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: AppendParticipantReceiptRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<crate::ParticipantReceiptLedgerV1, BindingContractError>>;

    fn close_participant_phase<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: CloseParticipantPhaseRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<SignedParticipantPhaseClosureV1, BindingContractError>>;

    fn append_transfer_authorization<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: AppendTransferAuthorizationRequestV1,
    ) -> crate::BoxTenancyFuture<
        'a,
        Result<crate::TransferAuthorizationJournalV1, BindingContractError>,
    >;

    fn seal_transfer_authorization_set<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: SealTransferAuthorizationSetRequestV1,
    ) -> crate::BoxTenancyFuture<
        'a,
        Result<SignedResidencyTransferAuthorizationSetV1, BindingContractError>,
    >;

    fn issue_transfer_execution_permit<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: IssueTransferExecutionPermitRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<SignedTransferExecutionPermitV1, BindingContractError>>;

    fn record_transfer_execution_outcome<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: RecordTransferExecutionOutcomeRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<SignedTransferExecutionOutcomeV1, BindingContractError>>;

    fn issue_source_fence_directive<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: IssueSourceFenceDirectiveRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<SourceFenceDirectiveIssueResultV1, BindingContractError>>;
}
