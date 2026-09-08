use crate::{
    BoxCellFuture, CellControlAuditRecordV1, CellControlIdempotencyRecordV1,
    CellControlOperationPreconditionV1, CellControlOperationV1, CellControlPersistenceAuthorityV1,
    CellLifecycleRevision, CellResourceV1, CellViewV1, DrainProofLedgerRevision,
    DrainProofLedgerV1, VerifiedCellDrainCompletion, VerifiedCellLifecycleTransitionV1,
    VerifiedDrainContributorManifest, VerifiedDrainContributorProof, VerifiedDrainContributorSeal,
};

#[derive(Debug, Eq, PartialEq)]
pub struct BeginDrainWriteSetV1 {
    parts: BeginDrainWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BeginDrainWriteSetPartsV1 {
    pub authority: CellControlPersistenceAuthorityV1,
    pub expected_lifecycle_revision: CellLifecycleRevision,
    pub expected_admission_epoch: crate::CellAdmissionEpoch,
    pub next_resource: CellResourceV1,
    pub lifecycle_transition: VerifiedCellLifecycleTransitionV1,
    pub manifest: VerifiedDrainContributorManifest,
    pub initial_ledger: DrainProofLedgerV1,
    pub operation_precondition: CellControlOperationPreconditionV1,
    pub operation: CellControlOperationV1,
    pub idempotency: CellControlIdempotencyRecordV1,
    pub proof_consumptions: Vec<crate::CellProofConsumptionV1>,
    pub audit_outbox: CellControlAuditRecordV1,
}

impl BeginDrainWriteSetV1 {
    pub fn assemble(
        _parts: BeginDrainWriteSetPartsV1,
    ) -> Result<Self, crate::PlacementContractError> {
        Err(crate::PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &BeginDrainWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct AppendDrainProofWriteSetV1 {
    parts: AppendDrainProofWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AppendDrainProofWriteSetPartsV1 {
    pub authority: CellControlPersistenceAuthorityV1,
    pub expected_lifecycle_revision: CellLifecycleRevision,
    pub expected_ledger_revision: DrainProofLedgerRevision,
    pub proof: VerifiedDrainContributorProof,
    pub contributor_seal: VerifiedDrainContributorSeal,
    pub next_ledger: DrainProofLedgerV1,
    pub operation_precondition: CellControlOperationPreconditionV1,
    pub operation: CellControlOperationV1,
    pub idempotency: CellControlIdempotencyRecordV1,
    pub proof_consumptions: Vec<crate::CellProofConsumptionV1>,
    pub audit_outbox: CellControlAuditRecordV1,
}

impl AppendDrainProofWriteSetV1 {
    pub fn assemble(
        _parts: AppendDrainProofWriteSetPartsV1,
    ) -> Result<Self, crate::PlacementContractError> {
        Err(crate::PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &AppendDrainProofWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CompleteDrainWriteSetV1 {
    parts: CompleteDrainWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CompleteDrainWriteSetPartsV1 {
    pub authority: CellControlPersistenceAuthorityV1,
    pub expected_lifecycle_revision: CellLifecycleRevision,
    pub expected_ledger_revision: DrainProofLedgerRevision,
    pub completion: VerifiedCellDrainCompletion,
    pub next_resource: CellResourceV1,
    pub lifecycle_transition: VerifiedCellLifecycleTransitionV1,
    pub operation_precondition: CellControlOperationPreconditionV1,
    pub operation: CellControlOperationV1,
    pub idempotency: CellControlIdempotencyRecordV1,
    pub proof_consumptions: Vec<crate::CellProofConsumptionV1>,
    pub audit_outbox: CellControlAuditRecordV1,
}

impl CompleteDrainWriteSetV1 {
    pub fn assemble(
        _parts: CompleteDrainWriteSetPartsV1,
    ) -> Result<Self, crate::PlacementContractError> {
        Err(crate::PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &CompleteDrainWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct DecommissionCellWriteSetV1 {
    parts: DecommissionCellWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct DecommissionCellWriteSetPartsV1 {
    pub authority: CellControlPersistenceAuthorityV1,
    pub expected_lifecycle_revision: CellLifecycleRevision,
    pub completion: VerifiedCellDrainCompletion,
    pub next_resource: CellResourceV1,
    pub lifecycle_transition: VerifiedCellLifecycleTransitionV1,
    pub operation_precondition: CellControlOperationPreconditionV1,
    pub operation: CellControlOperationV1,
    pub idempotency: CellControlIdempotencyRecordV1,
    pub proof_consumptions: Vec<crate::CellProofConsumptionV1>,
    pub audit_outbox: CellControlAuditRecordV1,
}

impl DecommissionCellWriteSetV1 {
    pub fn assemble(
        _parts: DecommissionCellWriteSetPartsV1,
    ) -> Result<Self, crate::PlacementContractError> {
        Err(crate::PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &DecommissionCellWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CellDrainMutationResultV1 {
    pub view: CellViewV1,
    pub operation: CellControlOperationV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CellDrainProofMutationResultV1 {
    pub ledger: DrainProofLedgerV1,
    pub operation: CellControlOperationV1,
}

pub trait CellDrainStore: Send + Sync {
    fn begin<'a>(
        &'a self,
        write_set: &'a BeginDrainWriteSetV1,
    ) -> BoxCellFuture<'a, Result<CellDrainMutationResultV1, crate::PlacementContractError>>;

    fn append_proof<'a>(
        &'a self,
        write_set: &'a AppendDrainProofWriteSetV1,
    ) -> BoxCellFuture<'a, Result<CellDrainProofMutationResultV1, crate::PlacementContractError>>;

    /// Reads the drain proof ledger row for one cell and drain term.
    ///
    /// `None` MEANS THE STORE LOOKED AND FOUND NO LEDGER ROW: no drain has been
    /// begun for that term. It is not "the cell does not exist" and not an
    /// authorization refusal; both of those are on the error channel.
    ///
    /// WHY IT EXISTS. [`AppendDrainProofWriteSetPartsV1::expected_ledger_revision`]
    /// is required by value and this store had NO read at all. The ledger is
    /// opened by [`CellDrainStore::begin`], whose result
    /// ([`CellDrainMutationResultV1`]) carries the operation and the cell view
    /// but not the ledger; every later append could take the revision from the
    /// previous [`CellDrainProofMutationResultV1`], but the FIRST append after
    /// a lost reply had no surface to read it from under the
    /// [`crate::CellControlPersistenceAuthorityV1`] the write holds. This read
    /// takes the read twin that authority subsumes.
    fn get_proof_ledger<'a>(
        &'a self,
        authority: &'a crate::CellControlReadAuthorityV1,
        cell_id: &'a crate::CellId,
        drain_term: crate::DrainTermV1,
    ) -> BoxCellFuture<'a, Result<Option<DrainProofLedgerV1>, crate::PlacementContractError>>;

    fn complete<'a>(
        &'a self,
        write_set: &'a CompleteDrainWriteSetV1,
    ) -> BoxCellFuture<'a, Result<CellDrainMutationResultV1, crate::PlacementContractError>>;

    fn decommission<'a>(
        &'a self,
        write_set: &'a DecommissionCellWriteSetV1,
    ) -> BoxCellFuture<'a, Result<CellDrainMutationResultV1, crate::PlacementContractError>>;
}
