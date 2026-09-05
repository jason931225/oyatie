use crate::{
    BindingAuditRecordV1, BindingIdempotencyRecordV1, BindingOperationRevision, BindingOperationV1,
    BindingPersistenceAuthorityV1, BindingProofConsumptionV1, BindingStoreError,
    BindingWritePrecondition, MigrationFenceClaimV1, SignedWriteFenceV1,
    SourceFenceDirectiveLedgerV1, VerifiedRetiredSourceEffectClosureV1, VerifiedWriteFence,
};

#[derive(Debug, Eq, PartialEq)]
pub struct BindingMigrationWriteFenceWriteSetV1 {
    authority: BindingPersistenceAuthorityV1,
    binding_precondition: BindingWritePrecondition,
    expected_operation_revision: BindingOperationRevision,
    migration_fence_claim: MigrationFenceClaimV1,
    source_authority_retirement: crate::VerifiedServingAuthorityRetirementV1,
    source_fence_directive_ledger: Option<SourceFenceDirectiveLedgerV1>,
    source_fencing_completion: VerifiedRetiredSourceEffectClosureV1,
    write_fence: VerifiedWriteFence,
    operation: BindingOperationV1,
    idempotency: BindingIdempotencyRecordV1,
    audit_outbox: BindingAuditRecordV1,
    binding_proof_consumptions: Vec<BindingProofConsumptionV1>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingMigrationWriteFenceWriteSetPartsV1 {
    pub authority: BindingPersistenceAuthorityV1,
    pub binding_precondition: BindingWritePrecondition,
    pub expected_operation_revision: BindingOperationRevision,
    pub migration_fence_claim: MigrationFenceClaimV1,
    pub source_authority_retirement: crate::VerifiedServingAuthorityRetirementV1,
    pub source_fence_directive_ledger: Option<SourceFenceDirectiveLedgerV1>,
    pub source_fencing_completion: VerifiedRetiredSourceEffectClosureV1,
    pub write_fence: VerifiedWriteFence,
    pub operation: BindingOperationV1,
    pub idempotency: BindingIdempotencyRecordV1,
    pub audit_outbox: BindingAuditRecordV1,
    pub binding_proof_consumptions: Vec<BindingProofConsumptionV1>,
}

impl BindingMigrationWriteFenceWriteSetV1 {
    pub fn assemble(
        _parts: BindingMigrationWriteFenceWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn authority(&self) -> &BindingPersistenceAuthorityV1 {
        &self.authority
    }

    #[must_use]
    pub fn binding_precondition(&self) -> &BindingWritePrecondition {
        &self.binding_precondition
    }

    #[must_use]
    pub fn expected_operation_revision(&self) -> BindingOperationRevision {
        self.expected_operation_revision
    }

    #[must_use]
    pub fn source_authority_retirement(&self) -> &crate::VerifiedServingAuthorityRetirementV1 {
        &self.source_authority_retirement
    }

    #[must_use]
    pub fn migration_fence_claim(&self) -> &MigrationFenceClaimV1 {
        &self.migration_fence_claim
    }

    #[must_use]
    pub fn source_fence_directive_ledger(&self) -> Option<&SourceFenceDirectiveLedgerV1> {
        self.source_fence_directive_ledger.as_ref()
    }

    #[must_use]
    pub fn source_fencing_completion(&self) -> &VerifiedRetiredSourceEffectClosureV1 {
        &self.source_fencing_completion
    }

    #[must_use]
    pub fn write_fence(&self) -> &VerifiedWriteFence {
        &self.write_fence
    }

    #[must_use]
    pub fn operation(&self) -> &BindingOperationV1 {
        &self.operation
    }

    #[must_use]
    pub fn idempotency(&self) -> &BindingIdempotencyRecordV1 {
        &self.idempotency
    }

    #[must_use]
    pub fn audit_outbox(&self) -> &BindingAuditRecordV1 {
        &self.audit_outbox
    }

    #[must_use]
    pub fn proof_consumptions(&self) -> &[BindingProofConsumptionV1] {
        &self.binding_proof_consumptions
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct MigrationWriteFenceMutationResultV1 {
    pub write_fence: SignedWriteFenceV1,
    pub operation: BindingOperationV1,
}
