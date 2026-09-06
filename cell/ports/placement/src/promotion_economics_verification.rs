//! Bounded, resumable replay of a promotion economics record against its
//! retained inputs, with partition-local durable checkpoints.
//!
//! Bounded work protects control-plane latency. A cap on the TOTAL size of an
//! estate does not: it makes a large but entirely well-formed cell permanently
//! ineligible for promotion, which is a size test wearing the costume of a
//! correctness test. So the budgets here are per STEP, and exhausting one
//! commits durable progress and returns
//! [`PromotionEconomicsVerificationStepOutcomeV1::Continued`]. Verification
//! resumes from that checkpoint; it never restarts, never samples, and never
//! reports failed readiness merely because a step ended.
//!
//! Memory stays flat in the size of the estate: one bounded page at a time, one
//! stream cursor, scalar accumulators, bounded hash state, and the last
//! ordering key. There is no in-memory duplicate-key set, no fan-out inside a
//! step, and no global input set. Duplicate and interval-overlap detection
//! works because the streams are canonically ordered, so a repeat is adjacent.
//!
//! Everything here is a declaration; every constructor, store and reader fails
//! closed with a typed `NotImplemented`.

use crate::{
    BoxCellFuture, CellControlReadAuthorityV1, CellId, CellProofEnvelopeV1, CellProofVerifier,
    CellRevisionIdentityV1, Digest32, PlacementPartitionV1, ProducerId,
    PromotionCostCategoryTotalV1, PromotionEconomicsInputMemberV1, PromotionEconomicsPolicyV1,
    PromotionEconomicsVerificationErrorV1, PromotionEconomicsWindowV1,
    SignedPromotionEconomicsSourceFinalizationV1, VerifiedCellPromotionEconomics,
    VerifiedPromotionEconomicsClosure,
};

/// One page request over one admitted source's retained records.
///
/// The request names the exact snapshot to read, so a mutable "latest" result
/// can never be substituted for the finalized bytes the closure committed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsRetainedPageRequestV1 {
    pub partition: PlacementPartitionV1,
    pub closure_digest: Digest32,
    pub source_ordinal: u64,
    pub snapshot_id: String,
    pub snapshot_revision: u64,
    pub first_record_ordinal: u64,
    pub maximum_rows: u32,
    pub maximum_bytes: u64,
}

/// One bounded page of retained records from one admitted source.
///
/// The returned partition, closure digest, source ordinal, finalization and
/// first record ordinal must all match the checkpoint that asked for them; a
/// mismatch is a reader contract violation, not a resumable condition. Every
/// page is decoded under the byte limit rather than after allocating for it.
///
/// The cell's retained-input adapter admits a maximum single canonical row
/// size. Oversized native objects are projected into bounded typed operands
/// carrying the exact finalized source reference; they are never loaded
/// recursively during promotion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsRetainedPageV1 {
    pub partition: PlacementPartitionV1,
    pub closure_digest: Digest32,
    pub source_ordinal: u64,
    pub first_record_ordinal: u64,
    pub finalization: SignedPromotionEconomicsSourceFinalizationV1,
    pub records: Vec<PromotionEconomicsInputMemberV1>,
    pub canonical_bytes: u64,
}

pub trait PromotionEconomicsRetainedInputReader: Send + Sync {
    fn read_page<'a>(
        &'a self,
        authority: &'a CellControlReadAuthorityV1,
        request: &'a PromotionEconomicsRetainedPageRequestV1,
    ) -> BoxCellFuture<
        'a,
        Result<PromotionEconomicsRetainedPageV1, PromotionEconomicsVerificationErrorV1>,
    >;
}

/// Which phase of verification a checkpoint is in.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionEconomicsVerificationPhaseV1 {
    VerifyingSourceClosure,
    ReplayingInputs,
    Complete,
}

