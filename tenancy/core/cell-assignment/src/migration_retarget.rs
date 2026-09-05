use cell_placement::CellProofConsumptionV1;

use crate::{
    BindingAuditRecordV1, BindingIdempotencyRecordV1, BindingOperationPreconditionV1,
    BindingOperationV1, BindingPersistenceAuthorityV1, BindingProofConsumptionV1,
    BindingStoreError, BindingWritePrecondition, BoxTenancyFuture, MigrationFenceClaimV1,
    TenantWriteAuthorityAdvanceV1,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MigrationFenceClaimRevision(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MigrationFenceClaimDispositionV1 {
    Active,
    Superseded,
    ConsumedByBindingCommit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationFenceClaimPreconditionV1 {
    pub revision: MigrationFenceClaimRevision,
    pub disposition: MigrationFenceClaimDispositionV1,
    pub record_digest: crate::BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoActiveMigrationFenceClaimPreconditionV1 {
    pub operation: crate::BindingOperationKey,
    pub authority_high_water_revision: crate::TenantWriteAuthorityHighWaterRevision,
    pub active_claim_index_digest: crate::BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MigrationFenceClaimTransitionV1 {
    pub precondition: MigrationFenceClaimPreconditionV1,
    pub next: MigrationFenceClaimV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RetargetMigrationRequestV1 {
    pub superseded_operation: crate::BindingOperationKey,
    pub replacement_operation: crate::BindingOperationKey,
    pub expected_superseded_operation_revision: crate::BindingOperationRevision,
    pub expected_replacement_operation_revision: crate::BindingOperationRevision,
    pub active_claim_precondition: MigrationFenceClaimPreconditionV1,
    pub replacement_claim: MigrationFenceClaimV1,
    pub idempotency_key: crate::BindingIdempotencyKey,
    pub canonical_request_digest: crate::BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MigrationRetargetWriteSetV1 {
    parts: MigrationRetargetWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MigrationRetargetWriteSetPartsV1 {
    pub authority: BindingPersistenceAuthorityV1,
    pub binding_precondition: BindingWritePrecondition,
    pub active_claim_precondition: MigrationFenceClaimPreconditionV1,
    pub superseded_claim: MigrationFenceClaimV1,
    pub replacement_claim: MigrationFenceClaimV1,
    pub superseded_operation_precondition: BindingOperationPreconditionV1,
    pub replacement_operation_precondition: BindingOperationPreconditionV1,
    pub superseded_operation: BindingOperationV1,
    pub replacement_operation: BindingOperationV1,
    pub authority_high_water_advance: TenantWriteAuthorityAdvanceV1,
    pub source_authority_freeze_intent: crate::ServingAuthorityFreezeIntentV1,
    pub source_handoff_precondition: crate::ServingAuthorityHandoffPreconditionV1,
    pub next_source_handoff: crate::ServingAuthorityHandoffRecordV1,
    pub idempotency: BindingIdempotencyRecordV1,
    pub audit_outbox: BindingAuditRecordV1,
    pub cell_proof_consumptions: Vec<CellProofConsumptionV1>,
    pub binding_proof_consumptions: Vec<BindingProofConsumptionV1>,
}

impl MigrationRetargetWriteSetV1 {
    pub fn assemble(_parts: MigrationRetargetWriteSetPartsV1) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &MigrationRetargetWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct MigrationRetargetMutationResultV1 {
    pub superseded_claim: MigrationFenceClaimV1,
    pub replacement_claim: MigrationFenceClaimV1,
    pub superseded_operation: BindingOperationV1,
    pub replacement_operation: BindingOperationV1,
}

pub trait MigrationRetargetStore: Send + Sync {
    fn supersede_and_retarget<'a>(
        &'a self,
        write_set: &'a MigrationRetargetWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<MigrationRetargetMutationResultV1, BindingStoreError>>;
}
