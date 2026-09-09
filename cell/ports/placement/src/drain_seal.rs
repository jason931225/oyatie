use crate::{
    BoxCellFuture, CellId, CellProofEnvelopeV1, CellProofVerifier, Digest32,
    DrainContributorKindV1, DrainTermV1, PlacementContractError, ProducerId,
    ProofVerificationError,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DrainContributorStateRevision(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DrainContributorStateDispositionV1 {
    Open,
    Sealed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainContributorStateV1 {
    pub cell_id: CellId,
    pub contributor_kind: DrainContributorKindV1,
    pub contributor_id: String,
    pub disposition: DrainContributorStateDispositionV1,
    pub drain_term: Option<DrainTermV1>,
    pub state_creation_high_water: u64,
    pub state_root_digest: Digest32,
    pub state_count: u64,
    pub revision: DrainContributorStateRevision,
    pub record_digest: Digest32,
}

/// Compare-and-set on the contributor state row, [`DrainContributorStateV1`].
///
/// IT HAS AN ABSENT ARM BECAUSE THE ROW HAS A FIRST WRITE.
/// [`DrainContributorMutationKindV1::Create`] opens the contributor row, and at
/// that mutation there is no prior revision and no prior record digest to pin.
/// The shape used to be a struct requiring both by value, so the only ways to
/// perform the opening write were to invent a revision or to let a missing row
/// launder into a clean first write — which is the shape where two contributors
/// each believe they are opening the ledger and each overwrites the other.
///
/// The mutation kind is not a substitute for this arm. A kind is a statement
/// about what the caller INTENDS; the precondition is what the store COMPARES,
/// and only the second is what a compare-and-set refuses on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DrainContributorMutationPreconditionV1 {
    /// The store must find NO contributor row for this cell and contributor.
    /// This is the state a first mutation is in, and it is the only arm a
    /// [`DrainContributorMutationKindV1::Create`] mutation may carry.
    ///
    /// The refusal when the assertion is false — a row IS present — is
    /// [`crate::PlacementContractError::Conflict`]: a precondition that did not
    /// hold. It is deliberately NOT
    /// [`crate::PlacementContractError::ProposedSuccessorMismatch`], which the
    /// ownership rule head names for a disagreeing PROPOSAL and which is raised
    /// only after every precondition holds.
    Absent {
        cell_id: CellId,
        contributor_id: String,
    },
    /// The store must find a contributor row in exactly this disposition, at
    /// exactly this revision and record digest.
    Present {
        cell_id: CellId,
        contributor_id: String,
        expected_disposition: DrainContributorStateDispositionV1,
        expected_revision: DrainContributorStateRevision,
        expected_record_digest: Digest32,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DrainContributorMutationKindV1 {
    Create,
    UpdateWhileOpen,
    Cleanup,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DrainContributorSubjectRelationKindV1 {
    Absent,
    Present,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainContributorSubjectRelationV1 {
    pub relation_kind: DrainContributorSubjectRelationKindV1,
    pub subject_digest: Digest32,
    pub expected_state_root_digest: Digest32,
    pub relation_proof_path: Vec<Digest32>,
    pub relation_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainContributorStateMutationV1 {
    pub precondition: DrainContributorMutationPreconditionV1,
    pub next_state: DrainContributorStateV1,
    pub mutation_kind: DrainContributorMutationKindV1,
    pub subject_relation: DrainContributorSubjectRelationV1,
    pub mutation_digest: Digest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct DrainContributorMutationSetV1 {
    parts: DrainContributorMutationSetPartsV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainContributorMutationSetPartsV1 {
    pub mutations: Vec<DrainContributorStateMutationV1>,
    pub ordered_mutation_root_digest: Digest32,
    pub mutation_count: u64,
    pub ordered_subject_relation_root_digest: Digest32,
    pub set_digest: Digest32,
}

impl DrainContributorMutationSetV1 {
    pub fn assemble(
        _parts: DrainContributorMutationSetPartsV1,
        _maximum_mutation_count: u32,
        _maximum_relation_proof_depth: u32,
    ) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &DrainContributorMutationSetPartsV1 {
        &self.parts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainContributorSealIntentV1 {
    pub schema_version: u32,
    pub cell_id: CellId,
    pub drain_term: DrainTermV1,
    pub manifest_digest: Digest32,
    pub contributor_kind: DrainContributorKindV1,
    pub contributor_id: String,
    pub sealed_state_revision: DrainContributorStateRevision,
    pub sealed_state_creation_high_water: u64,
    pub sealed_state_root_digest: Digest32,
    pub sealed_state_count: u64,
    pub sealed_state_record_digest: Digest32,
    pub intent_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainContributorSealCommitObservationV1 {
    pub schema_version: u32,
    pub seal_intent_digest: Digest32,
    pub sealed_state_revision: DrainContributorStateRevision,
    pub sealed_state_record_digest: Digest32,
    pub committed_transaction_digest: Digest32,
    /// Read out of the durable record.
    pub committed_at_unix_seconds: u64,
    /// When the observer itself looked.
    pub observed_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedDrainContributorSealCommitObservationV1 {
    pub payload: DrainContributorSealCommitObservationV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CommittedDrainContributorSealClaimV1 {
    pub intent: DrainContributorSealIntentV1,
    pub sealed_state: DrainContributorStateV1,
    pub observation: SignedDrainContributorSealCommitObservationV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainContributorSealCommitObservationExpectationV1 {
    pub seal_intent_digest: Digest32,
    pub sealed_state_revision: DrainContributorStateRevision,
    pub sealed_state_record_digest: Digest32,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCommittedDrainContributorSeal(CommittedDrainContributorSealClaimV1);

impl VerifiedCommittedDrainContributorSeal {
    #[must_use]
    pub fn claim(&self) -> &CommittedDrainContributorSealClaimV1 {
        &self.0
    }
}

pub fn verify_committed_drain_contributor_seal(
    _verifier: &dyn CellProofVerifier,
    _claim: CommittedDrainContributorSealClaimV1,
    _expectation: &DrainContributorSealCommitObservationExpectationV1,
) -> Result<VerifiedCommittedDrainContributorSeal, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainContributorSealPayloadV1 {
    pub schema_version: u32,
    pub intent: DrainContributorSealIntentV1,
    pub commit_observation: SignedDrainContributorSealCommitObservationV1,
    pub seal_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedDrainContributorSealV1 {
    pub payload: DrainContributorSealPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrainContributorSealExpectationV1 {
    pub cell_id: CellId,
    pub drain_term: DrainTermV1,
    pub manifest_digest: Digest32,
    pub contributor_kind: DrainContributorKindV1,
    pub contributor_id: String,
    pub expected_seal_intent_digest: Digest32,
    pub expected_commit_observation_digest: Digest32,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedDrainContributorSeal(SignedDrainContributorSealV1);

impl VerifiedDrainContributorSeal {
    #[must_use]
    pub fn signed(&self) -> &SignedDrainContributorSealV1 {
        &self.0
    }
}

pub fn verify_drain_contributor_seal(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedDrainContributorSealV1,
    _expectation: &DrainContributorSealExpectationV1,
) -> Result<VerifiedDrainContributorSeal, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub struct DrainContributorSealWriteSetV1 {
    parts: DrainContributorSealWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct DrainContributorSealWriteSetPartsV1 {
    /// Compare-and-set on the contributor state row this write proposes,
    /// [`DrainContributorStateV1`].
    ///
    /// THE AUTHORITY QUESTION IS STILL OPEN ON THIS LANE, and it is now open
    /// where it can be seen rather than discharged by a trivial route. This
    /// write set carries no `authority` member — a shape it shares with several
    /// others in both crates rather than uniquely — and
    /// [`DrainContributorSealStore::load_committed_seal`] likewise demands
    /// none, so the two are consistent with each other and inconsistent with
    /// the rest of the wave. The missing authority member, not the
    /// precondition, is what would have to change first, and it is one decision
    /// for the whole lane.
    ///
    /// What is no longer open is the FIRST-VALUE question:
    /// [`DrainContributorMutationPreconditionV1`] carries an
    /// [`DrainContributorMutationPreconditionV1::Absent`] arm, so an opening
    /// mutation asserts the row is not there instead of pinning a revision
    /// nobody can supply.
    pub precondition: DrainContributorMutationPreconditionV1,
    pub next_state: DrainContributorStateV1,
    pub seal_intent: DrainContributorSealIntentV1,
    pub local_idempotency_digest: Digest32,
    pub local_audit_record_digest: Digest32,
}

impl DrainContributorSealWriteSetV1 {
    pub fn assemble(
        _parts: DrainContributorSealWriteSetPartsV1,
    ) -> Result<Self, crate::PlacementContractError> {
        Err(crate::PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &DrainContributorSealWriteSetPartsV1 {
        &self.parts
    }
}

/// Independently re-reads a committed drain-contributor seal and signs what it
/// read.
///
/// WHY THIS PORT EXISTS. A store that signs an observation of its own write
/// vouches for itself, and no amount of downstream signature checking recovers
/// what that destroys. [`DrainContributorSealStore::seal`] used to return a
/// [`CommittedDrainContributorSealClaimV1`] — the durable sealed state AND a
/// signature over the transaction the same call had just performed. The
/// payload carries `committed_transaction_digest` and
/// `committed_at_unix_seconds`, facts only the committing store holds at the
/// moment it commits, so it could never have been an external arrival needing
/// no local producer: the committer was the producer.
///
/// The port accepts the same lookup [`DrainContributorSealStore::load_committed_seal`]
/// takes, plus a read authority, and nothing else: never a caller-supplied
/// record, never a caller's claim that a commit occurred.
///
/// It returns the RECORD TOGETHER WITH its observation rather than the
/// observation alone. Handing back a lone signature would put the caller in
/// charge of pairing it with a record, which reopens a narrower version of the
/// same steering hazard. That is the shape
/// [`crate::PromotionEconomicsCheckpointCommitObserver`] settled on and the one
/// [`crate::MovementActionResultCommitObserver`] uses.
///
/// `None` means the observer looked and found NO committed seal at that key. It
/// is an outcome, not a failure, and it is the fact that separates "never
/// durably sealed" from "sealed, reply lost". A `None` that DISAGREES with
/// [`DrainContributorSealStore::load_committed_seal`] reporting a record is a
/// REFUSAL, never a quiet fallback to "nothing was committed".
///
/// THE PROOF DOMAIN IS NEW, NOT RENAMED. `CellProofDomainV1` tag 27 named a
/// signed statement the STORE made about its own write. This is a different
/// producer making a different trust claim, so reusing that tag would let a
/// signature produced under the old self-attesting semantics validate as an
/// independent observation. Tag 27 is reserved by number and by name in
/// `cell/placement/v1/proof.proto` and replaced by
/// `CELL_PROOF_DOMAIN_V1_DRAIN_CONTRIBUTOR_SEAL_COMMIT_OBSERVATION`, named
/// rather than numbered so the pointer survives a renumber.
///
/// Separate from the store on purpose. Whether the deployed observer is in fact
/// a different party from the deployed store is A DEPLOYMENT OBLIGATION, NOT A
/// TYPE-LEVEL REFUSAL — one process may implement both traits. What the types
/// do is remove the shape in which self-observation was the ONLY implementable
/// one.
pub trait DrainContributorSealCommitObserver: Send + Sync {
    fn observe_committed_seal<'a>(
        &'a self,
        authority: &'a crate::PlacementReadAuthorityV1,
        cell_id: &'a CellId,
        contributor_id: &'a str,
        drain_term: DrainTermV1,
    ) -> BoxCellFuture<
        'a,
        Result<Option<CommittedDrainContributorSealClaimV1>, PlacementContractError>,
    >;
}

pub trait DrainContributorSealStore: Send + Sync {
    /// Durably seals the contributor state and returns THE DURABLE ROW ALONE.
    ///
    /// It never returns a signature, because a signature here would be the
    /// store attesting to its own write. The seal intent this write consumed is
    /// the caller's own input
    /// ([`DrainContributorSealWriteSetPartsV1::seal_intent`]); the commit
    /// signature comes from
    /// [`DrainContributorSealCommitObserver::observe_committed_seal`], which
    /// re-reads the committed row by the same lookup
    /// [`DrainContributorSealStore::load_committed_seal`] takes and returns
    /// [`CommittedDrainContributorSealClaimV1`] — the record together with its
    /// observation.
    fn seal<'a>(
        &'a self,
        write_set: &'a DrainContributorSealWriteSetV1,
    ) -> BoxCellFuture<'a, Result<DrainContributorStateV1, PlacementContractError>>;

    /// Writes the verified commit observation back onto the contributor seal
    /// row, so that [`DrainContributorSealStore::load_committed_seal`] has a
    /// durable source for the signature it returns.
    ///
    /// It returns THE DURABLE ROW ALONE, for the same reason
    /// [`DrainContributorSealStore::seal`] does: the signature it stores was
    /// minted by
    /// [`DrainContributorSealCommitObserver::observe_committed_seal`] before
    /// this call and arrives inside
    /// [`DrainContributorSealPublicationWriteSetPartsV1::committed_seal`], so
    /// this write echoes back a caller-supplied value rather than attesting to
    /// its own commit.
    fn publish_seal<'a>(
        &'a self,
        write_set: &'a DrainContributorSealPublicationWriteSetV1,
    ) -> BoxCellFuture<'a, Result<DrainContributorStateV1, PlacementContractError>>;

    /// Reads the committed contributor seal row.
    ///
    /// `None` MEANS THE STORE LOOKED AND FOUND NO COMMITTED SEAL for that cell,
    /// contributor and drain term.
    ///
    /// THE OBSERVATION IT RETURNS IS SERVICEABLE.
    /// [`CommittedDrainContributorSealClaimV1::observation`] is written by
    /// [`DrainContributorSealStore::publish_seal`], which carries it in
    /// verified form. Without that write this getter promised a value no write
    /// here could store, and the only implementation left would have minted the
    /// signature on the read.
    fn load_committed_seal<'a>(
        &'a self,
        cell_id: &'a CellId,
        contributor_id: &'a str,
        drain_term: DrainTermV1,
    ) -> BoxCellFuture<
        'a,
        Result<Option<CommittedDrainContributorSealClaimV1>, PlacementContractError>,
    >;
}

#[derive(Debug, Eq, PartialEq)]
pub struct DrainContributorSealPublicationWriteSetV1 {
    parts: DrainContributorSealPublicationWriteSetPartsV1,
}

/// Writes the independent commit observation back onto the row the loader
/// reads.
///
/// WHY IT EXISTS. [`DrainContributorSealStore::load_committed_seal`] returns
/// [`CommittedDrainContributorSealClaimV1`], whose third member is a
/// [`SignedDrainContributorSealCommitObservationV1`]. Before this write set no
/// write on this store carried that value, so the getter promised a signature
/// nothing durable could put there and the only way to service it was to mint
/// the signature ON THE READ — which is the self-attestation the observer
/// barrier exists to close, reopened at the getter. The two sibling lanes got
/// their write-back path when production moved to an observer
/// ([`crate::MovementPermitPublicationWriteSetPartsV1::committed_issuance`]
/// and `SourceReleasePublicationWriteSetPartsV1::committed_issuance`); this
/// lane got the observer and not the path.
///
/// THE OBSERVATION IS ALREADY PERSISTED SOMEWHERE ELSE, AND THAT IS NOT THIS.
/// It rides inside [`SignedDrainContributorSealV1`] in
/// [`crate::AppendDrainProofWriteSetPartsV1::contributor_seal`], which is a
/// different store trait ([`crate::CellDrainStore`]) writing a different row
/// (the drain proof ledger). Read per-record — which is the scope this wave's
/// law is stated at — a value on the proof ledger does not service a getter on
/// the contributor seal row.
#[derive(Debug, Eq, PartialEq)]
pub struct DrainContributorSealPublicationWriteSetPartsV1 {
    /// Compare-and-set on the contributor state row this write advances,
    /// [`DrainContributorStateV1`].
    ///
    /// OPEN, for the same reason and in the same words as
    /// [`DrainContributorSealWriteSetPartsV1::precondition`]: this write set
    /// carries no `authority` member, so the question "is this value readable
    /// under the authority this write takes" has no subject. The authority
    /// question on this lane is one decision, taken once, for the seal write,
    /// the publication write and
    /// [`DrainContributorSealStore::load_committed_seal`] together; splitting
    /// it here would put half the lane on an axis the other half is not on.
    pub precondition: DrainContributorMutationPreconditionV1,
    /// The observation the loader hands back, made durable on the row it is
    /// read from. Private-field: only
    /// [`verify_committed_drain_contributor_seal`] mints one, so a caller
    /// cannot restate a signature it did not have verified.
    pub committed_seal: VerifiedCommittedDrainContributorSeal,
    /// The signed seal the authority produced from that observation, which is
    /// what downstream proof consumption spends.
    pub seal: VerifiedDrainContributorSeal,
    pub published_state: DrainContributorStateV1,
    pub local_idempotency_digest: Digest32,
    pub local_audit_record_digest: Digest32,
}

impl DrainContributorSealPublicationWriteSetV1 {
    pub fn assemble(
        _parts: DrainContributorSealPublicationWriteSetPartsV1,
    ) -> Result<Self, crate::PlacementContractError> {
        Err(crate::PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &DrainContributorSealPublicationWriteSetPartsV1 {
        &self.parts
    }
}

pub trait DrainContributorSealAuthority: Send + Sync {
    fn sign_committed<'a>(
        &'a self,
        claim: &'a VerifiedCommittedDrainContributorSeal,
    ) -> BoxCellFuture<'a, Result<VerifiedDrainContributorSeal, PlacementContractError>>;
}
