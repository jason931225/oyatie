//! Cell-owned capability effect boundary: scope, projected owner authority
//! context, signed grant, durable local authority state and commit request.
//!
//! Cell states the conformance vocabulary. Tenancy remains the only owner of
//! binding generation, revision, write-authority epoch and serving
//! incarnation; every owner-assigned value that appears here is an immutable
//! projection Cell compares against and never allocates or increments.

use crate::{
    CellId, CellProofEnvelopeV1, CurrencyCode, Digest32, DrainContributorStateMutationV1,
    ProducerId, ProofConstructionError, TenantId, VerifiedCapabilityEffectGrantV1,
};

/// Opaque identifiers, matching the shape used by `proof.rs` and
/// `economics.rs`.
///
/// These keep `Ord`, unlike the domain enums and the serving incarnation in
/// this module. Lexicographic order over an opaque identifier is arbitrary
/// but stable, which is what canonical ordering and duplicate detection need,
/// and it asserts nothing: no reader takes `key_a < key_b` to mean that one
/// was issued first or outranks the other. The derives removed elsewhere in
/// this module were removed because their orderings would have been read as
/// domain facts, not because ordering is suspect in itself.
macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn parse(_value: impl Into<String>) -> Result<Self, ProofConstructionError> {
                Err(ProofConstructionError::NotImplemented)
            }
        }
    };
}

opaque_id!(CapabilityParticipantKeyV1);
opaque_id!(CapabilityEffectKeyV1);
opaque_id!(CapabilityAdapterIdV1);

/// A projected Tenancy write-authority epoch. Compared, never allocated by
/// Cell.
///
/// Its ordering IS load-bearing and is derived deliberately: the owner issues
/// these in sequence, so a lower fence than the installed one is exactly what
/// makes an authority stale. Note the asymmetry with
/// [`ProjectedServingIncarnationV1`], which derives no ordering: the fence
/// answers "older or newer", the incarnation answers "the same one or a
/// different one", and neither answers the other's question. Equal fences
/// with different incarnations is the case that motivates carrying both.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalAuthorityFenceV1(pub u64);

/// The capability store's own durable record revision.
///
/// Its ordering is load-bearing for the same reason: revisions advance within
/// one record, so comparing them is a real recency test rather than a
/// coincidence of representation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalCommitRevisionV1(pub u64);

/// A projection of the independently generated Tenancy serving incarnation.
///
/// The bytes are opaque to Cell: equality and rejection membership are the
/// only operations. Comparing a numeric epoch alone cannot distinguish a
/// replaced incarnation from the one it replaced, which is why this is not
/// folded into [`LocalAuthorityFenceV1`].
///
/// Two incarnations are incomparable. They are distinct identities of which
/// at most one serves, not two points on a scale, and the bytes are
/// independently generated rather than issued in a sequence. Deriving an
/// ordering would invite a future implementation to write a comparison that
/// reads as a recency test — take the greater of two incarnations, or check
/// that an observed one is not less than the installed one — and get a
/// confident answer to a question the bytes cannot answer. A replaced
/// incarnation may sort either side of its replacement. The only sound test
/// is identity against the installed instance, plus membership in durable
/// rejection state.
///
/// `Eq` and `Hash` are therefore the derives the semantics justify, and they
/// are deliberate rather than incidental: `Eq` is that identity test, and
/// `Hash` lets an implementation hold observed incarnations in a set while it
/// reconciles them against the rejection root and count that the store
/// durably owns. Byte order carries no rank.
///
/// The Tenancy counterpart this projects, `ServingAuthorityIncarnationV1`,
/// likewise derives no ordering.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ProjectedServingIncarnationV1(Vec<u8>);

