use crate::{
    BoxCellFuture, CellProofConsumptionV1, CellProofEnvelopeV1, CellProofVerifier, Digest32,
    MovementBudgetAuthorityPartition, MovementBudgetAuthorityPreconditionV1,
    MovementBudgetAuthorityStateV1, MovementBudgetGrantId, MovementBudgetGrantV1,
    PlacementAuditRecordV1, PlacementContractError, PlacementIdempotencyRecordV1,
    PlacementOperationKey, PlacementOperationPreconditionV1, PlacementOperationV1,
    PlacementReconciliationPersistenceAuthorityV1, ProducerId, ProofVerificationError, TenantId,
    VerifiedCellMovementPermit, VerifiedMovementBudgetLineage,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementBudgetSettlementClaimPayloadV1 {
    pub schema_version: u32,
    pub authority_partition: MovementBudgetAuthorityPartition,
    pub tenant_id: TenantId,
    pub operation: PlacementOperationKey,
    pub grant_id: MovementBudgetGrantId,
    pub movement_permit_digest: Digest32,
    pub transfer_execution_ledger_digest: Digest32,
    pub budget_lineage_digest: Digest32,
    pub leaf_scope_allocation_digest: Digest32,
    pub ordinary_debited_bytes: u64,
    pub ordinary_debited_effects: u64,
    pub ordinary_debited_cost_microunits: u64,
    pub forward_debited_bytes: u64,
    pub forward_debited_effects: u64,
    pub forward_debited_cost_microunits: u64,
    pub forward_completion_proven: bool,
    pub claim_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedMovementBudgetSettlementClaimV1 {
    pub payload: MovementBudgetSettlementClaimPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedMovementBudgetSettlementClaim(SignedMovementBudgetSettlementClaimV1);

impl VerifiedMovementBudgetSettlementClaim {
    #[must_use]
    pub fn signed(&self) -> &SignedMovementBudgetSettlementClaimV1 {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementBudgetSettlementClaimExpectationV1 {
    pub authority_partition: MovementBudgetAuthorityPartition,
    pub tenant_id: TenantId,
    pub operation: PlacementOperationKey,
    pub grant_id: MovementBudgetGrantId,
    pub movement_permit_digest: Digest32,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
}

pub fn verify_movement_budget_settlement_claim(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedMovementBudgetSettlementClaimV1,
    _expectation: &MovementBudgetSettlementClaimExpectationV1,
) -> Result<VerifiedMovementBudgetSettlementClaim, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MovementBudgetSettlementRevision(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementBudgetSettlementV1 {
    pub authority_partition: MovementBudgetAuthorityPartition,
    pub grant_id: MovementBudgetGrantId,
    pub operation: PlacementOperationKey,
    pub movement_permit_digest: Digest32,
    pub transfer_execution_ledger_digest: Digest32,
    pub budget_lineage_digest: Digest32,
    pub ordinary_debit_digest: Digest32,
    pub released_forward_reserve_digest: Digest32,
    pub next_leaf_authority_state_digest: Digest32,
    pub revision: MovementBudgetSettlementRevision,
    pub record_digest: Digest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MovementBudgetSettlementWriteSetV1 {
    parts: MovementBudgetSettlementWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MovementBudgetSettlementWriteSetPartsV1 {
    pub result_precondition: crate::MovementActionResultPreconditionV1,
    pub result: crate::MovementActionResultV1,
    pub authority: PlacementReconciliationPersistenceAuthorityV1,
    pub leaf_authority_precondition: MovementBudgetAuthorityPreconditionV1,
    pub next_leaf_authority_state: MovementBudgetAuthorityStateV1,
    pub budget_lineage: VerifiedMovementBudgetLineage,
    pub grant: MovementBudgetGrantV1,
    pub movement_permit: VerifiedCellMovementPermit,
    pub settlement_claim: VerifiedMovementBudgetSettlementClaim,
    pub settlement: MovementBudgetSettlementV1,
    pub operation_precondition: PlacementOperationPreconditionV1,
    pub operation: PlacementOperationV1,
    pub drain_mutations: crate::DrainContributorMutationSetV1,
    pub idempotency: PlacementIdempotencyRecordV1,
    pub proof_consumptions: Vec<CellProofConsumptionV1>,
    pub audit_outbox: PlacementAuditRecordV1,
}

impl MovementBudgetSettlementWriteSetV1 {
    pub fn assemble(
        _parts: MovementBudgetSettlementWriteSetPartsV1,
    ) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &MovementBudgetSettlementWriteSetPartsV1 {
        &self.parts
    }
}

pub trait MovementBudgetSettlementStore: Send + Sync {
    fn get_grant<'a>(
        &'a self,
        authority: &'a crate::PlacementReconciliationReadAuthorityV1,
        authority_partition: &'a MovementBudgetAuthorityPartition,
        grant_id: &'a MovementBudgetGrantId,
    ) -> BoxCellFuture<'a, Result<Option<MovementBudgetGrantV1>, PlacementContractError>>;

    fn settle<'a>(
        &'a self,
        write_set: &'a MovementBudgetSettlementWriteSetV1,
    ) -> BoxCellFuture<'a, Result<MovementBudgetSettlementV1, PlacementContractError>>;

    fn get_settlement<'a>(
        &'a self,
        authority: &'a crate::PlacementReconciliationReadAuthorityV1,
        authority_partition: &'a MovementBudgetAuthorityPartition,
        grant_id: &'a MovementBudgetGrantId,
    ) -> BoxCellFuture<'a, Result<Option<MovementBudgetSettlementV1>, PlacementContractError>>;
}