/// The durable identity of one verification.
///
/// Keys are content-addressed by what the work is over, not by a job id, so a
/// resumed step that finds a changed cell, closure, registry, policy or
/// calculation is working on different work and must not inherit the old
/// accumulators.
///
/// `SourceClosure` is keyed on the registry digest rather than a closure
/// digest, because the closure digest is that phase's output and does not exist
/// while the phase runs.
///
/// EVERY KEY COMMITS EVERYTHING ITS CHECKPOINT PINS. `SourceClosure` carries
/// `cell_revision_identity_digest` for that reason: a registry digest binds
/// partition, cell id and generation but NOT the cell revision, so without it
/// an ordinary capacity-revision bump would leave the key unchanged while the
/// checkpoint's `expected_cell` no longer matched — addressing the same durable
/// slot with incompatible work. `InputReplay` needs no such field because its
/// `closure_digest` transitively commits the revision through the closure's own
/// `cell`. With both, a changed cell revision yields a DIFFERENT KEY and
/// therefore a fresh verification, rather than a collision against retained
/// accumulators that must not be inherited.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PromotionEconomicsVerificationKeyV1 {
    SourceClosure {
        partition: PlacementPartitionV1,
        cell_id: CellId,
        /// `CellRevisionIdentityV1::revision_identity_digest` of the cell
        /// revision this verification is over.
        cell_revision_identity_digest: Digest32,
        registry_digest: Digest32,
        window: PromotionEconomicsWindowV1,
        policy_digest: Digest32,
    },
    InputReplay {
        partition: PlacementPartitionV1,
        cell_id: CellId,
        closure_digest: Digest32,
        calculation_digest: Digest32,
    },
}

/// Where a verification stopped and what it needs to continue.
///
/// The hash-state and record-key byte fields have format-defined bounded sizes
/// validated by the pinned calculation encoding version. They are NOT arbitrary
/// serialized hasher internals: a checkpoint that could carry an unbounded or
/// implementation-defined blob would be both an unbounded-memory hazard and an
/// untrusted-input decoder.
///
/// `previous_canonical_record_key` and `previous_interval_end_unix_seconds` are
/// the entire duplicate- and overlap-detection state. They suffice because the
/// streams are canonically ordered, so a repeated charge identity is adjacent
/// even when its category differs, and an overlapping interval for one
/// reservation is adjacent. Source key ranges may split between reservation
/// identities but never inside one reservation's intervals, which is what keeps
/// overlap detection correct across a source boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsVerificationCursorV1 {
    pub phase: PromotionEconomicsVerificationPhaseV1,
    pub next_source_ordinal: u64,
    pub next_record_ordinal: u64,
    pub source_records_verified: u64,
    pub source_bytes_verified: u64,
    pub source_ordered_hash_state: Vec<u8>,
    pub closure_ordered_hash_state: Vec<u8>,
    pub previous_canonical_record_key: Option<Vec<u8>>,
    pub previous_interval_end_unix_seconds: Option<u64>,
}

/// Durable partition-local progress for one verification key.
///
/// `revision` is the compare-and-set token: a commit whose expected revision no
/// longer matches loses to the writer that got there first, and expired leases
/// cannot commit at all. Conflicts retry from persisted progress rather than
/// from the beginning.
///
/// `category_totals` has exactly the admitted taxonomy's length, and the
/// taxonomy definition has a schema-bounded maximum size. That bound is on the
/// TAXONOMY, not on the estate: it limits how many accumulators exist, never
/// how many records may be verified.
///
/// `retained_until_unix_seconds` records the minimum authenticated source
/// retention deadline. Pausing and resuming is allowed only while every input
/// is still retained and the policy and cell identity still apply. Missing
/// retained bytes or an invalidated closure cannot be cured by an old
/// checkpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsVerificationCheckpointV1 {
    pub key: PromotionEconomicsVerificationKeyV1,
    pub revision: u64,
    pub expected_cell: CellRevisionIdentityV1,
    pub registry_digest: Digest32,
    pub policy_digest: Digest32,
    pub cursor: PromotionEconomicsVerificationCursorV1,
    pub category_totals: Vec<PromotionCostCategoryTotalV1>,
    pub raw_capacity_unit_seconds: u64,
    pub retained_until_unix_seconds: u64,
    pub updated_at_unix_seconds: u64,
    pub checkpoint_digest: Digest32,
}

