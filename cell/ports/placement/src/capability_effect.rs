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
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalAuthorityFenceV1(pub u64);

/// The capability store's own durable record revision.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalCommitRevisionV1(pub u64);

/// A projection of the independently generated Tenancy serving incarnation.
///
/// The bytes are opaque to Cell: equality and rejection membership are the
/// only operations. Comparing a numeric epoch alone cannot distinguish a
/// replaced incarnation from the one it replaced, which is why this is not
/// folded into [`LocalAuthorityFenceV1`].
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
        rejection_high_water: CapabilityAuthorityRejectionHighWaterV1,
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
    /// Its `disposition` must be the one the grant's action produces, per
    /// [`LocalAuthorityDispositionV1`]. The store recomputes that rather than
    /// trusting the caller.
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityEffectErrorV1 {
    NotImplemented,
    NotAdmitted,
    UnsupportedSink,
    VerificationFailed,
    RelationMismatch,
    AuthorityExpired,
    ClockUncertain,
    Fenced,
    StaleAuthority,
    Conflict,
    IdempotencyKeyReuse,
    EffectKeyReuse,
    BudgetExceeded,
    AuditUnavailable,
    DependencyUnavailable,
    OutcomeUnknown,
    RestoreEvidenceRequired,
    StaleIncarnation,
    RetainedEvidenceUnavailable,
    UncommittedReceipt,
}
