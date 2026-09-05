use core::future::Future;
use core::pin::Pin;

use cell_placement::{BindingOutcomeQueryRefV1, SignedBindingOutcomeV1};

use crate::{
    BindingAbortWriteSetV1, BindingAttemptMutationResultV1, BindingHistoryEntry,
    BindingHistoryPageRequestV1, BindingHistoryPageV1, BindingIdempotencyKey,
    BindingIdempotencyRecordV1, BindingMigrationFenceClaimWriteSetV1,
    BindingMigrationWriteFenceWriteSetV1, BindingOperationKey, BindingOperationV1,
    BindingOperationWriteSetV1, BindingReadAuthorityV1, BindingRepairWriteSetV1,
    BindingReservationAttemptOpenWriteSetV1, BindingReservationAttemptV1,
    BindingReservationAttemptWriteSetV1, BindingWriteSetV1, MigrationFenceClaimMutationResultV1,
    MigrationFenceClaimV1, MigrationWriteFenceMutationResultV1, SignedWriteFenceV1,
    TenantCellBinding, TenantId, TenantWriteAuthorityHighWaterV1,
};

pub type BoxTenancyFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingStoreError {
    NotImplemented,
    Unavailable,
    Conflict,
    IdempotencyKeyReuse,
    ProofAlreadyApplied,
    OutcomeAlreadyFinal,
    ActiveMigrationClaim,
    StaleMigrationClaim,
    WriteFenceAlreadyCommitted,
    StaleReservationAttempt,
    TerminalReservationAttempt,
    TerminalOperation,
    InvalidEffectBoundaryTransition,
    StaleAuthorityHighWater,
    StaleWriteAuthorityLeaseState,
    StaleWriteAuthorityLeaseIssuance,
    WriteAuthorityLeaseNotCommitted,
    WriteAuthorityLeaseFrozen,
    WriteAuthorityLeaseExpired,
    TokenValidityExceedsLease,
    StaleCapabilityWriteAuthority,
    CapabilityWriteAuthorityFenced,
    CapabilityAuthorityRollback,
    WriteAttemptOutsideAuthorityWindow,
    AuthorizationScopeMismatch,
    SnapshotNotSealed,
    SnapshotAlreadySealed,
    SnapshotIdentityMismatch,
    SnapshotPageOutOfOrder,
    SnapshotPageLimitExceeded,
    SnapshotMembershipMismatch,
    SnapshotPolicyMismatch,
    CorruptRecord,
}

pub trait TenantBindingStore: Send + Sync {
    fn get_binding<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        tenant_id: &'a TenantId,
    ) -> BoxTenancyFuture<'a, Result<Option<TenantCellBinding>, BindingStoreError>>;

    fn commit_initial_tenant_and_binding<'a>(
        &'a self,
        write_set: &'a BindingWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<crate::BindingCommitTransactionResultV1, BindingStoreError>>;

    fn open_reservation_attempt<'a>(
        &'a self,
        write_set: &'a BindingReservationAttemptOpenWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<BindingAttemptMutationResultV1, BindingStoreError>>;

    fn compare_and_swap_move<'a>(
        &'a self,
        write_set: &'a BindingWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<crate::BindingCommitTransactionResultV1, BindingStoreError>>;

    fn commit_aborted_outcome<'a>(
        &'a self,
        write_set: &'a BindingAbortWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<crate::BindingAbortTransactionResultV1, BindingStoreError>>;

    fn compare_and_set_migration_fence_claim<'a>(
        &'a self,
        write_set: &'a BindingMigrationFenceClaimWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<MigrationFenceClaimMutationResultV1, BindingStoreError>>;

    fn commit_migration_write_fence<'a>(
        &'a self,
        write_set: &'a BindingMigrationWriteFenceWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<MigrationWriteFenceMutationResultV1, BindingStoreError>>;

    fn apply_reservation_attempt<'a>(
        &'a self,
        write_set: &'a BindingReservationAttemptWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<BindingAttemptMutationResultV1, BindingStoreError>>;

    fn apply_operation<'a>(
        &'a self,
        write_set: &'a BindingOperationWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<BindingOperationV1, BindingStoreError>>;

    fn apply_repair<'a>(
        &'a self,
        write_set: &'a BindingRepairWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<crate::BindingRepairMutationResultV1, BindingStoreError>>;

    fn get_operation<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        operation: &'a BindingOperationKey,
    ) -> BoxTenancyFuture<'a, Result<Option<BindingOperationV1>, BindingStoreError>>;

    fn get_idempotent<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        tenant_id: &'a TenantId,
        key: &'a BindingIdempotencyKey,
    ) -> BoxTenancyFuture<'a, Result<Option<BindingIdempotencyRecordV1>, BindingStoreError>>;

    fn get_binding_outcome<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        query: &'a BindingOutcomeQueryRefV1,
    ) -> BoxTenancyFuture<'a, Result<Option<SignedBindingOutcomeV1>, BindingStoreError>>;

    fn get_migration_fence_claim<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        operation: &'a BindingOperationKey,
    ) -> BoxTenancyFuture<'a, Result<Option<MigrationFenceClaimV1>, BindingStoreError>>;

    fn get_migration_write_fence<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        operation: &'a BindingOperationKey,
    ) -> BoxTenancyFuture<'a, Result<Option<SignedWriteFenceV1>, BindingStoreError>>;

    fn get_reservation_attempt<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        operation: &'a BindingOperationKey,
    ) -> BoxTenancyFuture<'a, Result<Option<BindingReservationAttemptV1>, BindingStoreError>>;

    fn get_authority_high_water<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        tenant_id: &'a TenantId,
    ) -> BoxTenancyFuture<'a, Result<Option<TenantWriteAuthorityHighWaterV1>, BindingStoreError>>;

    fn list_history<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        tenant_id: &'a TenantId,
        page: &'a BindingHistoryPageRequestV1,
    ) -> BoxTenancyFuture<'a, Result<BindingHistoryPageV1, BindingStoreError>>;
}