/// What a checkpoint commit observer asserts it read, under committed read
/// isolation, at ONE verification key.
///
/// Every field describes what the OBSERVER itself found by re-reading. The
/// observer is given a lookup key and nothing else — never a caller's record,
/// never a caller's claim that a commit occurred — so it cannot be steered into
/// attesting to something it did not see.
///
/// `committed_at_unix_seconds` is read out of the durable record;
/// `observed_at_unix_seconds` is when the observer read it. Keeping both
/// separate is deliberate: an observer can testify to when it looked, and only
/// repeat what the record says about when the write happened.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsCheckpointCommitObservationV1 {
    pub schema_version: u32,
    pub partition: PlacementPartitionV1,
    pub key: PromotionEconomicsVerificationKeyV1,
    pub revision: u64,
    pub checkpoint_digest: Digest32,
    pub committed_at_unix_seconds: u64,
    pub observed_at_unix_seconds: u64,
}

/// Signed under
/// [`crate::CellProofDomainV1::PromotionEconomicsCheckpointCommitObservation`],
/// by the observer port's own signing identity — NEVER by the store that
/// performed the write.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedPromotionEconomicsCheckpointCommitObservationV1 {
    pub payload: PromotionEconomicsCheckpointCommitObservationV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

/// A durable checkpoint record together with an independent observation of its
/// commit.
///
/// This is a CLAIM, not evidence: only
/// [`verify_promotion_economics_checkpoint`] turns it into the private-field
/// [`VerifiedPromotionEconomicsCheckpoint`]. It is produced by
/// [`PromotionEconomicsCheckpointCommitObserver`] and NEVER by the store that
/// performed the commit.
///
/// The observer returns the record it read together with its own observation,
/// rather than the observation alone, for the reason the sibling rebalance
/// observer gives: handing back only a signature would put the caller in charge
/// of pairing it with a record, which reopens a narrower version of the
/// steering hazard this port exists to close.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedPromotionEconomicsCheckpointClaimV1 {
    pub checkpoint: PromotionEconomicsVerificationCheckpointV1,
    pub observation: SignedPromotionEconomicsCheckpointCommitObservationV1,
}

/// Independently re-reads one committed checkpoint and signs what it read.
///
/// The port accepts a verification KEY only. It is not given, and cannot be
/// given, a caller-supplied checkpoint or a caller's assertion that a commit
/// occurred. This is the second half of the wave law that the store fix alone
/// does not satisfy: a store that signs an observation of its own write vouches
/// for itself, so the signature has to come from somewhere the write did not.
///
/// `None` means the observer found NOTHING at that key. When
/// [`PromotionEconomicsCheckpointStore::acquire`] reported retained progress
/// and the observer then finds none, that disagreement is a REFUSAL, never a
/// quiet fallback to "no progress": the caller must not restart the
/// verification from zero on the strength of a store report the observer could
/// not corroborate. Only a `None` corroborating an `acquire` that also found
/// nothing means a genuinely fresh verification.
pub trait PromotionEconomicsCheckpointCommitObserver: Send + Sync {
    fn observe_committed_checkpoint<'a>(
        &'a self,
        authority: &'a CellControlReadAuthorityV1,
        key: &'a PromotionEconomicsVerificationKeyV1,
    ) -> BoxCellFuture<
        'a,
        Result<
            Option<CommittedPromotionEconomicsCheckpointClaimV1>,
            PromotionEconomicsVerificationErrorV1,
        >,
    >;
}

