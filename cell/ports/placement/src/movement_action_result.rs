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
/// [`MovementActionClosureCommitObserver`], and a conforming deployment does not
/// let the store that performed the commit hold that role. The types state the
/// separation; they do not enforce it - see [`MovementActionResultAuthority`]
/// for why, which applies to every verifier on this path.
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

/// Mints the movement-action closure signature.
///
/// Named `Authority` rather than `Signer` to match every other minting port in
/// this crate and its Tenancy peer - `CellMovementAuthority`,
/// `DrainContributorSealAuthority`, `PromotionEconomicsClosureAuthority`,
/// `TransferExecutionPermitAuthority` - and, in this file, its sibling
/// [`MovementActionResultAuthority`].
pub trait MovementActionClosureAuthority: Send + Sync {
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

/// What a result commit observer asserts it read, under committed read
/// isolation, about ONE movement-action result.
///
/// Signed under its own proof domain
/// [`CellProofDomainV1::MovementActionResultCommit`], which is NOT
/// [`CellProofDomainV1::MovementActionResult`]: this witnesses that a result was
/// durably committed, while that one carries the published result itself. An
/// observation can therefore never be presented as the result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementActionResultCommitObservationV1 {
    pub partition: MovementBudgetAuthorityPartition,
    pub key: MovementActionResultKeyV1,
    pub observed_revision: u64,
    pub observed_record_digest: Digest32,
    pub transaction_id: String,
    pub observed_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedMovementActionResultCommitObservationV1 {
    pub payload: MovementActionResultCommitObservationV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

/// A durable result record together with an independent observation of its
/// commit.
///
/// This is a CLAIM, not evidence: only
/// [`verify_committed_movement_action_result`] turns it into the private-field
/// [`VerifiedCommittedMovementActionResult`]. It is produced by
/// [`MovementActionResultCommitObserver`], and a conforming deployment does not
/// let the store that performed the commit hold that role - the same rule
/// [`CommittedMovementActionClosureClaimV1`] states, obeyed by the result path
/// as well as the closure path. It is a deployment obligation either way; see
/// [`MovementActionResultAuthority`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedMovementActionResultClaimV1 {
    pub result: MovementActionResultV1,
    pub observation: SignedMovementActionResultCommitObservationV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCommittedMovementActionResult(CommittedMovementActionResultClaimV1);

impl VerifiedCommittedMovementActionResult {
    #[must_use]
    pub fn claim(&self) -> &CommittedMovementActionResultClaimV1 {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementActionResultCommitExpectationV1 {
    pub partition: MovementBudgetAuthorityPartition,
    pub key: MovementActionResultKeyV1,
    pub expected_record_digest: Digest32,
    pub producer: ProducerId,
    pub audience: ProducerId,
    pub custody_configuration_digest: Digest32,
    pub now_unix_seconds: u64,
}

pub fn verify_committed_movement_action_result(
    _verifier: &dyn CellProofVerifier,
    _claim: CommittedMovementActionResultClaimV1,
    _expectation: &MovementActionResultCommitExpectationV1,
) -> Result<VerifiedCommittedMovementActionResult, PlacementContractError> {
    Err(PlacementContractError::NotImplemented)
}

/// Independently re-reads a committed movement-action result and signs what it
/// read.
///
/// The port accepts the same lookup [`MovementActionResultStore::get_result`]
/// takes and nothing else: never a caller-supplied record, never a caller's
/// claim that a commit occurred.
///
/// `None` means the observer looked and found NOTHING at that key. It is an
/// outcome, not a failure: the observer read under committed read isolation with
/// authority, and there was no committed row. That is precisely the fact
/// `[`MovementActionResultStore::get_result`]` exists to establish - it is what separates
/// "never durably committed" from "committed, reply lost" - so an observer that
/// could not say it would be unable to do its one job.
///
/// It is deliberately NOT `PlacementContractError::NotFoundOrNotAuthorized`.
/// That variant conflates absence with an authorization refusal, on purpose, so
/// that a lookup does not tell an unauthorized caller whether a record exists.
/// An observer already runs under authority, so for it the conflation destroys
/// exactly the distinction it is here to draw.
///
/// A `None` that DISAGREES with `[`MovementActionResultStore::get_result`]` reporting a record is a
/// REFUSAL, never a quiet fallback to "nothing was committed". The caller must
/// not proceed as if the write never happened on the strength of a store report
/// the observer could not corroborate.
pub trait MovementActionResultCommitObserver: Send + Sync {
    fn observe_committed_result<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        partition: &'a MovementBudgetAuthorityPartition,
        key: &'a MovementActionResultKeyV1,
    ) -> BoxCellFuture<
        'a,
        Result<Option<CommittedMovementActionResultClaimV1>, PlacementContractError>,
    >;
}

/// Mints the published movement-action result signature.
///
/// Its only argument is a private-field verified wrapper, so the wrapper must
/// have come from [`verify_committed_movement_action_result`] rather than being
/// constructed by the caller.
///
/// THAT IS A DEPLOYMENT OBLIGATION, NOT A TYPE-LEVEL REFUSAL, and an earlier
/// version of this doc claimed otherwise. The private field refuses DIRECT
/// construction and nothing else. [`CellProofVerifier`] is a public trait and
/// every `verify_*` here takes it as `&dyn`, so one out-of-crate component may
/// implement the verifier and this authority together and mint a wrapper by
/// handing itself in as the judge of authenticity. Nothing in these types
/// prevents that.
///
/// What keeps the checker separate from the checked is therefore the
/// DEPLOYMENT: the verifier implementation, the observer and the storage
/// adapter must be distinct trust domains with distinct signing identities.
/// These contracts express who is SUPPOSED to hold which role and give a
/// conforming deployment the shape to enforce; they do not enforce it.
pub trait MovementActionResultAuthority: Send + Sync {
    fn sign_committed<'a>(
        &'a self,
        committed: &'a VerifiedCommittedMovementActionResult,
    ) -> BoxCellFuture<'a, Result<SignedMovementActionResultV1, PlacementContractError>>;
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
    /// Durably records the rejection, or reads back the already-committed result
    /// for this key, and returns the record it holds. It returns NO envelope and
    /// NO signature.
    ///
    /// The write set it takes carries only the unsigned
    /// [`MovementActionResultV1`], so before this change the storage adapter was
    /// the only party that could have produced the signature it returned: it was
    /// vouching for its own write. The published signature now comes from
    /// [`MovementActionResultAuthority`], over a claim that
    /// [`MovementActionResultCommitObserver`] produced by re-reading.
    fn reject_or_load_grant<'a>(
        &'a self,
        write: &'a MovementActionRejectionWriteSetV1,
    ) -> BoxCellFuture<'a, Result<MovementActionResultV1, PlacementContractError>>;
    /// Reads one already-committed result. Returns the durable record, for the
    /// same reason: nothing on this path durably holds a published signature, so
    /// a store that returned one would have had to mint it.
    fn get_result<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        partition: &'a MovementBudgetAuthorityPartition,
        key: &'a MovementActionResultKeyV1,
    ) -> BoxCellFuture<'a, Result<Option<MovementActionResultV1>, PlacementContractError>>;
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
