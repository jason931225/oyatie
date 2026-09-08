use cell_placement::{
    CellProofConsumptionV1, SignedBindingParticipantManifestCommitmentV1,
    VerifiedCellPlacementDecision,
};

use crate::{
    BindingAuditRecordV1, BindingIdempotencyRecordV1, BindingOperationKey,
    BindingOperationRevision, BindingOperationV1, BindingPersistenceAuthorityV1,
    BindingProofConsumptionV1, BindingReadAuthorityV1, BindingReconciliationLeaseV1,
    BindingReconciliationReadAuthorityV1, BindingReservationAttemptRevision, BindingStoreError,
    BoxTenancyFuture, ParticipantManifestMemberPageRequestV1, ParticipantManifestMemberPageV1,
    ParticipantManifestMemberSetV1, ParticipantReceiptLedgerRevision, ParticipantReceiptLedgerV1,
    ParticipantReceiptPhaseV1, ParticipantReceiptWorkItemV1, ParticipantReceiptWorkPageRequestV1,
    ParticipantReceiptWorkPageV1, ParticipantReceiptWorkPreconditionV1,
    SignedParticipantManifestV1, SignedParticipantPhaseClosureV1, TenantId,
    VerifiedParticipantManifest, VerifiedParticipantPhaseClosure, VerifiedParticipantReceipt,
};