impl ProjectedServingIncarnationV1 {
    pub fn parse(_independently_generated: Vec<u8>) -> Result<Self, CapabilityEffectErrorV1> {
        Err(CapabilityEffectErrorV1::NotImplemented)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// The exact physical partition an effect commits into.
///
/// `transaction_domain_digest` names the commit boundary itself: one cell and
/// one adapter can front several transaction domains, and a grant admitted for
/// one of them proves nothing about another.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityPartitionRefV1 {
    pub cell: CellId,
    pub adapter: CapabilityAdapterIdV1,
    pub partition_id: String,
    pub topology_digest: Digest32,
    pub transaction_domain_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityEffectScopeV1 {
    pub tenant: TenantId,
    pub participant: CapabilityParticipantKeyV1,
    pub partition: CapabilityPartitionRefV1,
    pub authority_issuer: ProducerId,
}

/// The serving partition as projected from installed Tenancy authority. This
/// is the authority's partition, not necessarily the capability's physical
/// effect partition, so it is a separate reference from
/// [`CapabilityPartitionRefV1`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedServingPartitionRefV1 {
    pub cell: CellId,
    pub partition_id: String,
    pub topology_digest: Digest32,
}

/// Installed, serving authority. Only this context admits Activate, Write,
/// Fence and Release.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledCapabilityAuthorityContextV1 {
    pub serving_partition: ProjectedServingPartitionRefV1,
    pub incarnation: ProjectedServingIncarnationV1,
    pub authority_fence: LocalAuthorityFenceV1,
    pub owner_generation: u64,
    pub owner_revision: u64,
    pub owner_binding_record_digest: Digest32,
    pub installation_issuance_digest: Digest32,
    pub installed_grant_digest: Digest32,
    pub authority_lease_digest: Digest32,
    pub authority_lease_expires_at_unix_seconds: u64,
}

/// Preparation authority, issued by the Tenancy control owner from a binding
/// reservation attempt.
///
/// It carries no epoch, no lease and no incarnation because none has been
/// assigned yet. Inventing a zero epoch here would let a preparation grant
/// compare equal to a real fence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityPreparationContextV1 {
    pub owner_operation_digest: Digest32,
    pub binding_attempt_digest: Digest32,
    pub placement_decision_digest: Digest32,
    pub reservation_commit_permit_digest: Digest32,
    pub target_partition: CapabilityPartitionRefV1,
    pub preparation_authority_digest: Digest32,
    pub expires_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityAuthorityContextV1 {
    Preparation(CapabilityPreparationContextV1),
    Installed(InstalledCapabilityAuthorityContextV1),
}