/// What the owning module independently expects a restored checkpoint to be.
///
/// The expectation is computed by the verifier from the work it is resuming,
/// never copied out of the claim: a claim that supplies its own expectation
/// proves nothing.
///
/// The three identity fields have a source: the observer admission on the
/// policy. Before that admission existed this type could not be populated by
/// the one caller that must build it — `advance_cell_promotion_economics`
/// constructs this expectation itself, and no input it held named an observer.
/// A verifier holding an expectation nothing can fill is the same non-check one
/// level down: rigorous-looking, comparing against nothing.
///
/// There is deliberately NO `expected_checkpoint_digest`. A caller resuming
/// after a crash does not know what digest it is about to find — discovering
/// retained progress is the point — so requiring one would either be
/// unsatisfiable or would be satisfied by copying the claim's own digest, which
/// checks nothing. The verifier binds the observation to the key and to the
/// cell, registry and policy identity the caller independently holds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsCheckpointExpectationV1 {
    pub key: PromotionEconomicsVerificationKeyV1,
    pub expected_cell: CellRevisionIdentityV1,
    pub expected_registry_digest: Digest32,
    pub expected_policy_digest: Digest32,
    /// All three identity fields are populated from
    /// [`crate::PromotionEconomicsPolicyV1::checkpoint_observer`], the cell's
    /// admission of the observer, which the verified closure's `policy_digest`
    /// commits. They are never taken from the observation being checked: an
    /// expectation copied out of its own subject compares a value against
    /// itself.
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub expected_signing_key_id: crate::KeyId,
    pub now_unix_seconds: u64,
}

/// A checkpoint whose independent commit observation, provenance and digest
/// have been checked.
///
/// Private field, no public constructor, no deserialization path. Only
/// [`verify_promotion_economics_checkpoint`] mints one, and decoding an
/// arbitrary checkpoint does not: verification checks the observation's
/// signature, producer and audience, that the observation names the expected
/// key, the expected cell, registry and policy identity, and the checkpoint
/// digest. An unverified checkpoint is never admissible promotion evidence and
/// can never stand in for completed replay.
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedPromotionEconomicsCheckpoint(PromotionEconomicsVerificationCheckpointV1);

impl VerifiedPromotionEconomicsCheckpoint {
    #[must_use]
    pub fn checkpoint(&self) -> &PromotionEconomicsVerificationCheckpointV1 {
        &self.0
    }
}

/// Turns an independently observed claim into verified progress, or refuses.
///
/// This is the seam neither the store nor the caller may cross. The store says
/// what it durably holds; a separate observer re-reads it and signs what it
/// found; this function, owned by the module that owns the private wrapper,
/// decides whether that is progress.
pub fn verify_promotion_economics_checkpoint(
    _verifier: &dyn CellProofVerifier,
    _claim: CommittedPromotionEconomicsCheckpointClaimV1,
    _expectation: &PromotionEconomicsCheckpointExpectationV1,
) -> Result<VerifiedPromotionEconomicsCheckpoint, PromotionEconomicsVerificationErrorV1> {
    Err(PromotionEconomicsVerificationErrorV1::NotImplemented)
}

/// What the store durably holds about who currently holds execution, in the
/// form the store can actually produce.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsCheckpointLeaseClaimV1 {
    pub key: PromotionEconomicsVerificationKeyV1,
    pub expected_revision: u64,
    pub term: u64,
    pub expires_at_unix_seconds: u64,
    pub worker: ProducerId,
    pub lease_digest: Digest32,
}

/// What the owning module independently expects of a lease claim.
///
/// Like [`PromotionEconomicsCheckpointExpectationV1`], this is built by the
/// step rather than supplied by a caller, so every field needs a source among
/// the step's own inputs. Both identity-shaped fields have one, and the sources
/// are different in a way worth stating:
///
/// - `expected_worker` is the step's OWN identity, and it comes from the read
///   authority it already holds:
///   `authority.invocation().envelope.producer`. The worker is whoever invoked
///   this control operation, so the invocation's producer is exactly it. No
///   admission is involved and none should be — a party does not admit itself.
/// - `expected_revision` is the revision the step is resuming from, taken from
///   the retained checkpoint it is continuing, never from the lease claim being
///   checked. An expectation read out of its own subject compares a value
///   against itself.
///
/// The contrast with the checkpoint expectation is the point. There the
/// identity is a THIRD PARTY's, so it must be admitted by policy the caller
/// cannot choose; here it is the caller's own, so the authority the caller
/// already had to present is the correct and sufficient source. Reaching for
/// an admission here would add a knob that admits nothing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsCheckpointLeaseExpectationV1 {
    pub key: PromotionEconomicsVerificationKeyV1,
    pub expected_worker: ProducerId,
    pub expected_revision: u64,
    pub now_unix_seconds: u64,
}

