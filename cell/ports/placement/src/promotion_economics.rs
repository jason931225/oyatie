//! Reproducible total-cell promotion economics: the record, its policy, its
//! taxonomy and its retained calculation inputs.
//!
//! Promotion currently admits a unit-cost scalar plus an opaque evidence
//! reference. A scalar cannot be replayed: its cost scope is unnamed, so
//! excluding reserve, idle, network or control cost lowers it without changing
//! any declared field, and its denominator is not stated at all. The contract
//! here replaces that scalar with a record whose total, denominator and unit
//! cost are all recomputable from retained typed inputs under one exactly
//! identified policy and one exact cell revision.
//!
//! Marginal placement economics stay where they are.
//! [`crate::CommercialPlacementBasisV1`] expresses what one additional
//! placement costs; total cell operating cost includes idle capacity and
//! shared overhead that no marginal basis carries. The two are not
//! interchangeable and neither is derived from the other.
//!
//! Everything in this module is a declaration. Every constructor, verifier and
//! reader fails closed with a typed `NotImplemented`; no pricing, hashing,
//! paging, authorization or arithmetic runtime exists yet.

use crate::{
    CapacityAmountV1, CapacityDimensionV1, CellRevisionIdentityV1, CommercialSourceRecordRefV1,
    CurrencyCode, Digest32, ImmutableEvidenceRefV1, KeyId, MoneyMicrounitsV1, ProducerId,
    ReservationRefV1,
};

/// Half-open observation window `[start, end)`.
///
/// `start_unix_seconds < end_unix_seconds` is required; an empty or inverted
/// window is `WindowMismatch`. Every retained input window lies inside the
/// closure window, and no sampled or partial window is ever silently admitted
/// in place of the whole one.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PromotionEconomicsWindowV1 {
    pub start_unix_seconds: u64,
    pub end_unix_seconds: u64,
}

/// The cost scope a promotion economics record claims to measure.
///
/// One variant, deliberately: naming the scope is the whole point. A cell's
/// total operating cost includes home and idle capacity, recovery reserves,
/// network and movement, control and observability, security and compliance,
/// and allocated shared overhead. Adding a narrower scope variant later would
/// reopen exactly the omission this record exists to close, and requires its
/// own admitted taxonomy and independent review.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionCostScopeV1 {
    TotalCellOperatingCost,
}

/// The admitted taxonomy of cell operating expense categories.
///
/// These are categories of ALL cell operating expense, not a filter that lets
/// unrecognized expense vanish. Every cost record in every admitted source
/// window is classified exactly once, either into one of these categories or
/// explicitly as [`PromotionCostClassificationV1::OutsideCellOperatingScope`]
/// with a native scope reason. A future expense that fits none of these
/// requires a new taxonomy version and its classifiers in the same change;
/// there is no fallback bucket and no silent drop.
///
/// Declaration order is wire identity only and carries no rank.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionCostCategoryV1 {
    /// Home capacity AND the idle, reserved and headroom capacity that stands
    /// behind it. Stranded headroom is a cell operating cost, not an exclusion.
    HomeAndIdleCapacity,
    /// Backup storage, warm recovery reserves and restore exercises, including
    /// allocated shared recovery cost.
    RecoveryAndBackup,
    /// Network, egress and capacity movement.
    NetworkAndMovement,
    /// Control plane and observability.
    ControlAndObservability,
    /// Security and compliance.
    SecurityAndCompliance,
    /// Allocated shared operating overhead.
    SharedOperatingOverhead,
}

/// Whether a retained source row increases or decreases measured cost.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionCostSignV1 {
    Charge,
    Credit,
}

/// How a source charge is valued.
///
/// `AmortizedEffectiveCost` is the source owner's finalized effective amount
/// for precisely the retained window, with actual spot and commitment cost
/// already incurred and included. It is never a forecast, a list quote, a
/// risk premium, or a caller-chosen marginal amount. Supporting any other
/// valuation requires a separately admitted variant carrying its own retained
/// operands.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionCostValuationV1 {
    AmortizedEffectiveCost,
}

