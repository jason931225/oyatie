use cell_placement::{VerifiedCellMovementPermit, VerifiedCellPlacementDecision};

use crate::{
    BindingContractError, BindingDigest32, BindingIdempotencyKey, BindingOperationKey,
    BindingOperationRevision, BindingReservationAttemptRevision, PutParticipantManifestResultV1,
    SignedParticipantPhaseClosureV1, SignedResidencyTransferAuthorizationSetV1,
    SignedTransferExecutionOutcomeV1, SignedTransferExecutionPermitV1,
    SourceFenceDirectiveIssueResultV1, SourceFenceDirectiveLedgerRevision,
    TransferAuthorizationJournalRevision, TransferExecutionLedgerRevision,
    TransferExecutionLedgerV1, TransferExecutionPermitIssuanceAddressV1,
    TransferExecutionPermitIssuanceRecordV1, TransferExecutionPermitIssuanceRevision,
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

/// Result of [`TenancyMigrationCoordinationService::issue_transfer_execution_permit`].
///
/// Carries the durable UNSIGNED issuance only. There is deliberately no
/// signature-typed field on this type: a signed permit is reachable only through
/// [`TenancyMigrationCoordinationService::publish_transfer_execution_permit`],
/// and only after an independent commit observation of the issuance row has been
/// produced and verified.
#[derive(Debug, Eq, PartialEq)]
pub struct TransferExecutionPermitIssueResultV1 {
    pub issuance: TransferExecutionPermitIssuanceRecordV1,
    pub ledger: TransferExecutionLedgerV1,
    pub operation: crate::BindingOperationV1,
}

/// Request for
/// [`TenancyMigrationCoordinationService::publish_transfer_execution_permit`].
///
/// Every field is either a lookup key or a precondition that the service checks
/// against its own read. This type deliberately CANNOT carry an issuance record,
/// a commit observation, or a signature: the commit observer, the verifier and
/// the signer are internal collaborators of the service and are not nameable by
/// a caller. Do not add a `committed_issuance` field here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishTransferExecutionPermitRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_operation_revision: BindingOperationRevision,
    pub expected_ledger_revision: TransferExecutionLedgerRevision,
    pub issuance_address: TransferExecutionPermitIssuanceAddressV1,
    pub expected_issuance_revision: TransferExecutionPermitIssuanceRevision,
    pub expected_issuance_record_digest: BindingDigest32,
    pub required_read_isolation: crate::TransferExecutionIssuanceReadIsolationV1,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

/// Request for
/// [`TenancyMigrationCoordinationService::get_transfer_execution_permit`].
/// Read-only, so it carries no idempotency key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetTransferExecutionPermitRequestV1 {
    pub operation: BindingOperationKey,
    pub issuance_address: TransferExecutionPermitIssuanceAddressV1,
    pub canonical_request_digest: BindingDigest32,
}

/// Result of
/// [`TenancyMigrationCoordinationService::get_transfer_execution_permit`].
///
/// The pair of `Option`s is what makes a lost reply distinguishable from a
/// permit that was never durably issued:
///
/// - `issuance: None` — nothing was ever durably issued.
/// - `issuance: Some(_), permit: None` — the issuance committed but no
///   publication has been observed; the reply was lost and
///   [`TenancyMigrationCoordinationService::publish_transfer_execution_permit`]
///   may be called again for the same immutable issuance.
/// - `issuance: Some(_), permit: Some(_)` — published.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferExecutionPermitGetResultV1 {
    pub issuance: Option<TransferExecutionPermitIssuanceRecordV1>,
    pub permit: Option<SignedTransferExecutionPermitV1>,
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

    /// Durably issues an UNSIGNED transfer-execution permit.
    ///
    /// This call can never return a signature. Obtaining one requires
    /// [`Self::publish_transfer_execution_permit`], which runs behind an
    /// independent commit observation and its verification.
    fn issue_transfer_execution_permit<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: IssueTransferExecutionPermitRequestV1,
    ) -> crate::BoxTenancyFuture<
        'a,
        Result<TransferExecutionPermitIssueResultV1, BindingContractError>,
    >;

    /// Mints and returns the signed permit for an already-committed issuance.
    ///
    /// The request names the issuance by key only. The service is responsible
    /// for re-reading the committed row through
    /// [`crate::TransferExecutionCommitObserver`], verifying the observation
    /// through
    /// [`crate::verify_committed_transfer_execution_permit_issuance`], signing
    /// through [`crate::TransferExecutionPermitAuthority`] and publishing
    /// through [`crate::TransferExecutionStore::publish_permit`]. Republishing
    /// the same immutable issuance is permitted; byte-identical signatures are
    /// not required.
    fn publish_transfer_execution_permit<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: PublishTransferExecutionPermitRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<SignedTransferExecutionPermitV1, BindingContractError>>;

    /// Reads issuance and publication state without mutating anything, so a
    /// caller that lost a reply can tell "never issued" from "issued, not yet
    /// published".
    fn get_transfer_execution_permit<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: GetTransferExecutionPermitRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<TransferExecutionPermitGetResultV1, BindingContractError>>;

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