/// What a grant can authorize.
///
/// Action and context are not independent. The matrix below is enforced at
/// verification and again at the effect transaction, because a grant that
/// passed verification can still arrive after its authority was fenced:
///
/// Under a [`CapabilityAuthorityContextV1::Preparation`] context, and nothing
/// else: `Prepare`; `PreparationCleanup`; and `Transfer` into non-serving
/// staged data when the adapter's acceptance separately admits it. A
/// preparation grant can never satisfy `Activate`, `Write`, `Fence` or
/// `Release`.
///
/// Under a [`CapabilityAuthorityContextV1::Installed`] context: `Activate`,
/// `Write`, `Fence`, `Release` and `Transfer`. Never `PreparationCleanup`.
///
/// Every action must also appear in the acceptance's `supported_actions`;
/// membership of this enum admits nothing on its own.
///
/// Actions are incomparable and no ordering is derived. Declaration order and
/// protobuf tag order carry no rank, and the shape of this list makes that
/// easy to get wrong: `Write` follows `Fence` and `PreparationCleanup`
/// follows `Release`, so a comparison such as `action >= Fence` would read as
/// a privilege test and answer something else entirely — it would admit
/// `PreparationCleanup`, the least privileged action here, and reject
/// `Prepare`. What an action may do is decided by the context matrix above
/// and by `supported_actions`, never by a rank. `Hash` is retained because
/// `supported_actions` must be checked for duplicates and membership.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CapabilityEffectActionV1 {
    Prepare,
    Activate,
    Fence,
    Write,
    Transfer,
    Release,
    /// Removes staged data left by a preparation that never activated.
    ///
    /// It exists because the alternative is to spend `Release`, which is
    /// installed-authority retirement: reusing it here would let preparation
    /// authority perform a retirement it was never granted, and would make a
    /// cleanup indistinguishable in the audit record from a real release.
    /// Authorized only under a Preparation context; refused under Installed.
    PreparationCleanup,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityEffectGrantPayloadV1 {
    pub schema_version: u32,
    pub scope: CapabilityEffectScopeV1,
    pub action: CapabilityEffectActionV1,
    pub context: CapabilityAuthorityContextV1,
    pub owner_operation_digest: Digest32,
    pub owner_context_digest: Digest32,
    pub manifest_digest: Digest32,
    pub member_digest: Digest32,
    pub owner_authority_proof_digest: Digest32,
    pub effect_key: CapabilityEffectKeyV1,
    pub effect_schema_digest: Digest32,
    pub effect_digest: Digest32,
    pub acceptance_digest: Digest32,
    pub audit_policy_digest: Digest32,
    pub maximum_effect_bytes: u64,
    pub maximum_effect_cost_microunits: u64,
    pub currency: CurrencyCode,
    pub worker_identity_digest: Digest32,
    pub worker_epoch: u64,
    pub not_before_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedCapabilityEffectGrantV1 {
    pub payload: CapabilityEffectGrantPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

/// What the caller already knows, built from the capability's own installed
/// state and never from the wire message under verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityEffectExpectationV1 {
    pub scope: CapabilityEffectScopeV1,
    pub action: CapabilityEffectActionV1,
    pub owner_operation_digest: Digest32,
    pub owner_context_digest: Digest32,
    pub manifest_digest: Digest32,
    pub member_digest: Digest32,
    pub effect_key: CapabilityEffectKeyV1,
    pub effect_schema_digest: Digest32,
    pub effect_digest: Digest32,
    pub acceptance_digest: Digest32,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
    pub maximum_clock_uncertainty_millis: u64,
    /// Digest over the canonical encoding of the expected
    /// [`CapabilityAuthorityContextV1`], discriminant included.
    ///
    /// Without it the verifier has nothing to compare
    /// [`CapabilityEffectGrantPayloadV1::context`] against, and the
    /// Preparation-versus-Installed distinction is unenforceable at
    /// verification. Every other authority input here is already a digest and
    /// the caller holds this preimage for the same reason it holds those.
    pub expected_authority_context_digest: Digest32,
}

/// What the durable authority record says the participant may do next.
///
/// Action-to-disposition, which is a DIFFERENT relation from the
/// action-to-context matrix on [`CapabilityEffectActionV1`]: that one says
/// which authority may request an action, this one says which record the
/// action leaves behind. A grant can satisfy the first and still be refused
/// by the second.
///
///   Prepare             -> `Prepared`, from no record at all.
///   Activate            -> `Writable`, from `Prepared` only.
///   Write               -> `Writable` unchanged. A Write that would change
///                          the disposition is not a Write.
///   Fence               -> `Fenced`, and monotonically: no action returns a
///                          record from `Fenced` to `Writable`. Only an
///                          authenticated higher owner authority, installed
///                          afresh, serves again.
///   Release             -> `Released`. TERMINAL.
///   PreparationCleanup  -> `PreparationDiscarded`, from `Prepared` only.
///                          TERMINAL.
///   Transfer            -> the disposition is unchanged, because a transfer
///                          targets staged data and cannot activate it.
///
/// Transitions the source documents do not state are NOT invented here and
/// remain owner-decided: notably whether Transfer is admissible from
/// `Fenced`, and whether a `Fenced` record may be released directly or must
/// first be reinstalled.
///
/// No ordering is derived, because these are states in a branching relation
/// and not stages on a scale. The declaration order almost reads as a
/// lifecycle, which is precisely the trap: `PreparationDiscarded` sorts last
/// while being reached from `Prepared` without ever passing through
/// `Writable`, so `disposition > Writable` would silently merge two different
/// branches, and `disposition >= Fenced` would read as "terminal" while
/// `Fenced` is not terminal at all. Whether a record is terminal is stated by
/// the transition relation above and by the two variants marked terminal,
/// never by a comparison.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LocalAuthorityDispositionV1 {
    Prepared,
    Writable,
    Fenced,
    /// Terminal, reached only by `Release` under installed authority.
    Released,
    /// Terminal, reached only by `PreparationCleanup` under preparation
    /// authority.
    ///
    /// It exists because `PreparationCleanup` was given no terminal state when
    /// it was introduced. Mapping it onto `Released` would have made a
    /// discarded preparation indistinguishable, in the durable record and in
    /// the audit trail, from a real installed-authority retirement, which is
    /// the confusion that action was created to prevent.
    PreparationDiscarded,
}

