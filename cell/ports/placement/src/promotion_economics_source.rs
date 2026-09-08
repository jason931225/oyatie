//! Admission, finalization and closure of the accounting and usage sources a
//! promotion economics record is replayed from.
//!
//! A caller-supplied input root is not evidence of completeness. It proves the
//! promoter's chosen rows hash to the promoter's chosen value, which is exactly
//! what a promoter minimising its own unit cost would produce. Completeness is
//! instead established here, in two independent steps that the promoter does
//! not sit between:
//!
//! 1. The cell's own policy selects a [`PromotionEconomicsSourceRegistryV1`]
//!    naming every source scope required for this cell and window. That
//!    selection reaches this contract through
//!    [`CellPromotionEconomicsPolicySource`], a port, rather than through an
//!    argument: a registry a caller hands in is a registry a caller chose.
//! 2. Each admitted source's owner signs a
//!    [`SignedPromotionEconomicsSourceFinalizationV1`] over a closed, immutable
//!    snapshot of its own scope.
//!
//! Only when every admitted scope has an authenticated finalization, with exact
//! coverage and no missing, additional or duplicated source, does the issuer
//! produce a [`VerifiedPromotionEconomicsClosure`]. Completeness relative to
//! the accounting authority remains an attested boundary: no Merkle root over
//! rows you were given can prove anything about invoices you were not.
//!
//! Everything here is a declaration; every constructor, issuer, verifier and
//! reader fails closed with a typed `NotImplemented`.

use crate::{
    BoxCellFuture, CellControlReadAuthorityV1, CellId, CellProofEnvelopeV1, CellProofVerifier,
    CellRevisionIdentityV1, Digest32, ImmutableEvidenceRefV1, KeyId, PlacementPartitionV1,
    ProducerId, PromotionCostTaxonomyV1, PromotionEconomicsPolicyV1,
    PromotionEconomicsVerificationErrorV1, PromotionEconomicsVerificationKeyV1,
    PromotionEconomicsWindowV1,
};

/// The semantic role a source plays.
///
/// These are role labels for admission, not assertions that a matching deployed
/// adapter exists. Admitted combinations with
/// [`PromotionEconomicsSourceKindV1`] are: `Accounting` or `Billing` may
/// finalize effective charges; `Observability` may finalize committed-home
/// capacity intervals. Any other pairing fails admission.
///
/// `Accounting` and `Billing` overlapping is prevented by canonical charge
/// ownership rather than by preferring one owner: exactly one admitted scope
/// owns each canonical charge identity, and duplicate or overlapping ownership
/// domains fail registry and closure verification. A provider invoice copied
/// into both owners is one charge, not two.
///
/// Declaration order carries no rank.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionEconomicsSourceOwnerV1 {
    Accounting,
    Billing,
    Observability,
}

/// What a source finalizes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PromotionEconomicsSourceKindV1 {
    FinalizedEffectiveCharges,
    /// Backed by native committed reservation identity and state. Sampled
    /// demand or utilization telemetry is not an admissible substitute for a
    /// committed-home interval.
    CommittedHomeCapacityIntervals,
}

/// One admitted source scope: a bounded, non-overlapping native key range under
/// one producer, for one cell and partition.
///
/// The native key range is half-open
/// `[native_key_range_start_inclusive, native_key_range_end_exclusive)` under a
/// fixed-version, length-bounded key encoding. Half-open explicit endpoints are
/// what let a streaming verifier check `previous end <= next start` between
/// adjacent scopes without holding a global set of seen keys. Every retained
/// row's canonical native key must fall inside its own scope's range.
///
/// Source identity is the canonical native key, never a freely supplied alias,
/// so the same source object cannot be re-presented under another reservation.
/// Cross-scope identity ownership is established by the registry's disjoint
/// scopes; detecting arbitrary cross-source aliases after admission would need
/// unbounded state and is therefore not an admitted source format.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsSourceScopeV1 {
    pub owner: PromotionEconomicsSourceOwnerV1,
    pub producer: ProducerId,
    pub authority_id: String,
    pub repository_id: String,
    pub scope_id: String,
    pub partition: PlacementPartitionV1,
    pub cell_id: CellId,
    pub kind: PromotionEconomicsSourceKindV1,
    pub source_schema_version: u32,
    pub canonical_encoding_version: u32,
    pub native_key_encoding_version: u32,
    pub native_key_range_start_inclusive: Vec<u8>,
    pub native_key_range_end_exclusive: Vec<u8>,
    pub scope_digest: Digest32,
}