/// Rounding of the final normalized unit cost.
///
/// Ceiling at microunits, so normalization can never round a positive unit
/// cost down to zero.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionUnitCostRoundingV1 {
    CeilingMicrounit,
}

/// Rounding of per-source cell allocation, chosen so rounding can only
/// overstate cost, never understate it.
///
/// A charge attribution rounds UP (`ceil(amount * n / d)`) and a credit
/// attribution rounds DOWN (`floor(amount * n / d)`). Symmetric ceiling would
/// round credits upward and make measured cost look lower than it is. One
/// microunit of charge allocated one half contributes one; one microunit of
/// credit allocated one half contributes zero. Policy identity commits this
/// rule, so a record cannot silently switch conventions.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionCostAllocationRoundingV1 {
    ChargeCeilingCreditFloor,
}

/// The usage basis that forms the denominator.
///
/// Committed home capacity time only. Warm, tentative and idle reservations
/// are not denominators, and neither are tenant, reservation or movement
/// population counts: a cell that adds tenants without adding committed
/// capacity has not become cheaper per unit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionUsageBasisV1 {
    CommittedHomeCapacitySeconds,
}

/// The normalized unit the denominator is expressed in.
///
/// Both scalars must be strictly positive and the dimension must be one this
/// policy admits. `CpuMillis` with 1000 capacity units per normalized unit and
/// 3600 seconds per normalized unit means allocated vCPU-hours.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PromotionNormalizedUnitV1 {
    pub dimension: CapacityDimensionV1,
    pub capacity_units_per_normalized_unit: u64,
    pub seconds_per_normalized_unit: u64,
}

/// The denominator, retained in raw capacity-unit-seconds.
///
/// The exact normalized quantity is
/// `raw_capacity_unit_seconds / (capacity_units_per_normalized_unit *
/// seconds_per_normalized_unit)`. It is retained raw precisely so the division
/// happens once, inside the unit-cost formula, and never as a premature
/// rounding step: a fractional normalized quantity is valid and must never be
/// rounded to zero. `raw_capacity_unit_seconds` is the summed committed-home
/// interval product in the selected dimension; it is never a reservation count
/// and never rated rather than committed capacity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PromotionNormalizedQuantityV1 {
    pub unit: PromotionNormalizedUnitV1,
    pub raw_capacity_unit_seconds: u64,
}

/// The one admitted calculation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionEconomicsAlgorithmV1 {
    TotalEffectiveCostPerCommittedCapacityTime,
}

/// Immutable identity of the calculation that produced a record.
///
/// `implementation_digest` and `canonical_encoding_version` are compared for
/// exact equality against the expectation. A record whose scalars match but
/// whose calculation identity differs is `CalculationMismatch`, not a match:
/// the same numbers produced by a different algorithm are not the same
/// evidence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PromotionEconomicsCalculationIdentityV1 {
    pub schema_version: u32,
    pub algorithm: PromotionEconomicsAlgorithmV1,
    pub implementation_digest: Digest32,
    pub canonical_encoding_version: u32,
}

/// Version and definition digest of the admitted cost taxonomy.
///
/// The same taxonomy identity is carried by the policy, by the closure, and by
/// every admitted source: a source's `cost_taxonomy_digest` must equal this
/// `definition_digest`. That is what makes "every expense is classified
/// exactly once" checkable across sources rather than per source.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PromotionCostTaxonomyV1 {
    pub version: u32,
    pub definition_digest: Digest32,
}

