use crate::*;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RebalanceJobPartitionV1(String);

impl RebalanceJobPartitionV1 {
    pub fn parse(_value: impl Into<String>) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceJobAddressV1 {
    pub partition: RebalanceJobPartitionV1,
    pub job_id: RebalanceJobId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceJobClaimV1 {
    pub address: RebalanceJobAddressV1,
    pub worker_id: String,
    pub epoch: u64,
    pub revision: u64,
    pub expires_at_unix_seconds: u64,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceSourcePreconditionV1 {
    pub address: RebalanceJobAddressV1,
    pub expected_job_revision: u64,
    pub expected_job_digest: Digest32,
    pub expected_claim: RebalanceJobClaimV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceEvaluationV1 {
    pub address: RebalanceJobAddressV1,
    pub evaluation_id: String,
    pub candidate: RebalanceCandidateV1,
    pub operation: PlacementOperationKey,
    pub requirements_action: PlacementBusinessActionKeyV1,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RebalanceBusinessInputV1 {
    Requirements(Box<RebalanceCandidateRequirementsRequestV1>),
    Selection {
        requirements: Box<RebalanceStableRequirementsV1>,
        purpose: PlacementIntentPurposeV1,
        budget: MovementBudgetRequestV1,
    },
    Movement {
        binding_operation: BindingOutcomeQueryRefV1,
        budget: MovementBudgetRequestV1,
        requested_deadline_unix_seconds: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceBusinessRequestV1 {
    pub key: PlacementBusinessActionKeyV1,
    pub input: RebalanceBusinessInputV1,
    pub business_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RebalanceProofInputsV1 {
    Requirements,
    Selection(Box<SignedPlacementIntentV1>),
    Movement {
        decision: Box<SignedPlacementDecisionV1>,
        reservation: Box<SignedReservationCommitPermitV1>,
        manifest: Box<SignedBindingParticipantManifestCommitmentV1>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceProofPackageV1 {
    pub key: PlacementBusinessActionKeyV1,
    pub business_digest: Digest32,
    pub inputs: RebalanceProofInputsV1,
    pub package_id: String,
    pub expires_at_unix_seconds: u64,
    pub package_digest: Digest32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RebalanceActionStateV1 {
    Eligible,
    AuthorizedResultUnknown,
    Granted,
    ClosureCommitted,
    DurablyRejected,
    Settled,
    RequirementsRead,
    SelectionCompleted,
    NoCapacity,
    Deferred,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceStableRequirementsV1 {
    pub tenant_id: TenantId,
    pub incumbent: PlacementIncumbentV1,
    pub assurance: AssuranceCompilationPayloadV1,
    pub home_capacity: CapacityVectorV1,
    pub capacity_purchase_preference: CapacityPurchasePreferenceV1,
    pub resilience: ResilienceObjectiveV1,
    pub ordinary_movement_ceiling: MovementBudgetV1,
    pub forward_completion_ceiling: MovementBudgetV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceSelectedActionV1 {
    pub evaluation_outcome: Option<RebalanceEvaluationOutcomeV1>,
    pub request: RebalanceBusinessRequestV1,
    pub state: RebalanceActionStateV1,
    pub revision: u64,
    pub ordinary_encumbrance: RebalanceBudgetUsageV1,
    pub forward_encumbrance: RebalanceBudgetUsageV1,
    pub last_leaf_result: Option<SignedMovementActionResultV1>,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceActionPreconditionV1 {
    pub key: PlacementBusinessActionKeyV1,
    pub business_digest: Digest32,
    pub expected_revision: u64,
    pub expected_state: RebalanceActionStateV1,
    pub expected_record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceIssuanceAddressV1 {
    pub job: RebalanceJobAddressV1,
    pub key: PlacementBusinessActionKeyV1,
    pub issuance_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceInvocationIssuanceV1 {
    pub dispatch: PlacementDispatchInputV1,
    pub address: RebalanceIssuanceAddressV1,
    pub business: RebalanceBusinessRequestV1,
    pub package: RebalanceProofPackageV1,
    pub decision: SignedPlacementPolicyDecisionV1,
    pub unsigned_invocation: PlacementInvocationPayloadV1,
    pub dispatch_digest: Digest32,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementDispatchInputV1 {
    pub key: PlacementBusinessActionKeyV1,
    pub business_digest: Digest32,
    pub package_digest: Digest32,
    pub actor: SignedPlacementActorV1,
    pub purpose: PlacementIntentPurposeV1,
    pub realm: RealmId,
    pub jurisdiction: JurisdictionId,
    pub audience: ProducerId,
    pub deadline_unix_seconds: u64,
    pub claim: RebalanceJobClaimV1,
}

pub fn digest_placement_dispatch(
    _input: &PlacementDispatchInputV1,
) -> Result<Digest32, PlacementContractError> {
    Err(PlacementContractError::NotImplemented)
}

/// What a rebalance-source commit observer asserts it read, under committed
/// read isolation, about ONE invocation issuance.
///
/// `address` is the only subject this payload can name, and the type is signed
/// under its own proof domain
/// [`CellProofDomainV1::RebalanceInvocationIssuanceCommit`]. Its peer,
/// [`MovementActionClosureCommitObservationV1`], names a
/// [`MovementActionClosureAddressV1`] under
/// [`CellProofDomainV1::MovementActionClosureCommit`]. The two are different
/// Rust types under different domains, so a witness for one subject cannot be
/// passed where the other is required and cannot pass domain separation if it
/// is presented over the wire. That replaces the single
/// `RebalanceSourceCommit` domain, under which both subjects shared one payload
/// schema and separation could only be a check an implementer had to remember.
///
/// `record_digest` is the digest of the durable record the observer actually
/// found at `address`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceInvocationIssuanceCommitObservationV1 {
    pub address: RebalanceIssuanceAddressV1,
    pub committed_job_revision: u64,
    pub claim_epoch_at_commit: u64,
    pub record_digest: Digest32,
    pub transaction_id: String,
    pub committed_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedRebalanceInvocationIssuanceCommitObservationV1 {
    pub payload: RebalanceInvocationIssuanceCommitObservationV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

/// A durable issuance record together with an independent observation of its
/// commit.
///
/// This is a CLAIM, not evidence, and the distinction is the whole point: only
/// [`verify_committed_rebalance_issuance`] turns it into the private-field
/// [`VerifiedCommittedRebalanceIssuance`], and only that wrapper reaches
/// [`CellPlacementInvocationIssuer::sign_committed`].
///
/// It is produced by [`RebalanceInvocationIssuanceCommitObserver`] and NEVER by
/// the store that performed the commit. "Claim" here is the evidentiary sense;
/// it is unrelated to [`RebalanceJobClaimV1`], which is a worker's lease on a
/// job. The crate already carries both senses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedRebalanceIssuanceClaimV1 {
    pub issuance: RebalanceInvocationIssuanceV1,
    pub observation: SignedRebalanceInvocationIssuanceCommitObservationV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCommittedRebalanceIssuance(CommittedRebalanceIssuanceClaimV1);

impl VerifiedCommittedRebalanceIssuance {
    #[must_use]
    pub fn claim(&self) -> &CommittedRebalanceIssuanceClaimV1 {
        &self.0
    }
}

/// What a caller requires of an invocation-issuance commit observation.
///
/// There is no `expected_subject` here and none is needed: this expectation can
/// only be handed to [`verify_committed_rebalance_issuance`], which only accepts
/// [`SignedRebalanceInvocationIssuanceCommitObservationV1`], which can only name
/// a [`RebalanceIssuanceAddressV1`]. The subject is settled by the types before
/// any field is compared.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceInvocationIssuanceCommitExpectationV1 {
    pub address: RebalanceIssuanceAddressV1,
    pub expected_record_digest: Digest32,
    pub producer: ProducerId,
    pub audience: ProducerId,
    pub custody_configuration_digest: Digest32,
    pub now_unix_seconds: u64,
}

pub fn verify_committed_rebalance_issuance(
    _verifier: &dyn CellProofVerifier,
    _claim: CommittedRebalanceIssuanceClaimV1,
    _expectation: &RebalanceInvocationIssuanceCommitExpectationV1,
) -> Result<VerifiedCommittedRebalanceIssuance, PlacementContractError> {
    Err(PlacementContractError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub struct RebalanceSourceIssuanceWriteSetPartsV1 {
    pub authority: CellControlReconciliationPersistenceAuthorityV1,
    pub precondition: RebalanceSourcePreconditionV1,
    pub action_precondition: RebalanceActionPreconditionV1,
    pub decision: VerifiedPlacementPolicyDecision,
    pub issuance: RebalanceInvocationIssuanceV1,
    pub next_action: RebalanceSelectedActionV1,
    pub next_job: RebalanceJobV1,
    pub idempotency: CellControlIdempotencyRecordV1,
    pub audit_outbox: CellControlAuditRecordV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RebalanceSourceIssuanceWriteSetV1(RebalanceSourceIssuanceWriteSetPartsV1);

impl RebalanceSourceIssuanceWriteSetV1 {
    pub fn assemble(
        _parts: RebalanceSourceIssuanceWriteSetPartsV1,
    ) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &RebalanceSourceIssuanceWriteSetPartsV1 {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceActionPageQueryV1 {
    pub address: RebalanceJobAddressV1,
    pub continuation: Option<Vec<u8>>,
    pub maximum_records: u32,
    pub maximum_encoded_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceActionPageV1 {
    pub address: RebalanceJobAddressV1,
    pub actions: Vec<RebalanceSelectedActionV1>,
    pub continuation: Option<Vec<u8>>,
}

pub trait RebalanceSourceIssuanceStore: Send + Sync {
    /// Durably commits the invocation issuance and returns the record it now
    /// holds. It returns NO attestation and NO signature.
    ///
    /// A store cannot witness its own write. The signed observation that a
    /// commit occurred comes from [`RebalanceInvocationIssuanceCommitObserver`],
    /// which re-reads independently; the two are separate ports so that the same
    /// component cannot both perform the write and vouch for it.
    fn commit_issuance<'a>(
        &'a self,
        write: &'a RebalanceSourceIssuanceWriteSetV1,
    ) -> BoxCellFuture<'a, Result<RebalanceInvocationIssuanceV1, PlacementContractError>>;
    /// Re-reads one issuance by explicit address, for republication after a lost
    /// reply. Absence distinguishes "never durably issued" from "issued, reply
    /// lost". Returns the durable record only, for the same reason
    /// `commit_issuance` does.
    fn load_issuance<'a>(
        &'a self,
        authority: &'a CellControlReconciliationReadAuthorityV1,
        address: &'a RebalanceIssuanceAddressV1,
    ) -> BoxCellFuture<'a, Result<Option<RebalanceInvocationIssuanceV1>, PlacementContractError>>;
    fn list_actions<'a>(
        &'a self,
        authority: &'a CellControlReconciliationReadAuthorityV1,
        query: &'a RebalanceActionPageQueryV1,
    ) -> BoxCellFuture<'a, Result<RebalanceActionPageV1, PlacementContractError>>;
}

/// Independently re-reads a committed invocation issuance and signs what it read.
///
/// The port accepts an address only. It is not given, and cannot be given, a
/// caller-supplied record or a caller's claim that a commit occurred: every
/// field of the emitted observation describes what the observer itself found at
/// that address, under committed read isolation.
///
/// It returns the record it read together with its own attestation rather than
/// the attestation alone. Handing back only the attestation would put the caller
/// in charge of pairing it with a record, which reopens a narrower version of
/// the steering hazard this port exists to close.
pub trait RebalanceInvocationIssuanceCommitObserver: Send + Sync {
    fn observe_committed_issuance<'a>(
        &'a self,
        authority: &'a CellControlReconciliationReadAuthorityV1,
        address: &'a RebalanceIssuanceAddressV1,
    ) -> BoxCellFuture<'a, Result<CommittedRebalanceIssuanceClaimV1, PlacementContractError>>;
}