/// The cell's admission of one source scope.
///
/// Admission is explicit per scope, schema, producer, signing key, partition
/// and cell. `cost_taxonomy_digest` must equal the policy taxonomy's
/// `definition_digest`, which is what binds this source's classifications to
/// the same exhaustive taxonomy every other source used.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsSourceAdmissionV1 {
    pub scope: PromotionEconomicsSourceScopeV1,
    pub admission_generation: u64,
    pub expected_signing_key_id: KeyId,
    pub cost_taxonomy_digest: Digest32,
    pub row_order_version: u32,
    pub admission_digest: Digest32,
}

/// The complete set of source scopes the cell requires for one cell and
/// partition.
///
/// It is a plain public-field record with no producer and no verifier anywhere
/// in this crate, so possessing one proves nothing. What is supposed to make a
/// registry EXPECTED is where it came from:
/// [`CellPromotionEconomicsPolicySource`] is the only route in this contract by
/// which one becomes the registry a closure is resolved against, and no public
/// entrypoint on this path takes one as an argument.
///
/// An earlier version of this doc said the registry is "issued under verified
/// cell control policy authority" while
/// [`CellPromotionEconomicsClosureIssuerV1::admit`] took the registry from its
/// own caller and held no port through which any policy authority could be
/// reached. The issuing authority it named did not exist in the contract. It
/// exists now, as a port. What that port does NOT do is authenticate the
/// registry value — no type here can — and the half that stays a deployment
/// obligation is stated on the port itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsSourceRegistryV1 {
    pub partition: PlacementPartitionV1,
    pub cell_id: CellId,
    pub generation: u64,
    pub ordered_admission_root_digest: Digest32,
    pub admission_count: u64,
    pub immutable_record: ImmutableEvidenceRefV1,
    pub registry_digest: Digest32,
}