/// Durable rejection membership retained by the capability store.
///
/// The root and count reference retained rows, not a caller-supplied hash: a
/// known-rejected incarnation must stay rejected at the same numeric epoch,
/// and a caller-only digest cannot establish that.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityAuthorityRejectionHighWaterV1 {
    pub scope: CapabilityEffectScopeV1,
    pub owner_generation: u64,
    pub rejected_instance_root_digest: Digest32,
    pub rejected_instance_count: u64,
    pub revision: LocalCommitRevisionV1,
    pub record_digest: Digest32,
}

/// The durable local authority record.
///
/// Fields are public, matching the peer durable-state records in this crate
/// and in the binding crate. This type carries no signature and has no
/// verifier: its integrity comes from the capability's own transaction, not
/// from a check anyone could run on it. A private field with no mint would
/// protect nothing and would make the record unwritable by the out-of-crate
/// adapter that has to write it.
///
/// It deliberately does NOT embed the rejection high water. That is a
/// separately revisioned durable row, carried beside this one in the same
/// commit; embedding a copy of it here would let its revision advance while
/// this record's `record_digest` still covered the stale snapshot.
///
/// Public fields make the record writable by the adapter that has to write
/// it; they do not make it choosable. `context` in particular is a projection
/// the store copies from the signed grant, never a value a caller selects: see
/// [`LocalEffectCommitRequestV1::next_state`] for the obligation on each field
/// and the refusal that covers it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAuthorityStateV1 {
    pub scope: CapabilityEffectScopeV1,
    pub context: CapabilityAuthorityContextV1,
    pub disposition: LocalAuthorityDispositionV1,
    pub revision: LocalCommitRevisionV1,
    pub record_digest: Digest32,
}

/// Compare-and-set precondition for the local authority record.
///
/// Both arms carry rejection membership: absence of a state row is not
/// evidence that nothing was ever rejected at this scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalAuthorityPreconditionV1 {
    Absent {
        /// `None` only when the store holds no rejection membership row for
        /// this scope AT ALL, which is the state of a participant that has
        /// never been installed anywhere. It is read from
        /// [`crate::CapabilityLocalEffectStoreV1::get_rejection_high_water`], never
        /// assumed: `None` asserts that the store looked and found no row,
        /// which is a different claim from "nothing was rejected".
        ///
        /// A store that finds an authority record but no membership row is
        /// looking at restored or truncated state and returns
        /// [`CapabilityEffectErrorV1::RestoreEvidenceRequired`] rather than
        /// letting a caller assemble `None` here and start over.
        ///
        /// Without this option a participant's first `Prepare` cannot be
        /// assembled at all, because there is no row for the read to return.
        rejection_high_water: Option<CapabilityAuthorityRejectionHighWaterV1>,
    },
    Matches {
        revision: LocalCommitRevisionV1,
        record_digest: Digest32,
        rejection_high_water: CapabilityAuthorityRejectionHighWaterV1,
    },
}