/// The approved economics policy.
///
/// Every field is compared for exact equality against the expectation before
/// any input is read. The policy is supplied by the cell's own policy loader
/// from trusted state; it is never taken from the submitted promotion proof,
/// because a promoter that chooses its own policy chooses its own answer.
///
/// The `maximum_*_per_page`, `maximum_*_per_step`,
/// `maximum_concurrent_steps_per_partition` and `checkpoint_lease_seconds`
/// fields are SCHEDULING and BACKPRESSURE limits, not admission limits.
/// Reaching them yields
/// [`crate::PromotionEconomicsVerificationStepOutcomeV1::Continued`], never
/// `IncompleteInputSet` and never a promotion refusal. A large estate is
/// slower to verify, not permanently ineligible for promotion. Total input
/// counts recorded elsewhere are completeness commitments, not size caps.
///
/// NO DELIBERATE CEILING ON ESTATE SIZE EXISTS, but two refusals are
/// size-related and the distinction between them matters:
/// `ArithmeticOverflow` is a representability failure — a value that will not
/// fit in its output type — and cannot be reached by a well-formed estate of
/// any size that produces representable totals.
/// `RetentionInsufficient` is size-SENSITIVE and CAN deny a replay that would
/// otherwise have succeeded, because work remaining scales with the estate
/// while a retention deadline does not. It is a liveness bound rather than a
/// ceiling — it refuses because the evidence will cease to exist, not because
/// a number was judged too large — and its remedy is longer source retention
/// or more throughput, never a smaller estate. An earlier draft of this doc
/// claimed `ArithmeticOverflow` was the only size-shaped refusal; that was too
/// strong, and the correction is recorded here rather than left to be
/// rediscovered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsPolicyV1 {
    pub policy_generation: u64,
    pub policy_version: String,
    pub policy_digest: Digest32,
    pub cost_scope: PromotionCostScopeV1,
    /// Every monetary input and output must be in exactly this currency. There
    /// is no conversion step: mixed currency is `CurrencyMismatch`.
    pub currency: CurrencyCode,
    pub valuation: PromotionCostValuationV1,
    pub usage_basis: PromotionUsageBasisV1,
    pub unit: PromotionNormalizedUnitV1,
    pub rounding: PromotionUnitCostRoundingV1,
    pub allocation_rounding: PromotionCostAllocationRoundingV1,
    pub taxonomy: PromotionCostTaxonomyV1,
    pub calculation: PromotionEconomicsCalculationIdentityV1,
    /// Compared against the computed unit cost in identical currency and unit,
    /// after the calculation completes and never as a substitute for it.
    pub maximum_unit_cost: MoneyMicrounitsV1,
    pub maximum_rows_per_page: u32,
    pub maximum_bytes_per_page: u64,
    pub maximum_pages_per_step: u32,
    pub maximum_bytes_per_step: u64,
    pub maximum_concurrent_steps_per_partition: u32,
    pub checkpoint_lease_seconds: u64,
    pub minimum_retention_seconds: u64,
    /// The signing identity the checkpoint commit observer must present.
    ///
    /// It lives on the POLICY, not on the replay ports bundle, and the
    /// difference is the whole safety property. See
    /// [`PromotionEconomicsCheckpointObserverAdmissionV1`].
    pub checkpoint_observer: PromotionEconomicsCheckpointObserverAdmissionV1,
}

/// The cell's admission of the checkpoint commit observer's signing identity.
///
/// # Why this is on the policy and not on the ports bundle
///
/// The module's whole answer to "a dishonest observer is caught" is that its
/// observation is signed under its own proof domain and checked against an
/// expected producer and audience it cannot forge. That argument only holds if
/// the EXPECTED identity comes from somewhere the caller cannot choose.
///
/// A member of [`crate::PromotionEconomicsReplayPortsV1`] would not be such a
/// place. The bundle is assembled per call, so a caller supplying a dishonest
/// observer would supply its matching identity in the same breath and the check
/// would pass — the A3 steering hazard in a new coat, verifying a signature
/// against whatever the signer nominated.
///
/// The policy is different in kind, and the chain is worth stating because a
/// reader who sees only "identity lives in policy" will read it as another
/// per-call plain argument:
///
/// 1. `advance_cell_promotion_economics` compares its `policy` argument against
///    `closure.policy_digest`.
/// 2. `closure` is a [`crate::VerifiedPromotionEconomicsClosure`] — a
///    private-field wrapper only the closure issuer mints.
/// 3. The issuer resolves its policy from native cell policy at admission, not
///    from anything a caller passes.
///
/// So substituting an observer identity changes `policy_digest`, which
/// mismatches a closure the caller cannot forge, and the substitution fails
/// before any observation is examined. The identity is reachable and it is not
/// caller-chosen.
///
/// This mirrors how the module already admits SOURCE identities:
/// [`crate::PromotionEconomicsSourceAdmissionV1`] carries a producer and an
/// `expected_signing_key_id` per admitted scope. Sources are admitted per scope
/// through the registry; the checkpoint observer is one port per cell, so it is
/// admitted once, here.
///
/// # Operational coupling, stated rather than discovered
///
/// Because the identity is committed by the policy digest, ROTATING THE
/// OBSERVER'S SIGNING KEY REQUIRES A NEW POLICY GENERATION. That is the same
/// coupling admitted sources already have through `admission_generation`, and
/// it is the price of the property above: an identity that can be changed
/// without changing the policy identity is an identity a caller can change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsCheckpointObserverAdmissionV1 {
    pub producer: ProducerId,
    pub audience: ProducerId,
    pub expected_signing_key_id: KeyId,
    pub admission_digest: Digest32,
}

