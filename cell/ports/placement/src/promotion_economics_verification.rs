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
    BoxCellFuture, CellControlReadAuthorityV1, CellId, CellRevisionIdentityV1, Digest32,
    PlacementPartitionV1, ProducerId, PromotionCostCategoryTotalV1,
    PromotionEconomicsInputMemberV1, PromotionEconomicsPolicyV1,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PromotionEconomicsVerificationKeyV1 {
    SourceClosure {
        partition: PlacementPartitionV1,
        cell_id: CellId,
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

/// A checkpoint restored under verified store attestation and provenance.
///
/// Private field, no public constructor. Decoding an arbitrary checkpoint does
/// not mint one: restoration verifies the store's attestation and the
/// checkpoint digest first. An unverified checkpoint is never admissible
/// promotion evidence and can never stand in for completed replay.
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedPromotionEconomicsCheckpoint(PromotionEconomicsVerificationCheckpointV1);

impl VerifiedPromotionEconomicsCheckpoint {
    #[must_use]
    pub fn checkpoint(&self) -> &PromotionEconomicsVerificationCheckpointV1 {
        &self.0
    }
}

/// An execution lease over one verification key.
///
/// All fields are private and there is no public constructor: a lease is not a
/// credential a caller may assemble. A lease authorizes ONLY writes to its own
/// local verification checkpoint. It is never authority to mutate cell
/// resources or readiness, and the store checks the holder's read authority
/// against the same cell partition rather than accepting ambient credentials.
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

/// One durable checkpoint advance: an expected lease and revision, plus the
/// verified next checkpoint.
///
/// Private fields and no public assemble shortcut, so a caller cannot hand the
/// store a checkpoint it did not earn.
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

/// Mints an execution lease. Private to this module by design: lease
/// construction belongs to the cell verifier, not to a store implementation or
/// an external caller.
fn mint_promotion_economics_checkpoint_lease(
    _key: PromotionEconomicsVerificationKeyV1,
    _expected_revision: u64,
    _term: u64,
    _expires_at_unix_seconds: u64,
    _worker: ProducerId,
) -> Result<PromotionEconomicsCheckpointLeaseV1, PromotionEconomicsVerificationErrorV1> {
    Err(PromotionEconomicsVerificationErrorV1::NotImplemented)
}

/// Mints a durable checkpoint write. Private for the same reason as the lease.
fn mint_promotion_economics_checkpoint_write(
    _lease: PromotionEconomicsCheckpointLeaseV1,
    _next: VerifiedPromotionEconomicsCheckpoint,
) -> Result<PromotionEconomicsCheckpointWriteV1, PromotionEconomicsVerificationErrorV1> {
    Err(PromotionEconomicsVerificationErrorV1::NotImplemented)
}

/// What `acquire` hands back: the execution lease, and the retained progress if
/// any exists.
pub type PromotionEconomicsCheckpointAcquisitionV1 = (
    PromotionEconomicsCheckpointLeaseV1,
    Option<VerifiedPromotionEconomicsCheckpoint>,
);

/// The partition-local durable checkpoint store.
///
/// Implementations must provide, atomically: lease-term and revision
/// compare-and-set, exactly one active writer per key, the partition
/// concurrency budget from
/// [`PromotionEconomicsPolicyV1::maximum_concurrent_steps_per_partition`],
/// authenticated retained checkpoint reads, and durable acknowledgement before
/// the caller is told to continue. A crash before commit replays the step
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

    fn commit<'a>(
        &'a self,
        write: PromotionEconomicsCheckpointWriteV1,
    ) -> BoxCellFuture<
        'a,
        Result<VerifiedPromotionEconomicsCheckpoint, PromotionEconomicsVerificationErrorV1>,
    >;
}

/// Outcome of one bounded replay step.
///
/// `Continued` is backpressure, not refusal, and carries no economics record.
/// `Complete` is boxed because a completed record embeds a full cell revision
/// identity, the whole policy and the whole closure, which would otherwise make
/// every `Continued` value that large.
#[derive(Debug, Eq, PartialEq)]
pub enum PromotionEconomicsVerificationStepOutcomeV1 {
    Continued {
        key: PromotionEconomicsVerificationKeyV1,
        checkpoint_revision: u64,
    },
    Complete(Box<VerifiedCellPromotionEconomics>),
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
/// A promotion proof that expires while verification runs may be reissued over
/// the same immutable calculation and closure once replay completes, provided
/// the cell revision, policy and retention still hold: the completed replay is
/// not tied to an expiring outer signature. The promotion verifier still checks
/// its own current signature, expiry and exact economics tuple regardless.
pub fn advance_cell_promotion_economics<'a>(
    _reader: &'a dyn PromotionEconomicsRetainedInputReader,
    _store: &'a dyn PromotionEconomicsCheckpointStore,
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
