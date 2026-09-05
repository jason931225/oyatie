use cell_placement::{
    BindingAbortCodeV1, BindingOutcomeQueryRefV1, CellId, SignedBindingOutcomeV1,
    SignedSourceReservationReleasePermitV1, VerifiedCellMovementPermit,
    VerifiedCellPlacementDecision, VerifiedReservationCommitPermit,
};

use crate::{
    BindingAttemptCheckpointEvidenceV1, BindingAttemptMutationResultV1, BindingContractError,
    BindingDigest32, BindingGeneration, BindingHistoryEntry, BindingIdempotencyKey,
    BindingOperationKey, BindingOperationRevision, BindingOperationV1,
    BindingReservationAttemptPreconditionV1, BindingReservationAttemptV1, BindingRevision,
    FinalizeMigrationReleaseRequestV1, InitialBindingRequestV1,
    MigrationFenceClaimMutationResultV1, MigrationFenceClaimV1, MigrationRetargetMutationResultV1,
    MigrationWriteFenceMutationResultV1, MoveBindingRequestV1, RetargetMigrationRequestV1,
    SignedWriteFenceV1, TenantCellBinding, TenantId, VerifiedBindingInvocation,
    VerifiedBindingRepairAuthority, VerifiedParticipantManifest, VerifiedParticipantPhaseClosure,
    VerifiedResidencyTransferAuthorizationSet, VerifiedRetiredSourceEffectClosureV1,
    VerifiedTransferEffectManifest, VerifiedWriteFence,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingHistoryPageTokenV1(Vec<u8>);

impl BindingHistoryPageTokenV1 {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn parse(_value: Vec<u8>) -> Result<Self, BindingContractError> {
        Err(BindingContractError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingHistoryPageRequestV1 {
    pub page_size: u32,
    pub page_token: Option<BindingHistoryPageTokenV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingHistoryPageV1 {
    pub bindings: Vec<BindingHistoryEntry>,
    pub next_page_token: Option<BindingHistoryPageTokenV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingOperationMutationRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_revision: BindingOperationRevision,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AbortBindingOutcomeRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_operation_revision: BindingOperationRevision,
    pub abort_code: BindingAbortCodeV1,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct OpenBindingAttemptRequestV1 {
    pub operation: BindingOperationKey,
    pub placement_decision: VerifiedCellPlacementDecision,
    pub participant_manifest: VerifiedParticipantManifest,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CheckpointBindingAttemptRequestV1 {
    pub operation: BindingOperationKey,
    pub precondition: BindingReservationAttemptPreconditionV1,
    pub expected_operation_revision: BindingOperationRevision,
    pub evidence: BindingAttemptCheckpointEvidenceV1,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ClaimMigrationFenceRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_operation_revision: BindingOperationRevision,
    pub expected_source_cell_id: CellId,
    pub expected_source_generation: BindingGeneration,
    pub expected_source_revision: BindingRevision,
    pub source_binding_record_digest: BindingDigest32,
    pub binding_attempt_digest: BindingDigest32,
    pub expected_source_authority: crate::ServingAuthorityInstanceV1,
    pub participant_manifest: VerifiedParticipantManifest,
    pub placement_decision: VerifiedCellPlacementDecision,
    pub reservation_commit_permit: VerifiedReservationCommitPermit,
    pub transfer_effect_manifest: VerifiedTransferEffectManifest,
    pub transfer_authorization_set: VerifiedResidencyTransferAuthorizationSet,
    pub prepared_participant_closure: VerifiedParticipantPhaseClosure,
    pub movement_permit: VerifiedCellMovementPermit,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CommitMigrationWriteFenceRequestV1 {
    pub operation: BindingOperationKey,
    pub expected_operation_revision: BindingOperationRevision,
    pub write_fence: VerifiedWriteFence,
    pub source_fencing_completion: VerifiedRetiredSourceEffectClosureV1,
    pub source_authority_retirement: crate::VerifiedServingAuthorityRetirementV1,
    pub terminal_authority_closure: crate::VerifiedServingAuthorityTerminalClosure,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RepairBindingOperationRequestV1 {
    pub repair_operation: BindingOperationKey,
    pub target_operation: BindingOperationKey,
    pub expected_target_revision: BindingOperationRevision,
    pub authority: VerifiedBindingRepairAuthority,
    pub idempotency_key: BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

pub trait TenantBindingService: Send + Sync {
    fn get_binding<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        tenant_id: &'a TenantId,
    ) -> crate::BoxTenancyFuture<'a, Result<TenantCellBinding, BindingContractError>>;

    fn commit_initial_binding<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: InitialBindingRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<BindingOperationV1, BindingContractError>>;

    fn move_binding<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: MoveBindingRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<BindingOperationV1, BindingContractError>>;

    fn open_binding_attempt<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: OpenBindingAttemptRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<BindingAttemptMutationResultV1, BindingContractError>>;

    fn abort_binding_outcome<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: AbortBindingOutcomeRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<SignedBindingOutcomeV1, BindingContractError>>;

    fn checkpoint_binding_attempt<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: CheckpointBindingAttemptRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<BindingAttemptMutationResultV1, BindingContractError>>;

    fn claim_migration_fence<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: ClaimMigrationFenceRequestV1,
    ) -> crate::BoxTenancyFuture<
        'a,
        Result<MigrationFenceClaimMutationResultV1, BindingContractError>,
    >;

    fn commit_migration_write_fence<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: CommitMigrationWriteFenceRequestV1,
    ) -> crate::BoxTenancyFuture<
        'a,
        Result<MigrationWriteFenceMutationResultV1, BindingContractError>,
    >;

    fn retarget_migration<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: RetargetMigrationRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<MigrationRetargetMutationResultV1, BindingContractError>>;

    fn finalize_migration_release<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: FinalizeMigrationReleaseRequestV1,
    ) -> crate::BoxTenancyFuture<
        'a,
        Result<SignedSourceReservationReleasePermitV1, BindingContractError>,
    >;

    fn list_binding_history<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        tenant_id: &'a TenantId,
        page: &'a BindingHistoryPageRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<BindingHistoryPageV1, BindingContractError>>;

    fn get_operation<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        operation: &'a BindingOperationKey,
    ) -> crate::BoxTenancyFuture<'a, Result<BindingOperationV1, BindingContractError>>;

    fn cancel_operation<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: BindingOperationMutationRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<BindingOperationV1, BindingContractError>>;

    fn repair_operation<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        request: RepairBindingOperationRequestV1,
    ) -> crate::BoxTenancyFuture<'a, Result<BindingOperationV1, BindingContractError>>;

    fn get_binding_outcome<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        query: &'a BindingOutcomeQueryRefV1,
    ) -> crate::BoxTenancyFuture<'a, Result<SignedBindingOutcomeV1, BindingContractError>>;

    fn get_migration_fence_claim<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        operation: &'a BindingOperationKey,
    ) -> crate::BoxTenancyFuture<'a, Result<MigrationFenceClaimV1, BindingContractError>>;

    fn get_migration_write_fence<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        operation: &'a BindingOperationKey,
    ) -> crate::BoxTenancyFuture<'a, Result<SignedWriteFenceV1, BindingContractError>>;

    fn get_binding_attempt<'a>(
        &'a self,
        invocation: VerifiedBindingInvocation,
        operation: &'a BindingOperationKey,
    ) -> crate::BoxTenancyFuture<'a, Result<BindingReservationAttemptV1, BindingContractError>>;
}