/// What a source owner signs once its scoped window is closed.
///
/// A source issuer signs only after the entire scoped window is closed and
/// immutable: `finalized_through_unix_seconds >= window.end_unix_seconds`,
/// `unclassified_record_count == 0`, and a fixed snapshot version, root, count
/// and retention. An empty source issues a signed zero-count snapshot rather
/// than staying silent, because silence and emptiness must not look alike.
///
/// Late adjustments require a newly finalized snapshot and a new cell closure.
/// An old manifest is never mutated, and a reused `snapshot_id` whose bytes
/// changed fails.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsSourceFinalizationPayloadV1 {
    pub admission_digest: Digest32,
    pub scope: PromotionEconomicsSourceScopeV1,
    pub window: PromotionEconomicsWindowV1,
    pub snapshot_id: String,
    pub snapshot_revision: u64,
    pub ordered_record_root_digest: Digest32,
    pub record_count: u64,
    pub canonical_bytes: u64,
    pub finalized_through_unix_seconds: u64,
    pub retained_until_unix_seconds: u64,
    pub cost_taxonomy_digest: Digest32,
    /// Must be zero. A nonzero count means the source itself could not classify
    /// part of its own window, so the population is not exhaustive and the
    /// closure refuses.
    pub unclassified_record_count: u64,
    pub payload_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedPromotionEconomicsSourceFinalizationV1 {
    pub payload: PromotionEconomicsSourceFinalizationPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

/// A source finalization whose signature, producer, key, domain, audience,
/// admission and schema have all been checked.
///
/// Private field, no public constructor. Signing keys and their historical
/// validity are checked through the existing proof-key mechanism, not ambient
/// trust; an accounting record is a finalized historical input, so a key valid
/// at signing time need not still be valid now.
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedPromotionEconomicsSourceFinalization(
    SignedPromotionEconomicsSourceFinalizationV1,
);

impl VerifiedPromotionEconomicsSourceFinalization {
    #[must_use]
    pub fn signed(&self) -> &SignedPromotionEconomicsSourceFinalizationV1 {
        &self.0
    }
}

/// Verifies one signed source finalization against its admission and the
/// requested window.
///
/// Checks the envelope domain
/// [`crate::CellProofDomainV1::PromotionEconomicsSourceFinalization`], the
/// producer and signing key the admission expects, exact scope and admission
/// digest equality, the requested window, closure of that window
/// (`finalized_through >= window.end`, `window.end <= now`), a zero
/// unclassified count, and the taxonomy digest.
pub fn verify_promotion_economics_source_finalization(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedPromotionEconomicsSourceFinalizationV1,
    _admission: &PromotionEconomicsSourceAdmissionV1,
    _expected_window: &PromotionEconomicsWindowV1,
    _now_unix_seconds: u64,
) -> Result<VerifiedPromotionEconomicsSourceFinalization, PromotionEconomicsVerificationErrorV1> {
    Err(PromotionEconomicsVerificationErrorV1::NotImplemented)
}

/// The exact, cell-issued statement of what population a promotion economics
/// record must replay.
///
/// It commits every finalized source independently rather than one combined
/// manifest the promoter could have assembled. `retained_until_unix_seconds` is
/// the MINIMUM of the authenticated source retention deadlines: the population
/// is only replayable while its shortest-lived member is still retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsClosureV1 {
    pub partition: PlacementPartitionV1,
    pub cell: CellRevisionIdentityV1,
    pub window: PromotionEconomicsWindowV1,
    pub registry: PromotionEconomicsSourceRegistryV1,
    pub policy_digest: Digest32,
    pub taxonomy: PromotionCostTaxonomyV1,
    pub ordered_finalization_root_digest: Digest32,
    pub finalization_count: u64,
    pub retained_until_unix_seconds: u64,
    pub closure_digest: Digest32,
}

/// A closure every admitted source of which has been covered by an
/// authenticated finalization.
///
/// Private field, no public constructor, no deserialization path. The only
/// producer is the private `advance_promotion_economics_closure_step`, whose
/// registry and policy come from [`CellPromotionEconomicsPolicySource`] rather
/// than from anything the caller of closure construction passes.
///
/// THAT LAST CLAUSE IS A PROPERTY OF THIS SIGNATURE, NOT OF THE TRUST MODEL.
/// The private field refuses direct construction. What it does not refuse is a
/// deployment that composes a dishonest policy source, and no type in this
/// crate can: see [`CellPromotionEconomicsPolicySource`] for what the port
/// establishes and what it leaves to the composition root.
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedPromotionEconomicsClosure(PromotionEconomicsClosureV1);

impl VerifiedPromotionEconomicsClosure {
    #[must_use]
    pub fn closure(&self) -> &PromotionEconomicsClosureV1 {
        &self.0
    }
}

/// What a caller asks the closure authority to resolve.
///
/// It names the cell, partition, window and the policy generation the caller
/// believes is current. There is deliberately no member through which it could
/// name a registry, a policy or a taxonomy: those are resolved for this address
/// and generation through [`CellPromotionEconomicsPolicySource`], so the
/// population and the judge of the population do not arrive from the party
/// whose closure is being resolved.
///
/// `expected_policy_generation` is an EXPECTATION, not a selection. It is
/// compared against the generation the policy source returns, and a
/// disagreement is
/// [`PromotionEconomicsVerificationErrorV1::PolicyMismatch`]; a caller naming a
/// generation cannot thereby obtain a population of its own choosing, because
/// what that generation names is the policy source's answer and not the
/// caller's.
///
/// `now_unix_seconds` is carried here rather than passed separately because
/// [`PromotionEconomicsClosureAuthority::resolve_expected`] is the only public
/// entrypoint into closure construction, and the finalization and retention
/// checks it drives need a caller-stated evaluation time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsClosureRequestV1 {
    pub partition: PlacementPartitionV1,
    pub cell: CellRevisionIdentityV1,
    pub window: PromotionEconomicsWindowV1,
    pub expected_policy_generation: u64,
    pub now_unix_seconds: u64,
}