/// An execution lease over one verification key.
///
/// # What holding one does NOT prove
///
/// This is a private-field wrapper, and every other private-field wrapper in
/// this module is minted only after a signature check. This one is not, so read
/// it precisely rather than by analogy:
///
/// - IT IS NOT AN AUTHENTICATION. [`verify_promotion_economics_checkpoint_lease`]
///   takes no [`CellProofVerifier`] and checks no signature. It checks SHAPE AND
///   EXPECTATION only: that the claim names the key, worker and revision being
///   resumed, and that it has not expired at the stated time. A store that
///   fabricated the claim passes that check.
/// - IT IS NOT PROOF OF EXCLUSIVITY. Holding one does not establish that no
///   other worker is executing the same key. Exclusivity is enforced solely by
///   the store's revision compare-and-set at commit, and a lease that lost its
///   race still looks exactly like this.
///
/// Fabricating a lease therefore gains nothing, which is why no signature is
/// required here: progress integrity rests entirely on the signed checkpoint
/// attestation and the revision CAS, and a store dishonest enough to forge a
/// lease is dishonest enough to accept two conflicting commits — which no
/// signature on this type would prevent. The cryptography belongs where the
/// progress is.
///
/// # What it is
///
/// All fields are private and there is no public constructor: a lease is not a
/// credential a caller or a store may assemble. A lease authorizes ONLY writes
/// to its own local verification checkpoint. It is never authority to mutate
/// cell resources or readiness, and the store checks the holder's read
/// authority against the same cell partition rather than accepting ambient
/// credentials.
///
/// Expiry relinquishes execution, not accumulated progress: the next holder
/// resumes from the last committed checkpoint under a new term.
#[derive(Debug, Eq, PartialEq)]
pub struct PromotionEconomicsCheckpointLeaseV1 {
    key: PromotionEconomicsVerificationKeyV1,
    expected_revision: u64,
    term: u64,
    expires_at_unix_seconds: u64,
    worker: ProducerId,
    lease_digest: Digest32,
}

impl PromotionEconomicsCheckpointLeaseV1 {
    #[must_use]
    pub fn key(&self) -> &PromotionEconomicsVerificationKeyV1 {
        &self.key
    }

    #[must_use]
    pub fn expected_revision(&self) -> u64 {
        self.expected_revision
    }

    #[must_use]
    pub fn term(&self) -> u64 {
        self.term
    }

    #[must_use]
    pub fn expires_at_unix_seconds(&self) -> u64 {
        self.expires_at_unix_seconds
    }

    #[must_use]
    pub fn worker(&self) -> &ProducerId {
        &self.worker
    }

    #[must_use]
    pub fn lease_digest(&self) -> Digest32 {
        self.lease_digest
    }
}

/// Turns a reported lease claim into an execution lease, or refuses.
///
/// Deliberately takes no [`CellProofVerifier`]: unlike a checkpoint, a lease
/// carries no accumulated progress and asserts nothing about what was verified.
/// It is local execution scheduling state, so fabricating one gains nothing —
/// a commit still requires a signed checkpoint attestation, and an expired or
/// superseded lease loses its compare-and-set regardless of how it was
/// obtained. What this function checks is that the claim names the key, worker
/// and revision the caller is actually resuming, and that it has not expired at
/// `now_unix_seconds`.
pub fn verify_promotion_economics_checkpoint_lease(
    _claim: PromotionEconomicsCheckpointLeaseClaimV1,
    _expectation: &PromotionEconomicsCheckpointLeaseExpectationV1,
) -> Result<PromotionEconomicsCheckpointLeaseV1, PromotionEconomicsVerificationErrorV1> {
    Err(PromotionEconomicsVerificationErrorV1::NotImplemented)
}

/// One durable checkpoint advance: the execution lease being spent, and the
/// verified next checkpoint.
///
/// Private fields and no public assemble shortcut. Both members are
/// private-field wrappers that only this module's verifiers mint, so a store
/// cannot hand itself a write it did not earn.
#[derive(Debug, Eq, PartialEq)]
pub struct PromotionEconomicsCheckpointWriteV1 {
    lease: PromotionEconomicsCheckpointLeaseV1,
    next: VerifiedPromotionEconomicsCheckpoint,
}

