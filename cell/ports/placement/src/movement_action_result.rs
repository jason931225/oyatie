use crate::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementActionResultKeyV1 {
    pub tenant_id: TenantId,
    pub action: PlacementBusinessActionKeyV1,
    pub business_digest: Digest32,
}

/// The typed address of one movement-action closure.
///
/// This is the closure's identity, separated from its content so that it can
/// be named on its own by
/// [`RebalanceSourceCommitSubjectV1::MovementActionClosure`]. The peer
/// [`RebalanceIssuanceAddressV1`] plays the same role for the other subject a
/// rebalance-source commit observation can witness, so both subjects are
/// compared as one typed address rather than field by field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementActionClosureAddressV1 {
    pub source: RebalanceJobAddressV1,
    pub key: MovementActionResultKeyV1,
    pub closure_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementActionClosureV1 {
    pub address: MovementActionClosureAddressV1,
    pub closed_action_revision: u64,
    pub record_digest: Digest32,
}

/// A durable closure record together with the observation that witnesses its
/// commit. The observation's subject is what binds the two together; see
/// [`RebalanceSourceCommitSubjectV1`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedMovementActionClosureV1 {
    pub closure: MovementActionClosureV1,
    pub commit: SignedRebalanceSourceCommitObservationV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCommittedMovementActionClosure(CommittedMovementActionClosureV1);

impl VerifiedCommittedMovementActionClosure {
    #[must_use]
    pub fn claim(&self) -> &CommittedMovementActionClosureV1 {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedMovementActionClosureV1 {
    pub closure: MovementActionClosureV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedMovementActionClosure(SignedMovementActionClosureV1);

impl VerifiedMovementActionClosure {
    #[must_use]
    pub fn signed(&self) -> &SignedMovementActionClosureV1 {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementActionProofExpectationV1 {
    pub key: MovementActionResultKeyV1,
    pub source: RebalanceJobAddressV1,
    pub leaf_partition: MovementBudgetAuthorityPartition,
    pub producer: ProducerId,
    pub audience: ProducerId,
    pub custody_configuration_digest: Digest32,
    pub now_unix_seconds: u64,
}

pub fn verify_committed_movement_action_closure(
    _verifier: &dyn CellProofVerifier,
    _claim: CommittedMovementActionClosureV1,
    _expectation: &RebalanceCommitExpectationV1,
) -> Result<VerifiedCommittedMovementActionClosure, PlacementContractError> {
    Err(PlacementContractError::NotImplemented)
}

pub fn verify_movement_action_closure(
    _verifier: &dyn CellProofVerifier,
    _claim: SignedMovementActionClosureV1,
    _expectation: &MovementActionProofExpectationV1,
) -> Result<VerifiedMovementActionClosure, PlacementContractError> {
    Err(PlacementContractError::NotImplemented)
}

pub trait MovementActionClosureSigner: Send + Sync {
    fn sign_committed<'a>(
        &'a self,
        claim: &'a VerifiedCommittedMovementActionClosure,
    ) -> BoxCellFuture<'a, Result<SignedMovementActionClosureV1, PlacementContractError>>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MovementActionDispositionV1 {
    Granted {
        grant: Box<MovementBudgetGrantV1>,
        issuance: Box<MovementPermitIssuanceRecordV1>,
    },
    Rejected {
        closure: Box<SignedMovementActionClosureV1>,
    },
    Settled {
        grant: Box<MovementBudgetGrantV1>,
        settlement: Box<MovementBudgetSettlementV1>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementActionResultV1 {
    pub partition: MovementBudgetAuthorityPartition,
    pub key: MovementActionResultKeyV1,
    pub disposition: MovementActionDispositionV1,
    pub revision: u64,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MovementActionResultPreconditionV1 {
    Absent {
        partition: MovementBudgetAuthorityPartition,
        key: MovementActionResultKeyV1,
    },
    Matches {
        partition: MovementBudgetAuthorityPartition,
        key: MovementActionResultKeyV1,
        revision: u64,
        record_digest: Digest32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedMovementActionResultV1 {
    pub result: MovementActionResultV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedMovementActionResult(SignedMovementActionResultV1);

impl VerifiedMovementActionResult {
    #[must_use]
    pub fn signed(&self) -> &SignedMovementActionResultV1 {
        &self.0
    }
}

pub fn verify_movement_action_result(
    _verifier: &dyn CellProofVerifier,
    _claim: SignedMovementActionResultV1,
    _expectation: &MovementActionProofExpectationV1,
) -> Result<VerifiedMovementActionResult, PlacementContractError> {
    Err(PlacementContractError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub struct MovementActionRejectionWriteSetPartsV1 {
    pub closure: VerifiedMovementActionClosure,
    pub precondition: MovementActionResultPreconditionV1,
    pub rejection: MovementActionResultV1,
    pub audit_outbox: PlacementAuditRecordV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MovementActionRejectionWriteSetV1(MovementActionRejectionWriteSetPartsV1);

impl MovementActionRejectionWriteSetV1 {
    pub fn assemble(
        _parts: MovementActionRejectionWriteSetPartsV1,
    ) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &MovementActionRejectionWriteSetPartsV1 {
        &self.0
    }
}

pub trait MovementActionResultStore: Send + Sync {
    fn reject_or_load_grant<'a>(
        &'a self,
        write: &'a MovementActionRejectionWriteSetV1,
    ) -> BoxCellFuture<'a, Result<SignedMovementActionResultV1, PlacementContractError>>;
    fn get_result<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        partition: &'a MovementBudgetAuthorityPartition,
        key: &'a MovementActionResultKeyV1,
    ) -> BoxCellFuture<'a, Result<Option<SignedMovementActionResultV1>, PlacementContractError>>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementActionRestoreCheckpointV1 {
    pub partition: MovementBudgetAuthorityPartition,
    pub key: MovementActionResultKeyV1,
    pub retained_terminal_result: MovementActionResultV1,
    pub monotonic_restore_epoch: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceClosureRestoreCheckpointV1 {
    pub source: RebalanceJobAddressV1,
    pub closure: CommittedMovementActionClosureV1,
    pub monotonic_restore_epoch: u64,
}

pub fn qualify_movement_action_restore(
    _source: &RebalanceClosureRestoreCheckpointV1,
    _leaf: &MovementActionRestoreCheckpointV1,
) -> Result<(), PlacementContractError> {
    Err(PlacementContractError::NotImplemented)
}