/// The cell-owned issuer port that resolves the expected closure.
///
/// This trait is the ONLY public way to obtain a
/// [`VerifiedPromotionEconomicsClosure`], and neither it nor
/// [`PromotionEconomicsClosureRequestV1`] has a member through which a caller
/// could offer a registry or a policy. Implementations resolve both through
/// [`CellPromotionEconomicsPolicySource`].
///
/// The trait is sealed in practice — the only implementer is the in-crate
/// [`CellPromotionEconomicsClosureIssuerV1`] — and that seal now delegates to a
/// port rather than to an admission that took its selection-relevant inputs
/// from its own caller. See the port for what the delegation establishes and
/// for the part of it that remains a deployment obligation.
///
/// Large registries use the same bounded resumable machinery as input replay:
/// while closure construction is unfinished, `resolve_expected` returns
/// [`PromotionEconomicsClosureStepOutcomeV1::Continued`] rather than scanning
/// without bound, and its background build advances under the
/// [`crate::PromotionEconomicsVerificationPhaseV1::VerifyingSourceClosure`]
/// phase.
///
/// Continuation is reported on the OK channel carrying the verification key and
/// the checkpoint revision, exactly as the sibling
/// [`crate::advance_cell_promotion_economics`] reports it. That is not
/// cosmetic. A bare "still working" refusal collapses "advancing" and "stuck"
/// into one observable, so a caller polling a long closure build over
/// successive calls cannot tell progress from a wedge. The revision is the
/// progress token: it strictly advances while work is being done and stands
/// still when it is not.
pub trait PromotionEconomicsClosureAuthority: Send + Sync {
    fn resolve_expected<'a>(
        &'a self,
        authority: &'a CellControlReadAuthorityV1,
        request: &'a PromotionEconomicsClosureRequestV1,
    ) -> BoxCellFuture<
        'a,
        Result<PromotionEconomicsClosureStepOutcomeV1, PromotionEconomicsVerificationErrorV1>,
    >;
}

/// What the cell's own policy surface is asked to select.
///
/// An address and a generation, and nothing else. There is deliberately no
/// member through which a caller could offer a registry, a policy or a
/// taxonomy: a selection request that carried the thing being selected would be
/// the hole this port exists to close.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsPolicySelectionV1 {
    pub partition: PlacementPartitionV1,
    pub cell_id: CellId,
    pub expected_policy_generation: u64,
}

/// The registry and the policy that the cell's own policy selects for one
/// address and generation.
///
/// The two arrive TOGETHER from one call, not from two. Splitting them would
/// let one be current and the other stale, and the check that binds them —
/// every admission's `cost_taxonomy_digest` against the policy taxonomy's
/// `definition_digest` — would then be comparing two generations and passing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedPromotionEconomicsPopulationV1 {
    pub registry: PromotionEconomicsSourceRegistryV1,
    pub policy: PromotionEconomicsPolicyV1,
}