#[derive(Debug, Eq, PartialEq)]
pub struct PutParticipantManifestWriteSetV1 {
    parts: PutParticipantManifestWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PutParticipantManifestWriteSetPartsV1 {
    pub authority: crate::BindingWorkSnapshotMutationAuthorityV1,
    pub expected_attempt_revision: BindingReservationAttemptRevision,
    pub expected_operation_revision: BindingOperationRevision,
    pub operation: BindingOperationV1,
    pub placement_decision: VerifiedCellPlacementDecision,
    pub manifest: VerifiedParticipantManifest,
    pub members: ParticipantManifestMemberSetV1,
    pub published_snapshot: crate::BindingWorkSnapshotProgressV1,
    pub cell_commitment: SignedBindingParticipantManifestCommitmentV1,
    pub idempotency: BindingIdempotencyRecordV1,
    pub proof_consumptions: Vec<BindingProofConsumptionV1>,
    pub cell_proof_consumptions: Vec<CellProofConsumptionV1>,
    pub audit_outbox: BindingAuditRecordV1,
}

impl PutParticipantManifestWriteSetV1 {
    pub fn assemble(
        _parts: PutParticipantManifestWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &PutParticipantManifestWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PutParticipantManifestResultV1 {
    pub manifest: SignedParticipantManifestV1,
    pub cell_commitment: SignedBindingParticipantManifestCommitmentV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AppendParticipantReceiptWriteSetV1 {
    parts: AppendParticipantReceiptWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AppendParticipantReceiptWriteSetPartsV1 {
    pub authority: BindingPersistenceAuthorityV1,
    pub expected_operation_revision: BindingOperationRevision,
    pub operation: BindingOperationV1,
    /// Compare-and-set on the participant receipt ledger row for this
    /// operation, and the only site where that row may legitimately not exist
    /// yet.
    ///
    /// `None` ASSERTS THAT THE STORE MUST FIND NO RECEIPT LEDGER ROW FOR THIS
    /// OPERATION. `append_receipt` is the only write in either crate carrying a
    /// [`ParticipantReceiptLedgerV1`] as a next-state member — `put_manifest`
    /// returns a manifest and a cell commitment and no ledger — so the row is
    /// born by the first append. `Some(revision)` asserts a row exists at
    /// exactly that revision, read through
    /// [`ParticipantManifestStore::get_ledger`].
    ///
    /// THE STORE MUST REFUSE RATHER THAN PROCEED WHEN THE ASSERTION IS FALSE:
    /// [`crate::BindingStoreError::Conflict`] both when `None` was claimed and
    /// a row exists, and when `Some` was claimed and the row is absent or at a
    /// different revision.
    ///
    /// The sibling
    /// [`CloseParticipantPhaseWriteSetPartsV1::expected_ledger_revision`] keeps
    /// a required value, and the asymmetry is the point: a phase closes against
    /// a ledger an append already created, so `None` there would assert
    /// something that cannot be true.
    pub expected_ledger_revision: Option<ParticipantReceiptLedgerRevision>,
    pub receipt: VerifiedParticipantReceipt,
    pub work_precondition: ParticipantReceiptWorkPreconditionV1,
    pub next_work_item: ParticipantReceiptWorkItemV1,
    pub next_ledger: ParticipantReceiptLedgerV1,
    pub idempotency: BindingIdempotencyRecordV1,
    pub proof_consumptions: Vec<BindingProofConsumptionV1>,
    pub audit_outbox: BindingAuditRecordV1,
}

impl AppendParticipantReceiptWriteSetV1 {
    pub fn assemble(
        _parts: AppendParticipantReceiptWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &AppendParticipantReceiptWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CloseParticipantPhaseWriteSetV1 {
    parts: CloseParticipantPhaseWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CloseParticipantPhaseWriteSetPartsV1 {
    pub authority: BindingPersistenceAuthorityV1,
    pub expected_operation_revision: BindingOperationRevision,
    pub operation: BindingOperationV1,
    /// Required by value, unlike the same member on
    /// [`AppendParticipantReceiptWriteSetPartsV1`]: a phase closes against a
    /// receipt ledger an append already created, so the row necessarily exists
    /// by the time this write runs and `None` would assert something that
    /// cannot be true.
    pub expected_ledger_revision: ParticipantReceiptLedgerRevision,
    pub closure: VerifiedParticipantPhaseClosure,
    pub idempotency: BindingIdempotencyRecordV1,
    pub proof_consumptions: Vec<BindingProofConsumptionV1>,
    pub audit_outbox: BindingAuditRecordV1,
}

impl CloseParticipantPhaseWriteSetV1 {
    pub fn assemble(
        _parts: CloseParticipantPhaseWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &CloseParticipantPhaseWriteSetPartsV1 {
        &self.parts
    }
}

pub trait ParticipantManifestStore: Send + Sync {
    fn put_manifest<'a>(
        &'a self,
        write_set: &'a PutParticipantManifestWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<PutParticipantManifestResultV1, BindingStoreError>>;

    fn get_manifest<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        tenant_id: &'a TenantId,
        operation: &'a BindingOperationKey,
    ) -> BoxTenancyFuture<'a, Result<Option<PutParticipantManifestResultV1>, BindingStoreError>>;

    fn append_receipt<'a>(
        &'a self,
        write_set: &'a AppendParticipantReceiptWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<ParticipantReceiptLedgerV1, BindingStoreError>>;

    fn close_phase<'a>(
        &'a self,
        write_set: &'a CloseParticipantPhaseWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<SignedParticipantPhaseClosureV1, BindingStoreError>>;

    /// Reads the participant receipt ledger row for one operation and phase.
    ///
    /// `None` MEANS THE STORE LOOKED AND FOUND NO RECEIPT LEDGER ROW. It is not
    /// an error and not an unknown: it is the exact fact a first
    /// [`ParticipantManifestStore::append_receipt`] needs, and it is the value
    /// [`AppendParticipantReceiptWriteSetPartsV1::expected_ledger_revision`]
    /// must carry at that first append.
    fn get_ledger<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        operation: &'a BindingOperationKey,
        phase: ParticipantReceiptPhaseV1,
    ) -> BoxTenancyFuture<'a, Result<Option<ParticipantReceiptLedgerV1>, BindingStoreError>>;

    fn read_member_page<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        request: &'a ParticipantManifestMemberPageRequestV1,
    ) -> BoxTenancyFuture<'a, Result<ParticipantManifestMemberPageV1, BindingStoreError>>;

    fn read_member_page_for_reconciliation<'a>(
        &'a self,
        authority: &'a BindingReconciliationReadAuthorityV1,
        reconciliation_lease: &'a BindingReconciliationLeaseV1,
        request: &'a ParticipantManifestMemberPageRequestV1,
    ) -> BoxTenancyFuture<'a, Result<ParticipantManifestMemberPageV1, BindingStoreError>>;

    fn read_receipt_work_page_for_reconciliation<'a>(
        &'a self,
        authority: &'a BindingReconciliationReadAuthorityV1,
        reconciliation_lease: &'a BindingReconciliationLeaseV1,
        request: &'a ParticipantReceiptWorkPageRequestV1,
    ) -> BoxTenancyFuture<'a, Result<ParticipantReceiptWorkPageV1, BindingStoreError>>;
}
