use crate::{
    BoxCellFuture, CellProofConsumptionV1, DrainContributorMutationSetV1,
    MovementBudgetAuthorityPartition, MovementBudgetAuthorityPreconditionV1,
    MovementBudgetAuthorityStateV1, MovementBudgetDelegationId, MovementBudgetDelegationV1,
    MovementBudgetScopeV1, PlacementAuditRecordV1, PlacementContractError,
    PlacementIdempotencyRecordV1, PlacementOperationPreconditionV1, PlacementOperationV1,
    PlacementPersistenceAuthorityV1, PlacementReadAuthorityV1,
};

/// Compare-and-set on the CHILD budget authority row:
/// [`MovementBudgetAuthorityStateV1`]. `Absent` asserts the store must find no
/// authority for that partition and scope, which is the state the first
/// delegation into a scope is in -- the child row is born by this very write,
/// while the parent row named by
/// [`MovementBudgetDelegationWriteSetPartsV1::parent_precondition`] must
/// already exist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MovementBudgetChildStatePreconditionV1 {
    Absent {
        authority_partition: MovementBudgetAuthorityPartition,
        scope: MovementBudgetScopeV1,
    },
    Matches(MovementBudgetAuthorityPreconditionV1),
}

#[derive(Debug, Eq, PartialEq)]
pub struct MovementBudgetDelegationWriteSetV1 {
    parts: MovementBudgetDelegationWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MovementBudgetDelegationWriteSetPartsV1 {
    pub authority: PlacementPersistenceAuthorityV1,
    pub parent_precondition: MovementBudgetAuthorityPreconditionV1,
    pub next_parent_state: MovementBudgetAuthorityStateV1,
    pub child_precondition: MovementBudgetChildStatePreconditionV1,
    pub next_child_state: MovementBudgetAuthorityStateV1,
    pub delegation: MovementBudgetDelegationV1,
    pub operation_precondition: PlacementOperationPreconditionV1,
    pub operation: PlacementOperationV1,
    pub drain_mutations: DrainContributorMutationSetV1,
    pub idempotency: PlacementIdempotencyRecordV1,
    pub audit_outbox: PlacementAuditRecordV1,
    pub proof_consumptions: Vec<CellProofConsumptionV1>,
}

impl MovementBudgetDelegationWriteSetV1 {
    pub fn assemble(
        _parts: MovementBudgetDelegationWriteSetPartsV1,
    ) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &MovementBudgetDelegationWriteSetPartsV1 {
        &self.parts
    }
}

pub trait MovementBudgetDelegationStore: Send + Sync {
    fn allocate<'a>(
        &'a self,
        write_set: &'a MovementBudgetDelegationWriteSetV1,
    ) -> BoxCellFuture<'a, Result<MovementBudgetDelegationV1, PlacementContractError>>;

    fn get_authority_state<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        partition: &'a MovementBudgetAuthorityPartition,
        scope: &'a MovementBudgetScopeV1,
    ) -> BoxCellFuture<'a, Result<Option<MovementBudgetAuthorityStateV1>, PlacementContractError>>;

    fn get_delegation<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        partition: &'a MovementBudgetAuthorityPartition,
        delegation_id: &'a MovementBudgetDelegationId,
    ) -> BoxCellFuture<'a, Result<Option<MovementBudgetDelegationV1>, PlacementContractError>>;
}