/// The cell's own policy surface: the route by which a source registry and an
/// economics policy become the ones a closure is resolved against.
///
/// # Why this port exists
///
/// [`PromotionEconomicsSourceRegistryV1`] and
/// [`crate::PromotionEconomicsPolicyV1`] are the two SELECTION-RELEVANT inputs
/// on this path. The registry fixes which sources count; the policy fixes the
/// taxonomy, the thresholds and — through
/// [`crate::PromotionEconomicsCheckpointObserverAdmissionV1`] — who is admitted
/// to vouch for a checkpoint. Both are plain public-field structs with no
/// producer and no verifier, so a party that supplies both supplies the
/// population and the judge of the population in one breath.
///
/// [`CellPromotionEconomicsClosureIssuerV1::admit`] used to take both from its
/// caller while its doc prescribed resolving the registry "from native cell
/// policy at `policy.policy_generation`" — where `policy` was the caller's own
/// argument. That is a doc claiming a property its own signature could not
/// perform: no implementation of `admit`, however honest, could consult native
/// cell policy through a signature that contained no route to it, so the gap
/// could not have closed by implementation. It closes only by putting the route
/// in the signature, which is what this port is, and it is the same remedy this
/// module already applied when
/// [`crate::PromotionEconomicsReplayPortsV1`] gained the proof verifier it had
/// been documented as using.
///
/// # What it establishes, and what it does not
///
/// ESTABLISHED BY THE SIGNATURE: no per-call argument on this path can
/// substitute a registry or a policy. `resolve_expected` takes a
/// [`PromotionEconomicsClosureRequestV1`], which has no member for either; the
/// private closure step reaches both only through this port; and this port is
/// asked only for an address and a generation.
///
/// NOT ESTABLISHED, AND STATED RATHER THAN IMPLIED: this port is composed like
/// every other port here, so WHICH implementation serves native cell policy is
/// a DEPLOYMENT OBLIGATION. A composition root that wires an implementation the
/// promoter controls gets a population the promoter chose, and nothing in these
/// types refuses that. The contract expresses who is supposed to hold the role
/// and gives a conforming deployment the shape to enforce it; it does not
/// enforce it. This is the same species of obligation as clause (a) — see
/// [`crate::MovementActionResultAuthority`] — and it is written down here
/// because the previous wording of this module presented it as a structural
/// property three times over.
///
/// # Refusals
///
/// An implementation refuses rather than answering approximately:
/// [`PromotionEconomicsVerificationErrorV1::PolicyMismatch`] when no policy is
/// admitted at the requested generation or the returned pair disagree on
/// taxonomy; [`PromotionEconomicsVerificationErrorV1::CellMismatch`] when the
/// selected registry's `partition` or `cell_id` is not the one asked for; and
/// [`PromotionEconomicsVerificationErrorV1::NotAuthorized`] when `authority`
/// does not reach this cell's control state.
pub trait CellPromotionEconomicsPolicySource: Send + Sync {
    fn resolve_admitted_population<'a>(
        &'a self,
        authority: &'a CellControlReadAuthorityV1,
        selection: &'a PromotionEconomicsPolicySelectionV1,
    ) -> BoxCellFuture<
        'a,
        Result<AdmittedPromotionEconomicsPopulationV1, PromotionEconomicsVerificationErrorV1>,
    >;
}

/// One page request over the source registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsSourceRegistryPageRequestV1 {
    pub registry: PromotionEconomicsSourceRegistryV1,
    pub window: PromotionEconomicsWindowV1,
    pub first_source_ordinal: u64,
    pub maximum_rows: u32,
    pub maximum_bytes: u64,
}

/// One admitted source together with the signed finalization that covers it.
///
/// The pairing is what makes coverage checkable per page: a row carrying an
/// admission with no finalization, or a finalization for an unadmitted scope,
/// fails immediately rather than after the whole registry is assembled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsSourceRegistryMemberV1 {
    pub ordinal: u64,
    pub admission: PromotionEconomicsSourceAdmissionV1,
    pub finalization: SignedPromotionEconomicsSourceFinalizationV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionEconomicsSourceRegistryPageV1 {
    pub registry_digest: Digest32,
    pub first_source_ordinal: u64,
    pub members: Vec<PromotionEconomicsSourceRegistryMemberV1>,
    pub canonical_bytes: u64,
}

pub trait PromotionEconomicsSourceRegistryReader: Send + Sync {
    fn read_page<'a>(
        &'a self,
        authority: &'a CellControlReadAuthorityV1,
        request: &'a PromotionEconomicsSourceRegistryPageRequestV1,
    ) -> BoxCellFuture<
        'a,
        Result<PromotionEconomicsSourceRegistryPageV1, PromotionEconomicsVerificationErrorV1>,
    >;
}

