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
    pub expected_ledger_revision: SourceFenceDirectiveLedgerRevision,
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

    fn get_source_fence_directive_ledger<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        operation: &'a BindingOperationKey,
    ) -> BoxTenancyFuture<'a, Result<Option<SourceFenceDirectiveLedgerV1>, BindingStoreError>>;
}
