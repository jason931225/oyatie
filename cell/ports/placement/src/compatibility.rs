use crate::{
    BoxCellFuture, CellAdmissionEpoch, CellCapacityRevision, CellDeploymentUnitId, CellId,
    CellLifecycleRevision, CellProofEnvelopeV1, CellProofVerifier, CellReadinessRingV1,
    CellSpecRevision, Digest32, ImmutableEvidenceRefV1, PlacementReadAuthorityV1, ProducerId,
    ProofConstructionError, ProofVerificationError,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProtocolVersionRangeV1 {
    pub minimum: u32,
    pub maximum: u32,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OwnerReleaseId(String);

impl OwnerReleaseId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(_value: impl Into<String>) -> Result<Self, ProofConstructionError> {
        Err(ProofConstructionError::NotImplemented)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DeploymentOrderV1 {
    ExpandReadersBeforeWriters,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DurableStateRollbackV1 {
    RollbackBeforeWriterCutover,
    ForwardRecoveryAfterWriterCutover,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ControlPlaneOwnerV1 {
    Cell,
    Tenancy,
    GatewayProjection,
    CapabilityParticipant,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnerReleaseCompatibilityPayloadV1 {
    pub schema_version: u32,
    pub owner: ControlPlaneOwnerV1,
    pub owner_id: String,
    pub release_id: OwnerReleaseId,
    pub predecessor_release_id: OwnerReleaseId,
    pub api_read_versions: ProtocolVersionRangeV1,
    pub api_write_versions: ProtocolVersionRangeV1,
    pub proof_read_versions: ProtocolVersionRangeV1,
    pub proof_write_versions: ProtocolVersionRangeV1,
    pub durable_state_read_versions: ProtocolVersionRangeV1,
    pub durable_state_write_version: u32,
    pub minimum_forward_recovery_version: u32,
    pub deployment_order: DeploymentOrderV1,
    pub rollback: DurableStateRollbackV1,
    pub compatibility_matrix_digest: Digest32,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedOwnerReleaseCompatibilityV1 {
    pub payload: OwnerReleaseCompatibilityPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedOwnerReleaseCompatibility(SignedOwnerReleaseCompatibilityV1);

impl VerifiedOwnerReleaseCompatibility {
    #[must_use]
    pub fn signed(&self) -> &SignedOwnerReleaseCompatibilityV1 {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseCompatibilitySetV1 {
    pub immutable_set: ImmutableEvidenceRefV1,
    pub ordered_release_root_digest: Digest32,
    pub release_count: u64,
    pub set_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseCompatibilityMemberV1 {
    pub ordinal: u64,
    pub release: SignedOwnerReleaseCompatibilityV1,
    pub inclusion_path: Vec<Digest32>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedReleaseCompatibilityMember(ReleaseCompatibilityMemberV1);

impl VerifiedReleaseCompatibilityMember {
    #[must_use]
    pub fn member(&self) -> &ReleaseCompatibilityMemberV1 {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PromotionPrerequisiteStatusV1 {
    NotSatisfied,
    Satisfied,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionalRolloutCompatibilityEvidenceV1 {
    pub source_record: ImmutableEvidenceRefV1,
    pub contracts_signed: PromotionPrerequisiteStatusV1,
    pub controls_validated: PromotionPrerequisiteStatusV1,
    pub compliance_evidence_accepted: PromotionPrerequisiteStatusV1,
    pub capacity_reserved: PromotionPrerequisiteStatusV1,
    pub compatibility_record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellRevisionIdentityV1 {
    pub cell_id: CellId,
    pub cell_identity_digest: Digest32,
    pub deployment_unit_id: CellDeploymentUnitId,
    pub spec_revision: CellSpecRevision,
    pub spec_digest: Digest32,
    pub resource_digest: Digest32,
    pub lifecycle_revision: CellLifecycleRevision,
    pub admission_epoch: CellAdmissionEpoch,
    pub capacity_revision: CellCapacityRevision,
    pub capacity_record_digest: Digest32,
    pub topology_inventory_digest: Digest32,
    pub resilience_objective_digest: Digest32,
    pub capability_inventory_digest: Digest32,
    pub revision_identity_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwoCellIsolationEvidenceV1 {
    pub subject: CellRevisionIdentityV1,
    pub counterpart: CellRevisionIdentityV1,
    pub evidence: ImmutableEvidenceRefV1,
    pub observation_window_start_unix_seconds: u64,
    pub observation_window_end_unix_seconds: u64,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionMetricPopulationV1 {
    pub population_name: String,
    pub denominator: u64,
    pub population_root_digest: Digest32,
    pub population_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellPromotionMetricsV1 {
    pub slo_snapshot: ImmutableEvidenceRefV1,
    pub two_cell_isolation_evidence: TwoCellIsolationEvidenceV1,
    pub failure_injection_evidence: ImmutableEvidenceRefV1,
    pub version_skew_evidence: ImmutableEvidenceRefV1,
    pub progressive_delivery_evidence: ImmutableEvidenceRefV1,
    pub rollback_rehearsal_evidence: ImmutableEvidenceRefV1,
    pub local_control_plane_resilience_evidence: ImmutableEvidenceRefV1,
    pub backup_restore_evidence: ImmutableEvidenceRefV1,
    pub noisy_tenant_backpressure_evidence: ImmutableEvidenceRefV1,
    pub bad_deployment_containment_evidence: ImmutableEvidenceRefV1,
    pub observation_window_start_unix_seconds: u64,
    pub observation_window_end_unix_seconds: u64,
    pub tenant_population: PromotionMetricPopulationV1,
    pub reservation_population: PromotionMetricPopulationV1,
    pub movement_population: PromotionMetricPopulationV1,
    /// The reproducible, closure-backed promotion economics record.
    ///
    /// This replaces the previous `economics_evidence` reference plus the
    /// `unit_cost_microunits` / `currency` scalar pair. A scalar with an opaque
    /// reference cannot be replayed: its cost scope was unnamed, so excluding
    /// reserve, idle, network or control cost lowered it without changing any
    /// declared field, and its denominator was not stated at all.
    ///
    /// `economics.policy` governs economics ONLY. It is not a second authority
    /// for the general metric calculation policy below, which continues to
    /// govern every other metric and population on this record.
    pub economics: crate::CellPromotionEconomicsV1,
    pub slo_threshold_policy: ImmutableEvidenceRefV1,
    pub calculation_policy_version: String,
    pub calculation_policy_digest: Digest32,
    pub regional_rollout_compatibility: RegionalRolloutCompatibilityEvidenceV1,
    pub stranded_headroom_basis_points: u32,
    pub recovery_reserve_headroom_basis_points: u32,
    pub reservation_leakage_count: u64,
    pub oldest_reconciliation_age_seconds: u64,
    pub maximum_projection_age_seconds: u64,
    pub cross_cell_traffic_bytes: u64,
    pub movement_churn_basis_points: u32,
    pub bytes_moved: u64,
    pub correlated_risk_overlap_millionths: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellPromotionEvidencePayloadV1 {
    pub schema_version: u32,
    pub cell: CellRevisionIdentityV1,
    pub target_readiness: CellReadinessRingV1,
    pub current_release_set: ReleaseCompatibilitySetV1,
    pub target_release_set: ReleaseCompatibilitySetV1,
    pub metrics: CellPromotionMetricsV1,
    pub required_release_owner_root_digest: Digest32,
    pub required_release_owner_count: u64,
    pub current_release_coverage_proof_digest: Digest32,
    pub target_release_coverage_proof_digest: Digest32,
    pub policy_generation: u64,
    pub observed_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedCellPromotionEvidenceV1 {
    pub payload: CellPromotionEvidencePayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCellPromotionEvidence(SignedCellPromotionEvidenceV1);

impl VerifiedCellPromotionEvidence {
    #[must_use]
    pub fn signed(&self) -> &SignedCellPromotionEvidenceV1 {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellPromotionEvidenceExpectationV1 {
    pub cell: CellRevisionIdentityV1,
    pub target_readiness: CellReadinessRingV1,
    pub required_release_owner_root_digest: Digest32,
    pub required_release_owner_count: u64,
    pub expected_policy_generation: u64,
    /// The approved economics policy and the cell-issued source closure this
    /// promotion must have been priced under.
    ///
    /// Both are supplied by the calling policy loader from trusted finalized
    /// cell accounting inputs and from
    /// [`crate::PromotionEconomicsClosureAuthority`]. Neither is ever copied
    /// out of the submitted promotion proof: a promoter that supplies its own
    /// expectation has proved nothing, and a promoter that assembles a smaller
    /// self-signed closure must fail against an independently selected one.
    pub economics_policy_digest: Digest32,
    pub economics_closure_digest: Digest32,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
}

pub fn verify_owner_release_compatibility(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedOwnerReleaseCompatibilityV1,
    _expected_predecessor: &OwnerReleaseId,
    _expected_producer: &ProducerId,
    _expected_audience: &ProducerId,
    _now_unix_seconds: u64,
) -> Result<VerifiedOwnerReleaseCompatibility, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

/// Verifies a signed cell promotion proof.
///
/// The `economics` argument is the private-field proof that a complete bounded
/// replay of the promotion economics record actually finished. It is required,
/// not optional: without it this function would once again be trusting a
/// number carried inside the payload it is verifying. The gate checks that
/// `economics.evidence()` equals `signed.payload.metrics.economics` exactly,
/// and that its cell revision, window, policy digest and closure digest match
/// the expectation, whose economics digests came from trusted state rather
/// than from the proof.
///
/// A missing or stale replay proof therefore cannot produce a
/// [`VerifiedCellPromotionEvidence`], and verified economics for one proof
/// cannot be substituted into another. That much this gate does perform: the
/// digest comparisons above are this crate's own arithmetic and no argument
/// steers them.
///
/// WHAT THE GATE CANNOT ESTABLISH IS A DEPLOYMENT OBLIGATION. The strength of
/// "the replay actually finished" is the strength of
/// [`crate::VerifiedCellPromotionEconomics`], and that wrapper is minted behind
/// a `&dyn` [`CellProofVerifier`] seam, as is this function's own signature
/// check. One out-of-crate component implementing that verifier can hand itself
/// in as the judge of authenticity for both, and present a replay this gate
/// will then find internally consistent. The private fields refuse direct
/// construction and nothing else; see [`crate::MovementActionResultAuthority`].
///
/// Economics replay reports its own
/// precise refusals through
/// [`crate::PromotionEconomicsVerificationErrorV1`] before this gate is
/// reached; here a mismatch is `RelationMismatch` and the existing typed
/// errors continue to cover unsupported and expired proofs.
///
/// Verified economics is not itself a mutation authority and not a
/// transferable standalone promotion permit. The control service still
/// rechecks the current revision, readiness and policy on its own.
pub fn verify_cell_promotion_evidence(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedCellPromotionEvidenceV1,
    _economics: &crate::VerifiedCellPromotionEconomics,
    _expectation: &CellPromotionEvidenceExpectationV1,
) -> Result<VerifiedCellPromotionEvidence, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

pub fn verify_release_compatibility_member(
    _set: &ReleaseCompatibilitySetV1,
    _member: ReleaseCompatibilityMemberV1,
    _maximum_inclusion_path_depth: u32,
) -> Result<VerifiedReleaseCompatibilityMember, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

pub trait ReleaseCompatibilityReader: Send + Sync {
    /// Reads one member of a release compatibility set by ordinal.
    ///
    /// `None` MEANS THE READER LOOKED AND FOUND NO MEMBER at that ordinal —
    /// an ordinal past the end of the set. Absence is representable here
    /// because it is an outcome a legitimate caller reaches, and because the
    /// alternative was
    /// [`crate::PlacementContractError::NotFoundOrNotAuthorized`], which
    /// conflates absence with an authorization refusal on purpose. That
    /// conflation is right at an unauthenticated edge and wrong here: this
    /// method already runs under an explicit
    /// [`PlacementReadAuthorityV1`], so for it the conflation destroys exactly
    /// the distinction it is there to draw.
    fn read_member<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        set: &'a ReleaseCompatibilitySetV1,
        ordinal: u64,
    ) -> BoxCellFuture<
        'a,
        Result<Option<ReleaseCompatibilityMemberV1>, crate::PlacementContractError>,
    >;
}
