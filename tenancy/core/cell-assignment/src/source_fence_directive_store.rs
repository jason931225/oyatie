use crate::{
    BindingAuditRecordV1, BindingDigest32, BindingIdempotencyRecordV1, BindingOperationKey,
    BindingOperationRevision, BindingOperationV1, BindingPersistenceAuthorityV1,
    BindingProofConsumptionV1, BindingReadAuthorityV1, BindingStoreError, BoxTenancyFuture,
    MigrationFenceClaimV1, SignedSourceFenceDirectiveV1, VerifiedParticipantManifestMember,
    VerifiedSourceFenceDirective,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceFenceDirectiveLedgerRevision(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFenceDirectiveLedgerV1 {
    pub source_authority: crate::ServingAuthorityInstanceV1,
    pub source_authority_freeze_result_digest: BindingDigest32,
    pub committed_source_horizon: crate::ServingAuthorityCommittedIssuanceHorizonV1,
    pub operation: BindingOperationKey,
    pub participant_manifest_digest: BindingDigest32,
    pub expected_participant_root_digest: BindingDigest32,
    pub expected_participant_count: u64,
    pub next_participant_ordinal: u64,
    pub issued_directive_root_digest: BindingDigest32,
    pub issued_directive_count: u64,
    pub revision: SourceFenceDirectiveLedgerRevision,
    pub record_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SourceFenceDirectiveIssueWriteSetV1 {
    parts: SourceFenceDirectiveIssueWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SourceFenceDirectiveIssueWriteSetPartsV1 {
    pub authority: BindingPersistenceAuthorityV1,
    pub expected_operation_revision: BindingOperationRevision,
    pub operation: BindingOperationV1,
    pub migration_fence_claim: MigrationFenceClaimV1,
    pub source_authority_freeze: crate::VerifiedServingAuthorityFreezeResult,
    /// Compare-and-set on the source-fence-directive ledger row for this
    /// operation, and the only site where that row may legitimately not exist
    /// yet.
    ///
    /// `None` ASSERTS THAT THE STORE MUST FIND NO LEDGER ROW FOR THIS
    /// OPERATION: the row is born by this write, so before it there is nothing
    /// to compare against. `Some(revision)` asserts a row exists at exactly
    /// that revision, read through
    /// [`SourceFenceDirectiveStore::get_source_fence_directive_ledger`].
    ///
    /// THE STORE MUST REFUSE RATHER THAN PROCEED WHEN THE ASSERTION IS FALSE:
    /// [`crate::BindingStoreError::Conflict`] both when `None` was claimed and
    /// a row exists, and when `Some` was claimed and the row is absent or at a
    /// different revision. A missing row must not launder into a clean first
    /// write.
    ///
    /// PRE-WAVE DEBT, CLOSED HERE, and closed by the same reasoning the wave
    /// already applied one file over at
    /// [`crate::IssueTransferExecutionPermitWriteSetPartsV1::expected_ledger_revision`].
    /// The crate already treats "no source fence directive ledger exists" as a
    /// legitimate state —
    /// [`crate::BindingMigrationWriteFenceWriteSetPartsV1`] takes the ledger as
    /// `Option` evidence — while the write that would create one demanded a
    /// revision for it by value.
    pub expected_ledger_revision: Option<SourceFenceDirectiveLedgerRevision>,
    pub participant: VerifiedParticipantManifestMember,
    pub directive: VerifiedSourceFenceDirective,
    pub next_ledger: SourceFenceDirectiveLedgerV1,
    pub idempotency: BindingIdempotencyRecordV1,
    pub audit_outbox: BindingAuditRecordV1,
    pub proof_consumptions: Vec<BindingProofConsumptionV1>,
}

impl SourceFenceDirectiveIssueWriteSetV1 {
    pub fn assemble(
        _parts: SourceFenceDirectiveIssueWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &SourceFenceDirectiveIssueWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct SourceFenceDirectiveIssueResultV1 {
    pub directive: SignedSourceFenceDirectiveV1,
    pub ledger: SourceFenceDirectiveLedgerV1,
    pub operation: BindingOperationV1,
}

pub trait SourceFenceDirectiveStore: Send + Sync {
    fn issue_source_fence_directive<'a>(
        &'a self,
        write_set: &'a SourceFenceDirectiveIssueWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<SourceFenceDirectiveIssueResultV1, BindingStoreError>>;

    /// Reads the source-fence-directive ledger row for one operation.
    ///
    /// `None` MEANS THE STORE LOOKED AND FOUND NO LEDGER ROW for this
    /// operation. It is not an error and not an unknown: it is the exact fact a
    /// first [`SourceFenceDirectiveStore::issue_source_fence_directive`] needs,
    /// and it is the value
    /// [`SourceFenceDirectiveIssueWriteSetPartsV1::expected_ledger_revision`]
    /// must carry at that first write.
    fn get_source_fence_directive_ledger<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        operation: &'a BindingOperationKey,
    ) -> BoxTenancyFuture<'a, Result<Option<SourceFenceDirectiveLedgerV1>, BindingStoreError>>;
}
