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

/// The durable record of one reservation's external effect.
///
/// THIS ROW HAS NO WIRE FORM, deliberately. It is proposed as a successor by
/// [`CellReservationWriteSetPartsV1`] and there is no `message` for it in
/// `cell/placement/v1`, so the ownership rule an adapter reads on the wire
/// cannot reach it: the rule is stated for messages, and this is not one. What
/// binds it instead is the Rust contract on the write set that carries it,
/// together with [`crate::PlacementContractError::ProposedSuccessorMismatch`],
/// the same refusal the wire rule names. A successor row with no message is recorded here rather
/// than left for a sweep to discover as an unmapped name.
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
    /// and the term rides on `CellCatalogCandidateV1::admission_term`, inside
    /// the `CellCatalogPageV1::candidates` of the returned page. That is three
    /// levels of nesting below the returned type, so the depth-one read route
    /// in the law test cannot see it; the route is stated here instead. Written
    /// in single backticks deliberately: an intra-doc link on a precondition
    /// member is read by the law tests as a declaration of the ROW this
    /// compare-and-set is on, and these are not that.
    ///
    /// WHAT THE ROUTE COSTS, stated because a route that is only nameable is
    /// half a route. `read_page` is keyed by partition, snapshot and page
    /// token, never by a single `CellId`, so obtaining one cell's term means
    /// paging a partition-wide catalog until that cell appears. And the term
    /// that comes back rides on a `CellCatalogSnapshotV1` carrying its own
    /// `observed_at_unix_seconds` and `expires_at_unix_seconds`, so it is the
    /// term AS OF THE SNAPSHOT while the store evaluates this compare-and-set
    /// against current state. That gap is a liveness cost and not a safety one:
    /// a term that has moved since the snapshot makes the store REFUSE, which
    /// is the outcome this compare-and-set exists to produce, and the remedy is
    /// a fresher snapshot rather than a wider authority. A caller must not read
    /// the snapshot's expiry as a promise that the term still holds.
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

    /// Reads the cell capacity ledger row for one cell.
    ///
    /// `None` MEANS THE STORE LOOKED AND FOUND NO LEDGER ROW for that cell. It
    /// is not an authorization refusal and not "the cell does not exist"; both
    /// of those are on the error channel.
    ///
    /// WHY IT EXISTS. [`CellReservationWriteSetPartsV1::capacity_precondition`]
    /// is required BY VALUE — [`CellCapacityPreconditionV1`] pins a revision
    /// and a record digest and has no arm asserting the row is absent — and
    /// until this method existed no surface in these crates handed
    /// [`crate::CellCapacityLedgerV1`] back under an authority the write's own
    /// [`PlacementPersistenceAuthorityV1`] can hold. Everything that yielded
    /// the row was in the wrong family or at the wrong depth: `CellResourceStore`
    /// and `CellDrainStore` return it inside a [`crate::CellViewV1`] under
    /// [`crate::CellControlReadAuthorityV1`], a newtype over a DIFFERENT signed
    /// invocation; [`CellReservationMutationResultV1::capacity`] is this
    /// write's own return and is gone after a lost reply; and
    /// [`crate::CellCatalogCandidateV1::capacity`] rides three levels down a
    /// partition-wide catalog page whose values are AS OF A SNAPSHOT. This is
    /// the same gap [`crate::CellDrainStore::get_proof_ledger`] was added to
    /// close on the drain proof ledger, closed the same way: a depth-one read
    /// taking the read twin the write authority subsumes.
    ///
    /// IT ADDS NO AUTHORITY. [`PlacementReadAuthorityV1`] is derivable from
    /// [`PlacementPersistenceAuthorityV1::read_authority`], so a caller able to
    /// perform the write is already able to perform this read, and a read-only
    /// caller reaches it directly. Nothing here lets a reader write.
    fn get_capacity_ledger<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        cell_id: &'a CellId,
    ) -> BoxCellFuture<'a, Result<Option<crate::CellCapacityLedgerV1>, PlacementContractError>>;
}
