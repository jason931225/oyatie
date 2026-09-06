use crate::{
    BoxCellFuture, CellProofConsumptionV1, CommittedMovementPermitIssuanceClaimV1,
    DrainContributorMutationSetV1, MovementBudgetAuthorityPreconditionV1,
    MovementBudgetAuthorityStateV1, MovementBudgetGrantV1, MovementPermitIssuanceRecordV1,
    PlacementAuditRecordV1, PlacementContractError, PlacementIdempotencyRecordV1,
    PlacementOperationPreconditionV1, PlacementOperationV1, PlacementPersistenceAuthorityV1,
    VerifiedBindingParticipantManifestCommitment, VerifiedMovementBudgetLineage,
};

#[derive(Debug, Eq, PartialEq)]
pub struct MovementBudgetGrantWriteSetV1 {
    parts: MovementBudgetGrantWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MovementBudgetGrantWriteSetPartsV1 {
    pub result_precondition: crate::MovementActionResultPreconditionV1,
    pub result: crate::MovementActionResultV1,
    pub authority: PlacementPersistenceAuthorityV1,
    pub leaf_authority_precondition: MovementBudgetAuthorityPreconditionV1,
    pub next_leaf_authority_state: MovementBudgetAuthorityStateV1,
    pub operation_precondition: PlacementOperationPreconditionV1,
    pub operation: PlacementOperationV1,
    pub participant_manifest: VerifiedBindingParticipantManifestCommitment,
    pub budget_lineage: VerifiedMovementBudgetLineage,
    pub grant: MovementBudgetGrantV1,
    pub permit_issuance: MovementPermitIssuanceRecordV1,
    pub drain_mutations: DrainContributorMutationSetV1,
    pub idempotency: PlacementIdempotencyRecordV1,
    pub audit_outbox: PlacementAuditRecordV1,
    pub proof_consumptions: Vec<CellProofConsumptionV1>,
}

impl MovementBudgetGrantWriteSetV1 {
    pub fn assemble(
        _parts: MovementBudgetGrantWriteSetPartsV1,
    ) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &MovementBudgetGrantWriteSetPartsV1 {
        &self.parts
    }
}

pub trait MovementBudgetGrantStore: Send + Sync {
    fn consume_grant<'a>(
        &'a self,
        write_set: &'a MovementBudgetGrantWriteSetV1,
    ) -> BoxCellFuture<'a, Result<CommittedMovementPermitIssuanceClaimV1, PlacementContractError>>;
}
