use crate::{
    BoxCellFuture, CellCapacityLedgerV1, CellCapacityPreconditionV1, CellId,
    CellProofConsumptionV1, Digest32, PlacementActionV1, PlacementAuditRecordV1,
    PlacementContractError, PlacementIdempotencyKey, PlacementIdempotencyRecordV1,
    PlacementOperationKey, PlacementOperationPreconditionV1, PlacementOperationRevision,
    PlacementOperationV1, PlacementPersistenceAuthorityV1, PlacementReadAuthorityV1,
    PlacementRepairAppliedV1, PlacementRepairMutationResultV1, PlacementRepairWriteSetV1,
    PlacementSearchPlanV1, PlacementSelectionOutcomeV1, ReservationRefV1, ReservationStatusV1,
    SignedCellMovementPermitV1, SignedPlacementIntentV1, SignedReservationArmReceiptV1,
    SignedReservationCommitPermitV1, TenantId, VerifiedCellCapacityLedgerV1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellReservationEffectRecordV1 {
    pub operation: PlacementOperationKey,
    pub operation_revision: PlacementOperationRevision,
    pub action: PlacementActionV1,
    pub request_digest: Digest32,
    pub immutable_result_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementOperationEffectV1 {
    InternalCheckpoint {
        checkpoint_digest: Digest32,
    },
    TerminalWithoutExternalEffect {
        terminal_result_digest: Digest32,
    },
    Selection {
        intent: Box<SignedPlacementIntentV1>,
        outcome: Box<PlacementSelectionOutcomeV1>,
    },
    ReservationArmed(Box<SignedReservationArmReceiptV1>),
    CommitPermitIssued(Box<SignedReservationCommitPermitV1>),
    MovementScheduled(Box<SignedCellMovementPermitV1>),
    BindingOutcomeApplied(Box<ReservationStatusV1>),
    SourceReservationReleased(Box<ReservationStatusV1>),
    CancellationCheckpoint,
    RepairApplied(Box<PlacementRepairAppliedV1>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct PlacementOperationWriteSetV1 {
    authority: PlacementPersistenceAuthorityV1,
    precondition: PlacementOperationPreconditionV1,
    operation: PlacementOperationV1,
    search_plan: Option<PlacementSearchPlanV1>,
    idempotency: PlacementIdempotencyRecordV1,
    proof_consumptions: Vec<CellProofConsumptionV1>,
    effect: PlacementOperationEffectV1,
    drain_mutations: crate::DrainContributorMutationSetV1,
    audit_outbox: PlacementAuditRecordV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PlacementOperationWriteSetPartsV1 {
    pub authority: PlacementPersistenceAuthorityV1,
    pub precondition: PlacementOperationPreconditionV1,
    pub operation: PlacementOperationV1,
    pub search_plan: Option<PlacementSearchPlanV1>,
    pub idempotency: PlacementIdempotencyRecordV1,
    pub proof_consumptions: Vec<CellProofConsumptionV1>,
    pub effect: PlacementOperationEffectV1,
    pub drain_mutations: crate::DrainContributorMutationSetV1,
    pub audit_outbox: PlacementAuditRecordV1,
}

impl PlacementOperationWriteSetV1 {
    pub fn assemble(
        _parts: PlacementOperationWriteSetPartsV1,
    ) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn authority(&self) -> &PlacementPersistenceAuthorityV1 {
        &self.authority
    }

    #[must_use]
    pub fn precondition(&self) -> PlacementOperationPreconditionV1 {
        self.precondition
    }

    #[must_use]
    pub fn operation(&self) -> &PlacementOperationV1 {
        &self.operation
    }

    #[must_use]
    pub fn search_plan(&self) -> Option<&PlacementSearchPlanV1> {
        self.search_plan.as_ref()
    }

    #[must_use]
    pub fn idempotency(&self) -> &PlacementIdempotencyRecordV1 {
        &self.idempotency
    }

    #[must_use]
    pub fn proof_consumptions(&self) -> &[CellProofConsumptionV1] {
        &self.proof_consumptions
    }

    #[must_use]
    pub fn effect(&self) -> &PlacementOperationEffectV1 {
        &self.effect
    }

    #[must_use]
    pub fn drain_mutations(&self) -> &crate::DrainContributorMutationSetV1 {
        &self.drain_mutations
    }

    #[must_use]
    pub fn audit_outbox(&self) -> &PlacementAuditRecordV1 {
        &self.audit_outbox
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CellReservationWriteSetV1 {
    authority: PlacementPersistenceAuthorityV1,
    cell_id: CellId,
    admission_precondition: crate::CellAdmissionTermV1,
    drain_mutations: crate::DrainContributorMutationSetV1,
    capacity_precondition: CellCapacityPreconditionV1,
    next_capacity: VerifiedCellCapacityLedgerV1,
    expected_revision: Option<u64>,
    status: ReservationStatusV1,
    operation_precondition: PlacementOperationPreconditionV1,
    operation: PlacementOperationV1,
    effect_record: CellReservationEffectRecordV1,
    idempotency: PlacementIdempotencyRecordV1,
    proof_consumptions: Vec<CellProofConsumptionV1>,
    tenancy_release_proof_consumptions: Vec<crate::TenancyReleaseProofConsumptionV1>,
    audit_outbox: PlacementAuditRecordV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CellReservationWriteSetPartsV1 {
    pub authority: PlacementPersistenceAuthorityV1,
    pub cell_id: CellId,
    /// Compare-and-set on the cell's admission term,
    /// [`crate::CellAdmissionTermV1`]. That term is read here and written
    /// nowhere in these crates, so no write set opens it and the first-value
    /// question does not arise.
    ///
    /// The AUTHORITY question, which is a different one, is discharged by
    /// `CellCatalogReader::read_page`: it takes `PlacementReadAuthorityV1`, the
    /// read twin this write set's `PlacementPersistenceAuthorityV1` subsumes,
    /// and the term rides on `CellCatalogEntryV1::admission_term` inside the
    /// returned page. That is three levels of nesting below the returned type,
    /// so the depth-one read route in the law test cannot see it; the route is
    /// stated here instead. Written in single backticks deliberately: an
    /// intra-doc link on a precondition member is read by the law tests as a
    /// declaration of the ROW this compare-and-set is on, and these are not
    /// that.
    pub admission_precondition: crate::CellAdmissionTermV1,
    pub drain_mutations: crate::DrainContributorMutationSetV1,
    pub capacity_precondition: CellCapacityPreconditionV1,
    pub next_capacity: VerifiedCellCapacityLedgerV1,
    /// Compare-and-set on the reservation effect row this write proposes,
    /// [`crate::CellReservationEffectRecordV1`]. `None` asserts the store must
    /// find no reservation for this key, which is the state a first reservation
    /// is in.
    pub expected_revision: Option<u64>,
    pub status: ReservationStatusV1,
    pub operation_precondition: PlacementOperationPreconditionV1,
    pub operation: PlacementOperationV1,
    pub effect_record: CellReservationEffectRecordV1,
    pub idempotency: PlacementIdempotencyRecordV1,
    pub proof_consumptions: Vec<CellProofConsumptionV1>,
    pub tenancy_release_proof_consumptions: Vec<crate::TenancyReleaseProofConsumptionV1>,
    pub audit_outbox: PlacementAuditRecordV1,
}

impl CellReservationWriteSetV1 {
    pub fn assemble(
        _parts: CellReservationWriteSetPartsV1,
    ) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn authority(&self) -> &PlacementPersistenceAuthorityV1 {
        &self.authority
    }

    #[must_use]
    pub fn cell_id(&self) -> &CellId {
        &self.cell_id
    }

    #[must_use]
    pub fn admission_precondition(&self) -> &crate::CellAdmissionTermV1 {
        &self.admission_precondition
    }

    #[must_use]
    pub fn capacity_precondition(&self) -> &CellCapacityPreconditionV1 {
        &self.capacity_precondition
    }

    #[must_use]
    pub fn drain_mutations(&self) -> &crate::DrainContributorMutationSetV1 {
        &self.drain_mutations
    }

    #[must_use]
    pub fn next_capacity(&self) -> &VerifiedCellCapacityLedgerV1 {
        &self.next_capacity
    }

    #[must_use]
    pub fn expected_revision(&self) -> Option<u64> {
        self.expected_revision
    }

    #[must_use]
    pub fn status(&self) -> &ReservationStatusV1 {
        &self.status
    }

    #[must_use]
    pub fn operation_precondition(&self) -> PlacementOperationPreconditionV1 {
        self.operation_precondition
    }

    #[must_use]
    pub fn operation(&self) -> &PlacementOperationV1 {
        &self.operation
    }

    #[must_use]
    pub fn effect_record(&self) -> &CellReservationEffectRecordV1 {
        &self.effect_record
    }

    #[must_use]
    pub fn idempotency(&self) -> &PlacementIdempotencyRecordV1 {
        &self.idempotency
    }

    #[must_use]
    pub fn proof_consumptions(&self) -> &[CellProofConsumptionV1] {
        &self.proof_consumptions
    }

    #[must_use]
    pub fn tenancy_release_proof_consumptions(&self) -> &[crate::TenancyReleaseProofConsumptionV1] {
        &self.tenancy_release_proof_consumptions
    }

    #[must_use]
    pub fn audit_outbox(&self) -> &PlacementAuditRecordV1 {
        &self.audit_outbox
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CellReservationMutationResultV1 {
    pub reservation: ReservationStatusV1,
    pub operation: PlacementOperationV1,
    pub capacity: CellCapacityLedgerV1,
}

pub trait PlacementOperationStore: Send + Sync {
    fn apply<'a>(
        &'a self,
        write_set: &'a PlacementOperationWriteSetV1,
    ) -> BoxCellFuture<'a, Result<PlacementOperationV1, PlacementContractError>>;

    fn get<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        operation: &'a PlacementOperationKey,
    ) -> BoxCellFuture<'a, Result<Option<PlacementOperationV1>, PlacementContractError>>;

    fn get_idempotent<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        tenant_id: &'a TenantId,
        key: &'a PlacementIdempotencyKey,
    ) -> BoxCellFuture<'a, Result<Option<PlacementIdempotencyRecordV1>, PlacementContractError>>;

    fn get_search_plan<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        operation: &'a PlacementOperationKey,
    ) -> BoxCellFuture<'a, Result<Option<PlacementSearchPlanV1>, PlacementContractError>>;

    fn apply_repair<'a>(
        &'a self,
        write_set: &'a PlacementRepairWriteSetV1,
    ) -> BoxCellFuture<'a, Result<PlacementRepairMutationResultV1, PlacementContractError>>;
}

pub trait CellReservationStore: Send + Sync {
    fn apply<'a>(
        &'a self,
        write_set: &'a CellReservationWriteSetV1,
    ) -> BoxCellFuture<'a, Result<CellReservationMutationResultV1, PlacementContractError>>;

    fn get<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        reservation: &'a ReservationRefV1,
    ) -> BoxCellFuture<'a, Result<Option<ReservationStatusV1>, PlacementContractError>>;
}