impl PromotionEconomicsCheckpointWriteV1 {
    #[must_use]
    pub fn lease(&self) -> &PromotionEconomicsCheckpointLeaseV1 {
        &self.lease
    }

    #[must_use]
    pub fn next(&self) -> &VerifiedPromotionEconomicsCheckpoint {
        &self.next
    }
}

/// Assembles a durable checkpoint write. Private to this module: the write is
/// the verifier's own input to the store, not a value any caller supplies.
fn assemble_promotion_economics_checkpoint_write(
    _lease: PromotionEconomicsCheckpointLeaseV1,
    _next: VerifiedPromotionEconomicsCheckpoint,
) -> Result<PromotionEconomicsCheckpointWriteV1, PromotionEconomicsVerificationErrorV1> {
    Err(PromotionEconomicsVerificationErrorV1::NotImplemented)
}

/// What `acquire` hands back: the reported lease claim, and the DURABLE RECORD
/// of retained progress if any exists.
///
/// Both are the store's own unverified report and neither is evidence. The
/// retained checkpoint arrives here as a bare record with no signature attached,
/// exactly as `commit` returns one: the store reports, it does not vouch. To
/// resume from that progress the caller takes the same two steps it takes after
/// a commit — ask [`PromotionEconomicsCheckpointCommitObserver`] for an
/// independently observed claim at the same key, then mint the wrapper through
/// [`verify_promotion_economics_checkpoint`].
///
/// The record is still returned here rather than only by the observer so that
/// acquiring the lease and reading the progress it covers stay one atomic
/// store operation, which is what makes the revision compare-and-set meaningful.
pub type PromotionEconomicsCheckpointAcquisitionV1 = (
    PromotionEconomicsCheckpointLeaseClaimV1,
    Option<PromotionEconomicsVerificationCheckpointV1>,
);

