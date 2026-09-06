use crate::{
    BindingReadAuthorityV1, BindingReconciliationReadAuthorityV1, BindingStoreError,
    BoxTenancyFuture, CapabilityTransferEffectWriteSetV1, IssueTransferExecutionPermitWriteSetV1,
    PublishTransferExecutionPermitWriteSetV1, RecordTransferExecutionOutcomeWriteSetV1,
    SignedTransferExecutionCommitObservationV1, SignedTransferExecutionPermitV1,
    TransferExecutionLedgerV1, TransferExecutionPermitIssuanceLookupV1,
    TransferExecutionPermitIssuanceRecordV1, TransferExecutionRepairMutationResultV1,
    TransferExecutionRepairWriteSetV1, VerifiedCommittedTransferExecutionPermitIssuance,
    VerifiedTransferExecutionPermit,
};

pub trait TransferExecutionStore: Send + Sync {
    /// Durably records an unsigned transfer-execution permit issuance.
    ///
    /// The returned record never carries a signature: a signed permit is only
    /// reachable through [`TransferExecutionCommitObserver`],
    /// [`crate::verify_committed_transfer_execution_permit_issuance`],
    /// [`TransferExecutionPermitAuthority::sign_committed`] and
    /// [`TransferExecutionStore::publish_permit`].
    fn issue_permit<'a>(
        &'a self,
        write_set: &'a IssueTransferExecutionPermitWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<TransferExecutionPermitIssuanceRecordV1, BindingStoreError>>;

    /// Re-reads a permit issuance by explicit key, for republication after a
    /// lost reply. Absence distinguishes "never durably issued" from
    /// "issued, reply lost".
    fn load_permit_issuance<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        lookup: &'a TransferExecutionPermitIssuanceLookupV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<TransferExecutionPermitIssuanceRecordV1>, BindingStoreError>,
    >;

    fn load_permit_issuance_for_reconciliation<'a>(
        &'a self,
        authority: &'a BindingReconciliationReadAuthorityV1,
        lookup: &'a TransferExecutionPermitIssuanceLookupV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<TransferExecutionPermitIssuanceRecordV1>, BindingStoreError>,
    >;

    /// Publishes the signed permit for an already-committed issuance. The write
    /// set can only be assembled from a verified commit observation, so this is
    /// the sole path on which a signature is minted. Republishing the same
    /// immutable issuance is permitted; byte-identical signatures are not
    /// required.
    fn publish_permit<'a>(
        &'a self,
        write_set: &'a PublishTransferExecutionPermitWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<SignedTransferExecutionPermitV1, BindingStoreError>>;

    fn record_outcome<'a>(
        &'a self,
        write_set: &'a RecordTransferExecutionOutcomeWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<TransferExecutionLedgerV1, BindingStoreError>>;

    fn get_ledger<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        operation: &'a crate::BindingOperationKey,
    ) -> BoxTenancyFuture<'a, Result<Option<TransferExecutionLedgerV1>, BindingStoreError>>;

    fn get_ledger_for_reconciliation<'a>(
        &'a self,
        authority: &'a BindingReconciliationReadAuthorityV1,
        operation: &'a crate::BindingOperationKey,
    ) -> BoxTenancyFuture<'a, Result<Option<TransferExecutionLedgerV1>, BindingStoreError>>;

    fn apply_repair<'a>(
        &'a self,
        write_set: &'a TransferExecutionRepairWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<TransferExecutionRepairMutationResultV1, BindingStoreError>>;

    fn read_item_page_for_reconciliation<'a>(
        &'a self,
        authority: &'a BindingReconciliationReadAuthorityV1,
        reconciliation_lease: &'a crate::BindingReconciliationLeaseV1,
        request: &'a crate::TransferExecutionItemPageRequestV1,
    ) -> BoxTenancyFuture<'a, Result<crate::TransferExecutionItemPageV1, BindingStoreError>>;
}

/// Independently re-reads a committed permit issuance and signs what it read.
///
/// The port accepts a lookup key only. It is not given, and cannot be given, a
/// caller-supplied record or a caller's claim that a commit occurred: every
/// field of the emitted observation describes what the observer itself read
/// under [`crate::TransferExecutionIssuanceReadIsolationV1`].
pub trait TransferExecutionCommitObserver: Send + Sync {
    fn observe_committed_permit_issuance<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        lookup: &'a TransferExecutionPermitIssuanceLookupV1,
    ) -> BoxTenancyFuture<'a, Result<SignedTransferExecutionCommitObservationV1, BindingStoreError>>;
}

/// Mints the transfer-execution permit signature.
///
/// The only argument is a private-field verified wrapper, so no signature can
/// be produced from an unverified or precommit claim.
pub trait TransferExecutionPermitAuthority: Send + Sync {
    fn sign_committed<'a>(
        &'a self,
        issuance: &'a VerifiedCommittedTransferExecutionPermitIssuance,
    ) -> BoxTenancyFuture<'a, Result<VerifiedTransferExecutionPermit, BindingStoreError>>;
}

pub trait CapabilityTransferEffectStore: Send + Sync {
    fn consume_before_effect<'a>(
        &'a self,
        write_set: &'a CapabilityTransferEffectWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<crate::BindingDigest32, BindingStoreError>>;
}
