use crate::{
    BoxCellFuture, CellControlAuditRecordV1, CellControlIdempotencyRecordV1,
    CellControlOperationKeyV1, CellControlOperationPreconditionV1, CellControlOperationV1,
    CellControlPersistenceAuthorityV1, CellId, CellLifecycleRevision, CellProofConsumptionV1,
    CellProofEnvelopeV1, CellProofVerifier, CellResourceV1, Digest32,
    DrainContributorMutationSetV1, DrainProofLedgerRevision, DrainProofLedgerV1,
    PlacementContractError, ProducerId, ProofVerificationError, RebalanceJobPreconditionV1,
    RebalanceJobV1, VerifiedCellLifecycleTransitionV1,
};

/// Compare-and-set on whichever row the repair targets, one arm per row:
/// [`crate::CellResourceV1`] for `Lifecycle`, [`crate::DrainProofLedgerV1`] for
/// `DrainLedger`, [`crate::RebalanceJobV1`] for `RebalanceJob` and
/// [`crate::CellControlOperationV1`] for `ControlOperation`. Every arm names a
/// row an earlier write opened; a repair never opens one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CellControlRepairTargetPreconditionV1 {
    Lifecycle {
        cell_id: CellId,
        lifecycle_revision: CellLifecycleRevision,
        resource_digest: Digest32,
    },
    DrainLedger {
        cell_id: CellId,
        ledger_revision: DrainProofLedgerRevision,
        ledger_digest: Digest32,
    },
    RebalanceJob {
        cell_id: Option<CellId>,
        precondition: RebalanceJobPreconditionV1,
    },
    ControlOperation {
        operation: CellControlOperationKeyV1,
        precondition: CellControlOperationPreconditionV1,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellControlRepairAuthorityPayloadV1 {
    pub schema_version: u32,
    pub repair_operation: CellControlOperationKeyV1,
    pub target_operation: CellControlOperationKeyV1,
    pub target_precondition: CellControlRepairTargetPreconditionV1,
    pub requested_checkpoint_digest: Digest32,
    pub reason_digest: Digest32,
    pub expires_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedCellControlRepairAuthorityV1 {
    pub payload: CellControlRepairAuthorityPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellControlRepairAuthorityExpectationV1 {
    pub repair_operation: CellControlOperationKeyV1,
    pub target_operation: CellControlOperationKeyV1,
    pub target_precondition: CellControlRepairTargetPreconditionV1,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCellControlRepairAuthority(SignedCellControlRepairAuthorityV1);

impl VerifiedCellControlRepairAuthority {
    #[must_use]
    pub fn signed(&self) -> &SignedCellControlRepairAuthorityV1 {
        &self.0
    }
}

pub fn verify_cell_control_repair_authority(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedCellControlRepairAuthorityV1,
    _expectation: &CellControlRepairAuthorityExpectationV1,
) -> Result<VerifiedCellControlRepairAuthority, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub enum CellControlRepairMutationV1 {
    Lifecycle {
        next_resource: Box<CellResourceV1>,
        transition: Box<VerifiedCellLifecycleTransitionV1>,
    },
    DrainLedger(Box<DrainProofLedgerV1>),
    RebalanceJob(Box<RebalanceJobV1>),
    ControlOperation(Box<CellControlOperationV1>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct CellControlRepairWriteSetV1 {
    parts: CellControlRepairWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CellControlRepairWriteSetPartsV1 {
    pub authority: CellControlPersistenceAuthorityV1,
    pub repair_authority: VerifiedCellControlRepairAuthority,
    pub target_precondition: CellControlRepairTargetPreconditionV1,
    pub target_mutation: CellControlRepairMutationV1,
    pub repair_operation_precondition: CellControlOperationPreconditionV1,
    pub repair_operation: CellControlOperationV1,
    pub drain_mutations: DrainContributorMutationSetV1,
    pub idempotency: CellControlIdempotencyRecordV1,
    pub proof_consumptions: Vec<CellProofConsumptionV1>,
    pub audit_outbox: CellControlAuditRecordV1,
}

impl CellControlRepairWriteSetV1 {
    pub fn assemble(
        _parts: CellControlRepairWriteSetPartsV1,
    ) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &CellControlRepairWriteSetPartsV1 {
        &self.parts
    }
}

pub trait CellControlRepairStore: Send + Sync {
    fn apply_repair<'a>(
        &'a self,
        write_set: &'a CellControlRepairWriteSetV1,
    ) -> BoxCellFuture<'a, Result<CellControlOperationV1, PlacementContractError>>;
}
