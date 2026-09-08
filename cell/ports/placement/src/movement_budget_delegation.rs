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

    /// The reconciliation-authority twin of
    /// [`MovementBudgetDelegationStore::get_authority_state`]. `None` asserts
    /// the same thing.
    ///
    /// WHY IT EXISTS. [`crate::MovementBudgetSettlementWriteSetPartsV1::leaf_authority_precondition`]
    /// is required by value and its write takes a
    /// [`crate::PlacementReconciliationPersistenceAuthorityV1`]. The ordinary
    /// [`MovementBudgetDelegationStore::get_authority_state`] takes
    /// [`PlacementReadAuthorityV1`], which a RECONCILIATION authority does not
    /// subsume: the two are newtypes over different signed invocations
    /// ([`crate::SignedPlacementInvocationV1`] and
    /// [`crate::SignedReconciliationInvocationV1`]), so the persistence-subsumes-read
    /// ordering stated on [`crate::PlacementPersistenceAuthorityV1::read_authority`]
    /// does not cross the two families. Nor does the reconciler get the row in
    /// the subject it is handed: [`crate::CellReconciliationSubjectV1`] does not
    /// reach [`MovementBudgetAuthorityStateV1`] by any transitive route. This
    /// twin is the same remedy `TransferExecutionStore::get_ledger_for_reconciliation`
    /// is for its lane.
    fn get_authority_state_for_reconciliation<'a>(
        &'a self,
        authority: &'a crate::PlacementReconciliationReadAuthorityV1,
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