/// Outcome of one bounded closure-construction step.
///
/// `Continued` means the step budget was spent and durable progress was
/// committed; it is not a refusal and carries no closure. The completed
/// closure is boxed because it embeds two full cell-scale identities and the
/// whole registry, which would otherwise make every `Continued` value as large
/// as a completed one.
///
/// `key` is boxed. Growing the key so it commits the cell revision made
/// `Continued` the large variant against a `Complete` whose payload was already
/// boxed, so both sides are now behind one pointer and neither shape pays for
/// the other.
#[derive(Debug, Eq, PartialEq)]
pub enum PromotionEconomicsClosureStepOutcomeV1 {
    Continued {
        key: Box<PromotionEconomicsVerificationKeyV1>,
        checkpoint_revision: u64,
    },
    Complete(Box<VerifiedPromotionEconomicsClosure>),
}

/// Advances closure construction by one bounded step.
///
/// PRIVATE ON PURPOSE. The registry is a selection-relevant plain public
/// struct, and this step can mint the private-field
/// [`VerifiedPromotionEconomicsClosure`]. A public function taking the registry
/// per call would let a caller select a smaller registry and have the result
/// come back stamped as verified.
///
/// Its `SourceClosure` verification key is deliberately keyed on the registry
/// digest and not on a closure digest, because the closure digest is the
/// output of this phase and does not exist while the phase is running.
///
/// The registry and the policy are obtained here, once per resolution, from
/// [`CellPromotionEconomicsClosureIssuerV1::policy_source`] under `authority`,
/// with a [`PromotionEconomicsPolicySelectionV1`] built from `request`'s
/// address and `expected_policy_generation`. The registry reader, checkpoint
/// store, checkpoint observer and proof verifier are reached through `issuer`,
/// which holds them from construction; the evaluation time is
/// [`PromotionEconomicsClosureRequestV1::now_unix_seconds`].
///
/// That is the whole reason `issuer` is passed rather than five loose values:
/// there is no parameter on this function through which a registry or a policy
/// could arrive from anywhere but the policy port. Before the port existed the
/// same guarantee was claimed for values the issuer had been HANDED at
/// construction by the same party that later called it, which guaranteed only
/// that the party could not change its mind mid-resolution.
fn advance_promotion_economics_closure_step<'a>(
    _issuer: &'a CellPromotionEconomicsClosureIssuerV1,
    _authority: &'a CellControlReadAuthorityV1,
    _request: &'a PromotionEconomicsClosureRequestV1,
) -> BoxCellFuture<
    'a,
    Result<PromotionEconomicsClosureStepOutcomeV1, PromotionEconomicsVerificationErrorV1>,
> {
    Box::pin(async { Err(PromotionEconomicsVerificationErrorV1::NotImplemented) })
}

/// The concrete cell-owned closure issuer.
///
/// It holds five ports from CONSTRUCTION and NO SELECTION-RELEVANT VALUES. The
/// registry and the policy are selection-relevant, so a surface that can yield
/// a private-field verified wrapper must not take either from the party asking
/// for the wrapper — and it must not take them from that party at construction
/// either, which is the correction this type carries. They are resolved instead
/// through [`CellPromotionEconomicsPolicySource`], and the only per-call
/// arguments are a read authority and a request naming a cell, a window and an
/// expected policy generation, none of which can substitute a source
/// population.
///
/// Every method here is a typed `NotImplemented` stub; nothing in this crate
/// resolves policy at runtime yet.
pub struct CellPromotionEconomicsClosureIssuerV1 {
    policy_source: Box<dyn CellPromotionEconomicsPolicySource>,
    registry_reader: Box<dyn PromotionEconomicsSourceRegistryReader>,
    checkpoint_store: Box<dyn crate::PromotionEconomicsCheckpointStore>,
    checkpoint_observer: Box<dyn crate::PromotionEconomicsCheckpointCommitObserver>,
    proof_verifier: Box<dyn CellProofVerifier>,
}