/// Per-category totals over the retained population.
///
/// Exactly one total is present for each of the six
/// [`PromotionCostCategoryV1`] variants, in declaration order, including
/// categories with zero inputs and zero charge. An omitted category is
/// `CostScopeIncomplete`: silence is how idle capacity or shared overhead
/// disappears from a total, so an explicit zero is required to say a category
/// really is empty.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionCostCategoryTotalV1 {
    pub category: PromotionCostCategoryV1,
    pub charge_microunits: u64,
    pub credit_microunits: u64,
    pub input_count: u64,
}

/// Why a retained cost row contributes nothing to this cell's operating cost.
///
/// The reason is bounded and native-owner-defined. It never means "the
/// classifier did not recognize this category": an unrecognized category is
/// `UnsupportedCalculation` against the taxonomy, and a current cell operating
/// expense marked outside scope is a classification failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionOutsideCostScopeReasonV1 {
    /// The charge belongs to a different cell, provable from the retained
    /// source-native cell identity on the row.
    DifferentCell,
    /// The charge is not an operating expense of any cell.
    NonOperatingBusinessExpense,
}

/// Exhaustive classification of one retained cost row.
///
/// Every row in every admitted source window carries exactly one of these.
/// `OutsideCellOperatingScope` rows stay retained, stay counted in source
/// completeness, and contribute zero with cell allocation zero; they carry
/// enough source-native scope and cell identity to verify the reason. That is
/// what stops an unwanted charge from being dropped rather than excluded on
/// the record.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionCostClassificationV1 {
    Included(PromotionCostCategoryV1),
    OutsideCellOperatingScope(PromotionOutsideCostScopeReasonV1),
}

/// One retained cost operand.
///
/// The row embeds every formula operand together with the immutable native
/// source identity and digest it came from. A digest-only placeholder cannot
/// stand in for a row, and there is no hidden operand: the canonical retained
/// source record is exactly these fields plus source identity, with the
/// immutable digest excluding its own content digest.
///
/// `effective_charge` is the source owner's finalized amortized amount for
/// precisely this window, BEFORE cell allocation. Upstream amortization
/// correctness belongs to the native accounting authority; this record
/// reproduces cell allocation and unit cost, not invoice generation.
///
/// Allocation requires `cell_allocation_denominator > 0` and
/// `cell_allocation_numerator <= cell_allocation_denominator`. A zero
/// numerator is permitted only for an explicitly retained zero-attribution
/// source. Immutable row identity forbids splitting one source row across
/// several attributions to game rounding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionRetainedCostInputV1 {
    pub source_record: CommercialSourceRecordRefV1,
    pub classification: PromotionCostClassificationV1,
    pub window: PromotionEconomicsWindowV1,
    pub sign: PromotionCostSignV1,
    pub effective_charge: MoneyMicrounitsV1,
    pub cell_allocation_numerator: u64,
    pub cell_allocation_denominator: u64,
}

/// One retained committed-home usage interval.
///
/// The canonical source record is this finalized interval plus reservation
/// identity, with the digest excluding itself. Only committed home
/// reservations count: a warm, tentative or idle reservation source, a wrong
/// or absent capacity dimension, an interval overlapping another interval of
/// the same reservation and dimension, or a forged source binding all refuse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionRetainedUsageInputV1 {
    pub source_record: ImmutableEvidenceRefV1,
    pub reservation: ReservationRefV1,
    pub window: PromotionEconomicsWindowV1,
    pub committed_capacity: CapacityAmountV1,
}