/// The partition-local durable checkpoint store.
///
/// THE LAW HAS TWO CLAUSES AND THIS TRAIT OBEYS BOTH.
///
/// (a) ONLY THE OWNING MODULE'S VERIFIER MINTS A PRIVATE-FIELD WRAPPER. Neither
/// method returns one. They hand back
/// [`PromotionEconomicsVerificationCheckpointV1`] and
/// [`PromotionEconomicsCheckpointLeaseClaimV1`] — public records an out-of-crate
/// adapter can actually construct — and the caller mints through
/// [`verify_promotion_economics_checkpoint`] or
/// [`verify_promotion_economics_checkpoint_lease`]. A store that could return a
/// verified wrapper could fabricate progress nothing checked; a store that
/// cannot construct its own return type is simply unimplementable.
///
/// (b) ONLY AN INDEPENDENT PORT, RE-READING BY LOOKUP KEY ALONE, SIGNS AN
/// OBSERVATION OF A WRITE. Neither method returns a signature over its own
/// write. `commit` returns the durable record and nothing more; the signed
/// observation comes from
/// [`PromotionEconomicsCheckpointCommitObserver`], which is handed a key and
/// cannot be told what to find. A store that signs an observation of its own
/// write vouches for itself, and no amount of signature checking downstream
/// recovers what that destroys.
///
/// Both seams have to be in the signature, not only in prose. Satisfying (a)
/// alone still leaves a store attesting to its own work.
///
/// Implementations must provide, atomically: lease-term and revision
/// compare-and-set, exactly one active writer per key, the partition
/// concurrency budget from
/// [`PromotionEconomicsPolicyV1::maximum_concurrent_steps_per_partition`],
/// authenticated retained checkpoint reads, and durable acknowledgement before
/// the caller is told to continue.
///
/// Each of the two refusals that discipline carries has its own variant, so a
/// caller can tell them apart and apply the opposite recoveries they need:
/// `acquire` refuses a key already leased by a live foreign worker with
/// [`crate::PromotionEconomicsVerificationErrorV1::LeaseHeldByAnotherWorker`],
/// and refuses a spent partition budget with
/// [`crate::PromotionEconomicsVerificationErrorV1::PartitionStepBudgetExhausted`].
/// Backing off and retrying is right for the first and wrong for the second. A crash before commit replays the step
/// safely; a crash after commit resumes at the next ordinal without
/// double-counting cost.
pub trait PromotionEconomicsCheckpointStore: Send + Sync {
    fn acquire<'a>(
        &'a self,
        authority: &'a CellControlReadAuthorityV1,
        key: &'a PromotionEconomicsVerificationKeyV1,
        now_unix_seconds: u64,
    ) -> BoxCellFuture<
        'a,
        Result<PromotionEconomicsCheckpointAcquisitionV1, PromotionEconomicsVerificationErrorV1>,
    >;

    /// Durably advances the checkpoint under the lease's revision
    /// compare-and-set, and returns THE DURABLE RECORD ALONE. It never returns a
    /// signature, because a signature here would be the store attesting to its
    /// own write.
    /// Reads the lease currently held for one key WITHOUT acquiring it, so a
    /// worker refused by `acquire` can find out how long to wait.
    ///
    /// `None` means no live lease: re-acquire immediately. `Some` carries the
    /// holder and the expiry that
    /// [`crate::PromotionEconomicsVerificationErrorV1::LeaseHeldByAnotherWorker`]
    /// tells the caller to wait for. Without this read that recovery named a
    /// value only `acquire`'s SUCCESS channel produced, so a caller that had
    /// been refused was told to wait for something it could not obtain.
    ///
    /// READING A LEASE IS NOT A STEP TOWARD HOLDING ONE. The claim comes back
    /// unverified, and it is not the reader's:
    /// [`verify_promotion_economics_checkpoint_lease`] compares
    /// `expected_worker` against the caller's own identity, so a foreign lease
    /// claim cannot be minted into a
    /// [`PromotionEconomicsCheckpointLeaseV1`] by whoever read it. The read
    /// answers "how long", never "may I".
    ///
    /// The expiry is a SNAPSHOT, not a promise. A holder may renew, so a caller
    /// that wakes at the reported time and is refused again reads again rather
    /// than assuming the lease must have lapsed.
    fn read_lease<'a>(
        &'a self,
        authority: &'a CellControlReadAuthorityV1,
        key: &'a PromotionEconomicsVerificationKeyV1,
    ) -> BoxCellFuture<
        'a,
        Result<
            Option<PromotionEconomicsCheckpointLeaseClaimV1>,
            PromotionEconomicsVerificationErrorV1,
        >,
    >;

    fn commit<'a>(
        &'a self,
        write: PromotionEconomicsCheckpointWriteV1,
    ) -> BoxCellFuture<
        'a,
        Result<PromotionEconomicsVerificationCheckpointV1, PromotionEconomicsVerificationErrorV1>,
    >;
}

/// Outcome of one bounded replay step.
///
/// `Continued` is backpressure, not refusal, and carries no economics record.
/// `Complete` is boxed because a completed record embeds a full cell revision
/// identity, the whole policy and the whole closure, which would otherwise make
/// every `Continued` value that large.
///
/// `key` is boxed. Growing the key so it commits the cell revision made
/// `Continued` the large variant against a `Complete` whose payload was already
/// boxed, so both sides are now behind one pointer and neither shape pays for
/// the other.
#[derive(Debug, Eq, PartialEq)]
pub enum PromotionEconomicsVerificationStepOutcomeV1 {
    Continued {
        key: Box<PromotionEconomicsVerificationKeyV1>,
        checkpoint_revision: u64,
    },
    Complete(Box<VerifiedCellPromotionEconomics>),
}