impl CellPromotionEconomicsClosureIssuerV1 {
    /// Admits one issuer over the ports it will resolve every closure through.
    ///
    /// IT TAKES NO REGISTRY AND NO POLICY, and that absence is the contract.
    /// Two earlier parameters carried both, while this doc prescribed resolving
    /// the registry "from native cell policy at `policy.policy_generation`" —
    /// naming the caller's own sixth argument as the anchor. The signature had
    /// no route to any policy authority, so the check it prescribed could not
    /// be performed by any implementation of it, honest or otherwise. The route
    /// is now `policy_source`, and the resolution happens per call in
    /// `advance_promotion_economics_closure_step` under the caller's verified
    /// [`CellControlReadAuthorityV1`], which is a value `admit` has no way to
    /// hold at composition time.
    ///
    /// What `admit` still cannot do is authenticate the ports it is handed.
    /// Choosing an implementation of `policy_source` that really serves native
    /// cell policy is the composition root's obligation; see the port.
    pub fn admit(
        _policy_source: Box<dyn CellPromotionEconomicsPolicySource>,
        _registry_reader: Box<dyn PromotionEconomicsSourceRegistryReader>,
        _checkpoint_store: Box<dyn crate::PromotionEconomicsCheckpointStore>,
        _checkpoint_observer: Box<dyn crate::PromotionEconomicsCheckpointCommitObserver>,
        _proof_verifier: Box<dyn CellProofVerifier>,
    ) -> Result<Self, PromotionEconomicsVerificationErrorV1> {
        Err(PromotionEconomicsVerificationErrorV1::NotImplemented)
    }

    /// The policy source this issuer was admitted with.
    ///
    /// There is no `registry()` and no `policy()` accessor, and their removal is
    /// deliberate rather than tidying: an issuer that could hand back "its"
    /// registry would be an issuer that had one before a request named a
    /// generation, which is the shape this type was corrected out of.
    #[must_use]
    pub fn policy_source(&self) -> &dyn CellPromotionEconomicsPolicySource {
        self.policy_source.as_ref()
    }

    /// The registry reader this issuer was admitted with.
    #[must_use]
    pub fn registry_reader(&self) -> &dyn PromotionEconomicsSourceRegistryReader {
        self.registry_reader.as_ref()
    }

    /// The checkpoint store this issuer was admitted with.
    #[must_use]
    pub fn checkpoint_store(&self) -> &dyn crate::PromotionEconomicsCheckpointStore {
        self.checkpoint_store.as_ref()
    }

    /// The checkpoint commit observer this issuer was admitted with.
    ///
    /// Required from CONSTRUCTION for the same A3 reason as the store: the
    /// closure step is private and reaches every dependency through the issuer,
    /// and since a checkpoint store now reports only durable records, resuming
    /// retained progress needs this port to obtain an independently observed
    /// claim before the module's verifier will mint a wrapper from it. Without
    /// it the sealed step could not use the store its own contract mandates.
    #[must_use]
    pub fn checkpoint_observer(&self) -> &dyn crate::PromotionEconomicsCheckpointCommitObserver {
        self.checkpoint_observer.as_ref()
    }

    /// The proof verifier this issuer was admitted with, used to authenticate
    /// each source finalization.
    #[must_use]
    pub fn proof_verifier(&self) -> &dyn CellProofVerifier {
        self.proof_verifier.as_ref()
    }
}

impl PromotionEconomicsClosureAuthority for CellPromotionEconomicsClosureIssuerV1 {
    fn resolve_expected<'a>(
        &'a self,
        authority: &'a CellControlReadAuthorityV1,
        request: &'a PromotionEconomicsClosureRequestV1,
    ) -> BoxCellFuture<
        'a,
        Result<PromotionEconomicsClosureStepOutcomeV1, PromotionEconomicsVerificationErrorV1>,
    > {
        Box::pin(
            async move { advance_promotion_economics_closure_step(self, authority, request).await },
        )
    }
}
