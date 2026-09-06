use crate::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementActionResultKeyV1 {
    pub tenant_id: TenantId,
    pub action: PlacementBusinessActionKeyV1,
    pub business_digest: Digest32,
}

/// The typed address of one movement-action closure.
///
/// This is the closure's identity, separated from its content so that it can be
/// named on its own by [`MovementActionClosureCommitObservationV1`]. The peer
/// [`RebalanceIssuanceAddressV1`] plays the same role for the invocation
/// issuance, and the two are never interchangeable: each is reachable only from
/// its own signed observation type under its own proof domain.
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

/// What a rebalance-source commit observer asserts it read, under committed
/// read isolation, about ONE movement-action closure.
///
/// `address` is the only subject this payload can name, and the type is signed
/// under its own proof domain
/// [`CellProofDomainV1::MovementActionClosureCommit`]. Its peer,
/// [`RebalanceInvocationIssuanceCommitObservationV1`], names a
/// [`RebalanceIssuanceAddressV1`] under
/// [`CellProofDomainV1::RebalanceInvocationIssuanceCommit`]. Separation between
/// the two subjects is therefore carried by the type and the domain, not by a
/// comparison an implementer must remember to perform.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementActionClosureCommitObservationV1 {
    pub address: MovementActionClosureAddressV1,
    pub committed_job_revision: u64,
    pub claim_epoch_at_commit: u64,
    pub record_digest: Digest32,
    pub transaction_id: String,
    pub committed_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedMovementActionClosureCommitObservationV1 {
    pub payload: MovementActionClosureCommitObservationV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

/// What a caller requires of a movement-action closure commit observation.
///
/// No `expected_subject` field is needed: this expectation reaches only
/// [`verify_committed_movement_action_closure`], which accepts only a
/// [`SignedMovementActionClosureCommitObservationV1`], which can name only a
/// [`MovementActionClosureAddressV1`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementActionClosureCommitExpectationV1 {
    pub address: MovementActionClosureAddressV1,
    pub expected_record_digest: Digest32,
    pub producer: ProducerId,
    pub audience: ProducerId,
    pub custody_configuration_digest: Digest32,
    pub now_unix_seconds: u64,
}

/// A durable closure record together with an independent observation of its
/// commit.
///
/// This is a CLAIM, not evidence: only
/// [`verify_committed_movement_action_closure`] turns it into the private-field
/// [`VerifiedCommittedMovementActionClosure`]. It is produced by
/// [`MovementActionClosureCommitObserver`] and NEVER by the store that performed
/// the commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedMovementActionClosureClaimV1 {
    pub closure: MovementActionClosureV1,
    pub observation: SignedMovementActionClosureCommitObservationV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCommittedMovementActionClosure(CommittedMovementActionClosureClaimV1);

impl VerifiedCommittedMovementActionClosure {
    #[must_use]
    pub fn claim(&self) -> &CommittedMovementActionClosureClaimV1 {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedMovementActionClosureV1 {
    pub payload: MovementActionClosureV1,
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
    _claim: CommittedMovementActionClosureClaimV1,
    _expectation: &MovementActionClosureCommitExpectationV1,
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
    pub payload: MovementActionResultV1,
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
    pub closure: CommittedMovementActionClosureClaimV1,
    pub monotonic_restore_epoch: u64,
}

pub fn qualify_movement_action_restore(
    _source: &RebalanceClosureRestoreCheckpointV1,
    _leaf: &MovementActionRestoreCheckpointV1,
) -> Result<(), PlacementContractError> {
    Err(PlacementContractError::NotImplemented)
}
