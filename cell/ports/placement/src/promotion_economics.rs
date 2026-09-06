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
    CurrencyCode, Digest32, ImmutableEvidenceRefV1, MoneyMicrounitsV1, ReservationRefV1,
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
/// counts recorded elsewhere are completeness commitments, not size caps; the
/// only size-shaped refusal is `ArithmeticOverflow`, which is a
/// representability failure rather than a deliberate ceiling.
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
    MissingInput,
    NotAuthorized,
    DependencyUnavailable,
    UnsupportedCalculation,
    PolicyMismatch,
    CellMismatch,
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
    /// A checkpoint lease expired, lost a compare-and-set race, or bound a
    /// different cell, closure, registry, policy or calculation than the work
    /// being resumed.
    CheckpointConflict,
}