/// A retained calculation operand: either a cost row or a usage interval.
///
/// Both payloads are boxed. A usage row carries a whole reservation reference
/// and a cost row a whole commercial source reference, so an unboxed variant
/// would make every member of a streamed page as large as the largest operand
/// shape. Boxing both rather than only the larger one keeps the two operand
/// kinds symmetric, which is what the wire `oneof` expresses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PromotionEconomicsInputV1 {
    Cost(Box<PromotionRetainedCostInputV1>),
    Usage(Box<PromotionRetainedUsageInputV1>),
}

/// One ordinal-addressed member of the retained input stream.
///
/// Canonical order is all cost rows before all usage rows; cost rows by source
/// authority, repository, object and version then category; usage rows by
/// reservation cell, id and term then interval start, ties broken by source
/// identity. Ordinals are dense and gapless, so a duplicate, a reorder, an
/// omission and an unexpected row are each detectable while streaming.
/// `member_digest` is recomputed from the canonical encoding rather than
/// trusted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsInputMemberV1 {
    pub ordinal: u64,
    pub input: PromotionEconomicsInputV1,
    pub member_digest: Digest32,
}

/// The reproducible promotion economics record.
///
/// `total_cost`, `denominator` and `unit_cost` are all recomputable from the
/// retained inputs the `closure` commits, under exactly `policy` and exactly
/// `cell`. The record is admissible only when that replay reproduces all three.
///
/// Arithmetic is checked `u128` at every multiplication, with any result not
/// representable in the output `u64` refusing as `ArithmeticOverflow`:
///
/// - attributed charge = `ceil(effective_charge * n / d)`;
/// - attributed credit = `floor(effective_charge * n / d)`;
/// - `total_cost` = summed charges minus summed credits, refusing
///   `NegativeTotalCost` below zero, with category totals equal to the summed
///   attributed rows;
/// - `denominator.raw_capacity_unit_seconds` = summed
///   `committed_capacity.units * interval_seconds` over committed-home
///   reservations in the policy dimension;
/// - `unit_cost` = `ceil(total_cost * capacity_units_per_normalized_unit *
///   seconds_per_normalized_unit / raw_capacity_unit_seconds)`.
///
/// A zero total with a positive denominator is a valid answer. A zero
/// denominator always refuses as `ZeroDenominator`; it is never treated as an
/// unconstrained or infinite unit cost.
///
/// `calculation_digest` binds the schema and calculation identity, the entire
/// policy, the exact cell revision and window, the finalized closure, the
/// total, the exact denominator with its normalized unit, and the output cost.
/// A matching scalar over a different input closure or policy therefore fails.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellPromotionEconomicsV1 {
    pub cell: CellRevisionIdentityV1,
    pub window: PromotionEconomicsWindowV1,
    pub policy: PromotionEconomicsPolicyV1,
    pub closure: crate::PromotionEconomicsClosureV1,
    pub total_cost: MoneyMicrounitsV1,
    pub denominator: PromotionNormalizedQuantityV1,
    pub unit_cost: MoneyMicrounitsV1,
    pub calculation_digest: Digest32,
}

/// A promotion economics record whose complete replay has finished.
///
/// The field is private and there is no public constructor, no assemble
/// shortcut and no deserialization path: the only way to obtain one is
/// [`crate::advance_cell_promotion_economics`] returning
/// [`crate::PromotionEconomicsVerificationStepOutcomeV1::Complete`] after
/// every admitted source and every record has been verified. A `Continued`
/// step, an unverified checkpoint and a decoded record are all inadmissible.
///
/// Holding one is not authority to mutate anything. It is not transferable to
/// another promotion proof, and the promotion verifier still checks its own
/// signature, expiry, readiness and expected revision independently.
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCellPromotionEconomics(CellPromotionEconomicsV1);

impl VerifiedCellPromotionEconomics {
    #[must_use]
    pub fn evidence(&self) -> &CellPromotionEconomicsV1 {
        &self.0
    }
}