/// One capability-owned physical commit request. `E` is the capability's own
/// native effect type; Cell interprets no arbitrary program.
#[derive(Debug, Eq, PartialEq)]
pub struct LocalEffectCommitRequestV1<E> {
    pub authority: VerifiedCapabilityEffectGrantV1,
    pub precondition: LocalAuthorityPreconditionV1,
    /// The authority record this effect leaves behind.
    ///
    /// Never optional. Every admitted action writes a record, terminals
    /// included: `LocalAuthorityPreconditionV1::Absent` describes only the
    /// state before a participant's first `Prepare`, never a state an action
    /// returns to. A terminal disposition retains the row so that a later
    /// request cannot read absence and start over.
    ///
    /// It is an ASSERTION, not an authority to write. Every one of its fields
    /// is derivable by the store from the grant, the action and the
    /// precondition, so the caller proposes nothing the store does not
    /// independently recompute. It is carried because the write set must be a
    /// complete, digestible description of the mutation that conformance can
    /// compare against the persisted row, and because a caller that has
    /// misunderstood the transition should be refused rather than silently
    /// corrected.
    ///
    /// Every field therefore has an obligation and a refusal:
    ///
    /// - `scope` must equal the grant's scope. A successor naming a different
    ///   tenant, participant or partition would write outside the grant.
    ///   Refusal: [`CapabilityEffectErrorV1::RelationMismatch`].
    /// - `context` must be the grant's context, field for field. Cell never
    ///   assigns owner generation, revision, fence or incarnation; it only
    ///   projects what the owner signed. A caller proposing different ones is
    ///   proposing to install authority that was never granted, which Cell has
    ///   no power to accept at any disposition.
    ///   Refusal: [`CapabilityEffectErrorV1::AuthorityContextMismatch`].
    /// - `disposition` must be the one the grant's action produces, per
    ///   [`LocalAuthorityDispositionV1`].
    ///   Refusal: [`CapabilityEffectErrorV1::DispositionMismatch`].
    /// - `revision` must be the successor of the revision the precondition
    ///   matched. Refusal: [`CapabilityEffectErrorV1::Conflict`].
    /// - `record_digest` must equal the store's own recomputation over the
    ///   canonical successor record.
    ///   Refusal: [`CapabilityEffectErrorV1::RelationMismatch`].
    pub next_state: LocalAuthorityStateV1,
    /// The rejection membership this effect leaves behind, carried separately
    /// because it is a separately revisioned row, exactly as the peer serving
    /// authority write sets carry theirs.
    ///
    /// A Fence records rejection membership and the state mutation in one
    /// commit; an action that rejects nothing still restates the high water it
    /// observed, so that "unchanged" is written down rather than inferred.
    pub next_rejection_high_water: CapabilityAuthorityRejectionHighWaterV1,
    pub effect: E,
    pub idempotency_key_digest: Digest32,
    pub canonical_request_digest: Digest32,
    pub drain_mutation: DrainContributorStateMutationV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalEffectCommitReceiptPayloadV1 {
    pub scope: CapabilityEffectScopeV1,
    pub action: CapabilityEffectActionV1,
    pub effect_key: CapabilityEffectKeyV1,
    pub grant_digest: Digest32,
    pub request_digest: Digest32,
    pub effect_digest: Digest32,
    pub result_digest: Digest32,
    pub previous_state_digest: Digest32,
    pub committed_state_digest: Digest32,
    pub commit_revision: LocalCommitRevisionV1,
    pub proof_consumption_digest: Digest32,
    pub idempotency_record_digest: Digest32,
    pub drain_state_digest: Digest32,
    pub audit_record_digest: Digest32,
    pub accounted_bytes: u64,
    pub accounted_cost_microunits: u64,
    pub committed_at_unix_seconds: u64,
    pub rejection_high_water_digest: Digest32,
    pub authority_context_digest: Digest32,
    pub adapter_acceptance_digest: Digest32,
    pub currency: CurrencyCode,
}

/// The published receipt. It is produced only from an already verified
/// committed observation; see [`crate::CapabilityEffectReceiptPublisherV1`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedLocalEffectCommitReceiptV1 {
    pub payload: LocalEffectCommitReceiptPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

/// Refusals at the capability effect boundary.
///
/// Every variant states its boundary against the neighbour it is most easily
/// confused with. An unstated overlap is not neutral: it gets resolved
/// arbitrarily by whoever implements first, and two adapters then disagree
/// about what the same refusal means.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityEffectErrorV1 {
    /// The contract exists and nothing implements it yet. Every constructor
    /// and verifier in this module returns this today. Not `NotAdmitted`:
    /// that one means an implementation exists and is not qualified to run.
    NotImplemented,
    /// No current, verified acceptance covers this adapter at this
    /// transaction domain, or the acceptance does not list the requested
    /// action in `supported_actions`.
    ///
    /// Not `UnsupportedSink`: this is a missing or insufficient
    /// qualification for a store that could in principle qualify. Not
    /// `NotAuthorized`-shaped either; the caller's authority is not in
    /// question here, the adapter's admission is.
    NotAdmitted,
    /// The sink cannot provide same-boundary fencing, dedup and receipt
    /// semantics at all, so no acceptance could ever be issued for it.
    ///
    /// Not `NotAdmitted`: that is a qualification this deployment lacks;
    /// this is a property the sink does not have. A bare network call
    /// between local begin and commit lands here, permanently.
    UnsupportedSink,
    /// A signature, envelope, domain, producer, audience or key check failed:
    /// the bytes are not authentic.
    ///
    /// Not `RelationMismatch`: authenticity failed before any field was
    /// compared. A forged grant is this; an authentic grant for the wrong
    /// tenant is that.
    VerificationFailed,
    /// The bytes are authentic and a compared field disagrees with what was
    /// expected: scope, partition, action, effect key, a digest, or the
    /// authority context digest.
    ///
    /// Also covers a proposed successor record whose `scope` is not the
    /// grant's, or whose `record_digest` is not the store's own recomputation
    /// over the canonical successor.
    ///
    /// Not `VerificationFailed`. Reporting a relation failure as an
    /// authenticity failure hides which side is wrong. Not
    /// `AuthorityContextMismatch`, which is reserved for the owner-assigned
    /// half of the successor's context, where the caller is not merely
    /// inconsistent but is proposing authority it does not hold.
    RelationMismatch,
    /// The grant, lease or recovery authority is authentic and its validity
    /// window has passed against a trusted bounded clock.
    ///
    /// Not `StaleAuthority`: expiry is about time. Not `Fenced`: expiry does
    /// not mean anything replaced it. An expired effect grant does not
    /// prevent read-only receipt recovery under fresh recovery authority.
    AuthorityExpired,
    /// A trusted bounded clock could not be established, or the remaining
    /// uncertainty exceeds what the caller declared it would tolerate.
    ///
    /// Not `AuthorityExpired`: this is a refusal to judge expiry at all,
    /// which fails closed rather than guessing.
    ClockUncertain,
    /// The local authority record is `Fenced`, and the requested action needs
    /// a record that serves.
    ///
    /// Not `StaleAuthority` and not `StaleIncarnation`: fencing is a decided
    /// state of the record, not a comparison against a newer authority. Fence
    /// is monotonic; a same-epoch grant never reopens it.
    Fenced,
    /// The presented authority is older than what the record durably holds:
    /// a lower owner generation, revision or fence than the installed one.
    ///
    /// Not `StaleIncarnation`, which is the case where the numbers match.
    StaleAuthority,
    /// The incarnation is in durable rejection membership, at the same owner
    /// generation and the same numeric fence as the installed one.
    ///
    /// This is the variant `StaleAuthority` cannot express, and the reason
    /// rejection membership is retained at all: a replaced incarnation
    /// carrying identical numbers is otherwise indistinguishable from the one
    /// that replaced it.
    StaleIncarnation,
    /// A compare-and-set precondition did not hold: the record moved under
    /// the caller between read and write.
    ///
    /// Also covers a proposed successor `revision` that is not the successor
    /// of the revision the precondition matched: both are the caller's belief
    /// about where the record stands, and both are wrong in the same way.
    ///
    /// Not `StaleAuthority`: the authority may be perfectly current and the
    /// caller simply lost a race. Retryable after re-reading, and the reads
    /// that recovery names are
    /// [`crate::CapabilityLocalEffectStoreV1::get_authority_state`] and
    /// [`crate::CapabilityLocalEffectStoreV1::get_rejection_high_water`]. A stated
    /// recovery whose read does not exist is worse than an unstated one.
    Conflict,
    /// The idempotency key was already spent by a request with a different
    /// canonical request digest.
    ///
    /// The same key with the same request is not an error: it returns the
    /// original committed result and performs no second effect. Not
    /// `EffectKeyReuse`, which is about the effect rather than the request.
    IdempotencyKeyReuse,
    /// The effect key was already spent by a different effect.
    ///
    /// Not `IdempotencyKeyReuse`: the request may be byte-identical and still
    /// name an effect this key already committed.
    EffectKeyReuse,
    /// The actual bytes or cost exceeded the ceiling the grant signed.
    ///
    /// Raised before the effect where the size is known in advance, and the
    /// transaction aborts where it is not. Never reported after a commit.
    BudgetExceeded,
    /// The audit record required by policy could not be written in the same
    /// commit as the effect.
    ///
    /// Not `DependencyUnavailable`: this specifically refuses to let the
    /// effect commit without its audit record. A separate audit transaction
    /// is not a fallback; it fails admission.
    AuditUnavailable,
    /// A store or component the effect needs is unreachable, and it is known
    /// that no effect was committed.
    ///
    /// Not `OutcomeUnknown`: the distinction is whether the caller may retry
    /// freely. Here it may.
    DependencyUnavailable,
    /// The result of the effect is genuinely unknown: a timeout or a lost
    /// reply where the commit may or may not have happened.
    ///
    /// The caller recovers by key and never retries the effect blindly, and
    /// never reports this as a failure before the effect. Not
    /// `UncommittedReceipt`, which is a definite negative observation.
    OutcomeUnknown,
    /// The authority record cannot be trusted until an owner-qualified
    /// reconciliation runs: a restored backup, or rejection membership that
    /// is missing or older than the state it guards.
    ///
    /// It is specifically NOT reported as an absent precondition. Absence
    /// would let a restored store start over and reactivate an incarnation
    /// that was already rejected.
    RestoreEvidenceRequired,
    /// The committed result existed and is no longer retained: the request
    /// falls outside the documented replay and recovery horizon.
    ///
    /// Not `DependencyUnavailable`, which is a store the caller cannot reach,
    /// and never a silent absence. Unavailability of retained evidence is
    /// stated, because a caller that reads absence may conclude the effect
    /// never happened.
    RetainedEvidenceUnavailable,
    /// The observer looked and found no committed row, or found one not yet
    /// visible under committed read isolation.
    ///
    /// A definite negative observation, unlike `OutcomeUnknown`. A caller
    /// that fabricates a receipt payload gets this, because the observer
    /// reads the store rather than the claim.
    UncommittedReceipt,
    /// The successor authority record does not carry the disposition the
    /// grant's action produces, per [`LocalAuthorityDispositionV1`].
    ///
    /// Not `RelationMismatch`, which compares the grant against the
    /// expectation. This compares the caller's proposed successor against the
    /// transition the action defines, and the store recomputes it rather than
    /// trusting what was handed over.
    DispositionMismatch,
    /// The grant's action is not admissible under the authority context the
    /// grant carries: a preparation context asking for Activate, Write, Fence
    /// or Release, or an installed context asking for PreparationCleanup.
    ///
    /// Not `NotAdmitted`, which is about the adapter's qualification. This is
    /// about the authority presented, and it is the refusal the two separate
    /// issuer ports exist to make unconstructible in the first place.
    ActionNotPermittedByContext,
    /// The proposed successor record's authority context is not the one the
    /// verified grant carries.
    ///
    /// Cell projects owner generation, revision, fence and incarnation and
    /// never assigns them, so a caller proposing a successor context that
    /// differs from the grant's is asking Cell to install authority the owner
    /// did not issue. There is no disposition at which that is admissible, and
    /// no reading under which it is a formatting error.
    ///
    /// Not `ActionNotPermittedByContext`, which is about the context the GRANT
    /// carries being wrong for the action; here the grant's context may be
    /// perfectly valid and the successor disagrees with it. Not
    /// `StaleAuthority` or `StaleIncarnation`, which compare a presented
    /// authority against what the record already holds; this compares the
    /// caller's proposed future record against the grant authorizing it. Not
    /// `RelationMismatch`, which is the same shape of disagreement over fields
    /// Cell may legitimately compute.
    AuthorityContextMismatch,
}