/// The ports one promotion economics replay collaborates with.
///
/// # What this is
///
/// A grouping of the four ports a single replay needs, passed together because
/// they are always passed together. Seven loose arguments that must travel in a
/// fixed relationship were a struct that had not been written yet, and needing
/// an eighth is what made that visible.
///
/// # What this is NOT
///
/// IT IS NOT AN AUTHORITY OBJECT AND HOLDING ONE CONFERS NOTHING. It carries no
/// permission, names no subject, and asserts nothing about the caller. Every
/// question of what may be read and what the answer must be is settled by the
/// per-call arguments it deliberately does NOT contain: the read authority, the
/// already-verified closure, the record under test and the policy. Assembling
/// this struct is not a step in gaining access to anything.
///
/// # Why its fields are public
///
/// None of the four is selection-relevant, so the private-constructor
/// discipline the closure issuer needs does not apply here. A port supplies
/// behaviour; it does not choose the answer. What the replay must reproduce is
/// fixed by `closure` — itself a private-field wrapper only this module mints —
/// and by `policy`, both of which stay per-call. Substituting a dishonest
/// observer changes who vouches, not what is required, and that substitution is
/// caught where it should be: the observation is signed under its own proof
/// domain and checked against an expected producer and audience it cannot
/// forge. Sealing this struct would add ceremony without adding a gate.
///
/// The closure issuer holds its own ports by value from construction instead,
/// because A3 requires its selection-relevant registry and policy to be fixed
/// at admission. That difference is deliberate, not drift: one is a sealed
/// long-lived issuer, the other a per-call collaborator set.
pub struct PromotionEconomicsReplayPortsV1<'a> {
    /// Reads bounded pages of retained records from one admitted source.
    pub retained_input_reader: &'a dyn PromotionEconomicsRetainedInputReader,
    /// Leases execution and durably advances the checkpoint. Reports records;
    /// signs nothing.
    pub checkpoint_store: &'a dyn PromotionEconomicsCheckpointStore,
    /// Independently re-reads a committed checkpoint by key and signs what it
    /// found. Required to resume retained progress at all, since the store
    /// deliberately cannot vouch for its own write.
    pub checkpoint_observer: &'a dyn PromotionEconomicsCheckpointCommitObserver,
    /// Checks the signatures the replay depends on: each admitted source's
    /// finalization, and the observation backing a resumed checkpoint.
    pub proof_verifier: &'a dyn CellProofVerifier,
}

/// Advances promotion economics replay by one bounded step.
///
/// This is the ONLY way a [`VerifiedCellPromotionEconomics`] comes into
/// existence, and it produces one only after every admitted source and every
/// record in the closure has been verified: each source's complete count, root
/// and byte total are recomputed against its authenticated finalization before
/// the stream advances past it. A count or root mismatch after the complete
/// declared stream is a real refusal; a spent step budget is a continuation.
///
/// The closure is passed as an already-verified private-field wrapper rather
/// than as a registry or manifest, so this public entrypoint cannot be used to
/// select a different source population. The policy and the record are both
/// checked against that closure.
///
/// The collaborator set arrives as [`PromotionEconomicsReplayPortsV1`]. Two of
/// its four members were absent from this signature before: the checkpoint
/// observer, without which a resumed checkpoint cannot be turned into verified
/// progress at all, and the proof verifier, without which "recomputed against
/// its authenticated finalization" above could not be performed — this function
/// asserted it authenticated signatures while holding nothing able to check
/// one. Both are collaborators of this replay, so both belong in the set.
///
/// A promotion proof that expires while verification runs may be reissued over
/// the same immutable calculation and closure once replay completes, provided
/// the cell revision, policy and retention still hold: the completed replay is
/// not tied to an expiring outer signature. The promotion verifier still checks
/// its own current signature, expiry and exact economics tuple regardless.
pub fn advance_cell_promotion_economics<'a>(
    _ports: &'a PromotionEconomicsReplayPortsV1<'a>,
    _authority: &'a CellControlReadAuthorityV1,
    _closure: &'a VerifiedPromotionEconomicsClosure,
    _evidence: &'a crate::CellPromotionEconomicsV1,
    _policy: &'a PromotionEconomicsPolicyV1,
    _now_unix_seconds: u64,
) -> BoxCellFuture<
    'a,
    Result<PromotionEconomicsVerificationStepOutcomeV1, PromotionEconomicsVerificationErrorV1>,
> {
    Box::pin(async { Err(PromotionEconomicsVerificationErrorV1::NotImplemented) })
}