/// Every way promotion economics verification refuses.
///
/// The variants are deliberately precise rather than collapsed into one
/// rejection, so an operator reading a refusal learns which invariant broke
/// without consulting an index.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionEconomicsVerificationErrorV1 {
    NotImplemented,
    /// A retained input the closure said exists could not be read: the reader
    /// reached its exact source, snapshot and ordinal and found nothing there.
    ///
    /// This is READER-LEVEL ABSENCE of a specific record, and it is the
    /// narrowest of the four refusals in this enum that mention something
    /// missing. The other three are about sets, not records:
    /// `IncompleteInputSet` is the declared population failing its own
    /// accounting — a count, root or byte total that does not reconcile after
    /// the complete stream, or a page that ended early.
    /// `CostScopeIncomplete` is the TAXONOMY not being covered — a category
    /// total absent, or an admitted scope the total scope requires not present
    /// in the population at all.
    /// `SourceClosureRejected` is CLOSURE-TIME, the authority refusing to bind
    /// a population that is missing an admitted scope.
    /// So: a scope absent from the registry refuses at closure time; a scope
    /// absent from the taxonomy coverage is `CostScopeIncomplete`; a population
    /// that does not add up is `IncompleteInputSet`; and a single record whose
    /// bytes are not where the manifest says they are is this one.
    ///
    /// Distinct from `RetentionInsufficient`, which is the same absence
    /// FORESEEN rather than encountered: retention refuses before reading
    /// because the bytes will be gone, this refuses after reading because they
    /// already are.
    MissingInput,
    NotAuthorized,
    DependencyUnavailable,
    UnsupportedCalculation,
    PolicyMismatch,
    CellMismatch,
    /// A window relation failed at REPLAY time: a retained input's window is
    /// not inside the closure window, or the record's window disagrees with the
    /// closure or policy, or a window is malformed — `start >= end`, or
    /// `end` later than now, or a finalization claiming to close a window it
    /// does not cover.
    ///
    /// BOUNDARY against `SourceClosureRejected`, which also names window
    /// disagreement: that is CLOSURE-TIME and is about the admitted POPULATION
    /// — the source scopes do not agree on a common window, so no closure is
    /// bound. This one is about a SINGLE record or finalization measured
    /// against a closure that already exists. Same relation, two different
    /// moments, and only one of them has a closure to compare against.
    WindowMismatch,
    /// The declared population was not fully verified: a missing source, a
    /// missing record, an early terminal page or a count, root or byte
    /// mismatch after the complete stream.
    IncompleteInputSet,
    DuplicateInput,
    InputDigestMismatch,
    NonCanonicalInput,
    /// A category total is absent, or an admitted source scope required for the
    /// total scope is missing.
    CostScopeIncomplete,
    CurrencyMismatch,
    InvalidAllocation,
    ZeroDenominator,
    ArithmeticOverflow,
    NegativeTotalCost,
    /// The retained evidence this replay depends on will not survive long
    /// enough to finish verifying it.
    ///
    /// The closure records the MINIMUM authenticated source retention deadline
    /// and the checkpoint carries it forward, so a step can tell before reading
    /// anything whether the bytes it still needs will outlive the work still to
    /// do, with `minimum_retention_seconds` as the required margin.
    ///
    /// RECOVERY: TERMINAL FOR THIS VERIFICATION KEY, and it is the one refusal
    /// in this enum where THE PASSAGE OF TIME ITSELF is the harm.
    ///
    /// The distinction against the contention refusals is not that they are
    /// free to retry — `PartitionStepBudgetExhausted` says plainly that
    /// retrying adds load to the thing that is saturated. It is that for
    /// `LeaseHeldByAnotherWorker`, `PartitionStepBudgetExhausted` and
    /// `CheckpointConflict`, WAITING IS THE REMEDY: the condition clears while
    /// you wait, and only eager retrying is costly, at the system's expense
    /// rather than your own chance of success. Here waiting IS the cost. The
    /// margin that was already short is consumed by the wait, so a caller that
    /// backs off and returns has strictly less chance than one that acted
    /// immediately, and one that backs off long enough has none.
    ///
    /// It is NOT terminal for promotion. The forward path is to re-finalize the
    /// affected sources with renewed retention and issue a NEW closure, which
    /// has a new closure digest and therefore a new verification key, so replay
    /// starts fresh rather than inheriting accumulators bound to evidence that
    /// is expiring. Renewing retention on an old closure is not a path: the
    /// closure committed the deadline it was bound with.
    ///
    /// BOUNDARY against `SourceClosureRejected`, which names retention too —
    /// this is the overlap that would otherwise be resolved arbitrarily.
    /// `SourceClosureRejected` is CLOSURE-TIME: the authority declines to bind
    /// at all because the admitted population's common retention cannot meet
    /// the requirement, so no verification ever starts.
    /// `RetentionInsufficient` is REPLAY-TIME: a closure WAS bound with an
    /// adequate margin and that margin has since eroded, or a step reached a
    /// source whose own deadline is nearer than the closure's minimum. One
    /// refuses to begin; the other stops something already under way.
    ///
    /// BOUNDARY against `WorkLimitExceeded`: that is a reader breaking a bound
    /// it was given. This is nothing breaking any bound — every party behaved
    /// correctly and the evidence is simply expiring.
    ///
    /// ON WHETHER THIS IS A SIZE CEILING — it is size-SENSITIVE, and the
    /// policy's claim about size-shaped refusals is qualified accordingly; see
    /// [`PromotionEconomicsPolicyV1`]. Work remaining scales with the estate
    /// while the retention deadline does not, so a large enough estate against
    /// a short enough retention cannot finish, and this refusal CAN deny a
    /// replay that would otherwise have succeeded. It is still not the defect
    /// that `maximum_total_rows` was: that refused a large estate while all its
    /// evidence was present and the replay could have completed, so the refusal
    /// added nothing and had no remedy but a smaller estate. This one refuses
    /// because the bytes will be gone, which no relaxation can fix — removing
    /// the check would admit an unverifiable replay, not a large one — and its
    /// remedy is longer source retention or more throughput, both properties of
    /// the deployment rather than of the estate.
    RetentionInsufficient,
    /// A per-page or per-request bound was violated by the reader, or a zero
    /// limit was configured. Distinct from a `Continued` step outcome: this is a
    /// contract violation, not budget exhaustion. Budget exhaustion is never an
    /// error at all — it is reported on the OK channel with its progress token.
    WorkLimitExceeded,
    CalculationMismatch,
    UnitCostThresholdExceeded,
    /// A CLOSURE-LEVEL refusal: the closure authority declined to bind a
    /// closure at all, because the admitted source population disagreed on
    /// coverage, registry identity, taxonomy, window or common retention.
    ///
    /// Boundary against the per-source errors: this is about the SET of
    /// sources — a missing or additional or duplicated admitted scope,
    /// overlapping canonical charge ownership between owners, a registry that
    /// is not the one cell policy selects, a taxonomy digest that disagrees
    /// with the policy taxonomy, or a retention deadline no source can meet.
    /// A single source's own finalization failing its signature, producer,
    /// key, domain, audience, admission or schema check is that source's
    /// failure and is reported by
    /// [`crate::verify_promotion_economics_source_finalization`], not here.
    /// Two errors with overlapping and unstated boundaries get chosen
    /// arbitrarily, so the split is stated rather than left to a future
    /// implementer.
    SourceClosureRejected,
    /// A LIVE LEASE FOR THIS KEY IS HELD BY ANOTHER WORKER, so `acquire`
    /// declined to issue one.
    ///
    /// RECOVERY: read the current lease with
    /// [`crate::PromotionEconomicsCheckpointStore::read_lease`], back off until
    /// the expiry it reports, then re-acquire. This is not a failure and not a
    /// wedge — the work is progressing under someone else, and the right
    /// response is to wait for them rather than to duplicate them.
    ///
    /// The variant carries no payload ON PURPOSE, and the doc names the read
    /// instead. An error that embedded an expiry would be handing out a
    /// snapshot that a renewal can invalidate before the caller acts on it; the
    /// read gives current truth each time it is asked. What is NOT acceptable
    /// is what this doc used to do — name a value and provide neither, so the
    /// caller was told to wait for something only `acquire`'s success channel
    /// ever produced.
    ///
    /// BOUNDARY against `CheckpointConflict`: that one is AFTER THE FACT — you
    /// held a lease and lost the compare-and-set. This one is BEFORE — you
    /// never got a lease at all. BOUNDARY against
    /// `PartitionStepBudgetExhausted`: this is per-KEY contention with a named
    /// holder; that is per-PARTITION saturation with no holder to wait on.
    LeaseHeldByAnotherWorker,
    /// The partition's `maximum_concurrent_steps_per_partition` budget is
    /// spent, so `acquire` declined to start another step here.
    ///
    /// RECOVERY: reschedule under partition-level backoff. Do not spin on this
    /// key, and do not hold the caller waiting on a specific moment — retrying
    /// adds load to precisely the thing that is saturated.
    ///
    /// THIS VARIANT DELIBERATELY NAMES NO OBSERVABLE, unlike its neighbour
    /// `LeaseHeldByAnotherWorker`, which names one and now has
    /// [`crate::PromotionEconomicsCheckpointStore::read_lease`] to supply it.
    /// The asymmetry is real rather than an omission. A lease has ONE holder
    /// and ONE expiry, so a read returns a concrete "wait until T" that makes
    /// the recovery precise. Partition occupancy has neither: it is a
    /// continuously changing count with no holder to wait on, any read of it is
    /// stale the moment it returns, and there is no moment a caller could
    /// correctly sleep until. An occupancy read would look like the lease read
    /// and answer nothing, which is worse than not offering it.
    ///
    /// So this recovery prescribes only what a caller can actually perform. An
    /// earlier wording said "do not retry until a step completes elsewhere",
    /// which named an event no caller can observe — the same defect as telling
    /// a refused worker to wait for an expiry it had no way to read.
    ///
    /// BOUNDARY against `WorkLimitExceeded`: that is a READER CONTRACT
    /// VIOLATION on a single page — a bound the reader was told and broke.
    /// This is the store's own scheduling budget behaving correctly. Neither
    /// means the estate is too large; no refusal in this enum does.
    PartitionStepBudgetExhausted,
    /// STORE-SIDE CONTENTION ONLY: an execution lease expired, or a commit lost
    /// its revision compare-and-set to a writer that got there first.
    ///
    /// RECOVERY: retry from persisted progress under a fresh acquire. Nothing
    /// is discarded — a lost race means another worker advanced the same key,
    /// not that the accumulated work is wrong. It shares "resume, do not
    /// restart" with `LeaseHeldByAnotherWorker`; what is distinctive is that
    /// this is the only one reached AFTER work began.
    ///
    /// That recovery is why this variant must not also carry identity
    /// mismatches. A changed cell, closure, registry, policy or calculation
    /// needs the OPPOSITE response — the accumulators must not be inherited at
    /// all — and one variant cannot direct both "resume from what you have" and
    /// "discard what you have".
    ///
    /// It does NOT mean the lease was refused up front — that is
    /// `LeaseHeldByAnotherWorker` or `PartitionStepBudgetExhausted`, which are
    /// decisions `acquire` makes before any work starts and which carry
    /// different recovery.
    ///
    /// It can no longer mean "bound a different cell, closure, registry, policy
    /// or calculation than the work being resumed". Every verification key
    /// commits everything its checkpoint pins, so different work is a DIFFERENT
    /// KEY and starts a fresh verification instead of colliding. Collapsing the
    /// two would be actively harmful: the module prescribes opposite recovery
    /// for them — resume from persisted progress, versus must not inherit the
    /// accumulators at all — and one error cannot direct both.
    ///
    /// BOUNDARY against the identity errors: `CellMismatch` and `PolicyMismatch`
    /// fire when the RECORD OR EXPECTATION the caller supplied disagrees with
    /// the cell revision or policy being verified. They are about what was
    /// handed in. `CheckpointConflict` is about the durable store alone and says
    /// nothing about whether the caller's inputs agree.
    CheckpointConflict,
}
