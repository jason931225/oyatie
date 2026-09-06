use crate::{
    CapacityPurchasePreferenceV1, CapacityVectorV1, CellId, CellProofEnvelopeV1, CellProofVerifier,
    CommercialAllocationAttributionV1, CommercialPlacementBasisV1, Digest32,
    ImmutableEvidenceRefV1, PlacementOperationKey, PlacementPartitionV1, ProducerId,
    ProofConstructionError, ProofVerificationError, ReservationRefV1, ResilienceObjectiveV1,
    SignedAssuranceCompilationV1, SignedAssuranceEvidenceV1, SignedRecoveryEvidenceV1, TenantId,
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

opaque_id!(CatalogSnapshotId);
opaque_id!(PlacementCursorToken);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlacementPolicyGeneration(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementSearchCursorV1 {
    pub next_partition_ordinal: u64,
    pub next_window_ordinal: u64,
    pub next_candidate_ordinal: u64,
    pub evaluated_candidate_count: u64,
    pub opaque_token: PlacementCursorToken,
    pub exhaustion_accumulator_digest: Digest32,
    pub current_best_candidate_digest: Option<Digest32>,
    pub current_best_score_digest: Option<Digest32>,
    pub current_best: Option<Box<PlacementBestCandidateV1>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementSearchPlanV1 {
    pub snapshot: CatalogSnapshotId,
    pub ordered_partition_root_digest: Digest32,
    pub partition_count: u64,
    pub ordered_candidate_root_digest: Digest32,
    pub candidate_count: u64,
    pub maximum_candidates_per_window: u32,
    pub maximum_windows_per_step: u32,
    pub maximum_windows_per_reconciliation_lease: u32,
    pub cursor: PlacementSearchCursorV1,
    pub plan_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementIncumbentV1 {
    pub cell_id: CellId,
    pub binding_generation: u64,
    pub binding_revision: u64,
    pub binding_digest: Digest32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PlacementIntentPurposeV1 {
    InitialAdmission,
    ExplicitMigration,
    Drain,
    Recovery,
    DeploymentPolicy,
    SustainedCapacityBandCrossing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementIntentPayloadV1 {
    pub schema_version: u32,
    pub operation: PlacementOperationKey,
    pub tenant_id: TenantId,
    pub assurance_compilation: SignedAssuranceCompilationV1,
    pub home_capacity: CapacityVectorV1,
    pub capacity_purchase_preference: CapacityPurchasePreferenceV1,
    pub resilience: ResilienceObjectiveV1,
    pub incumbent: Option<PlacementIncumbentV1>,
    pub purpose: PlacementIntentPurposeV1,
    pub movement_authority_digest: Option<Digest32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedPlacementIntentV1 {
    pub payload: PlacementIntentPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DominantResourceUtilizationV1 {
    pub dimension: crate::CapacityDimensionV1,
    pub post_placement_utilization_millionths: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PlacementScoreOrderV1 {
    DominantUtilizationThenCorrelatedRiskThenMovementThenMarginalCostThenStableHash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementScoreV1 {
    pub order: PlacementScoreOrderV1,
    pub dominant_resource_utilization: DominantResourceUtilizationV1,
    pub correlated_risk_overlap_millionths: u32,
    pub correlated_risk_evidence_digest: Digest32,
    pub data_movement_bytes: u64,
    pub data_movement_cost_digest: Digest32,
    pub commercial_basis: CommercialPlacementBasisV1,
    pub raw_marginal_cost: crate::MoneyMicrounitsV1,
    pub risk_adjusted_marginal_cost: crate::MoneyMicrounitsV1,
    pub stable_tie_break_digest: Digest32,
    pub ordered_score_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementBestCandidateV1 {
    pub cell_id: CellId,
    pub partition: PlacementPartitionV1,
    pub candidate_ordinal: u64,
    pub candidate_digest: Digest32,
    pub admission_term_digest: Digest32,
    pub capacity_revision: crate::CellCapacityRevision,
    pub capacity_record_digest: Digest32,
    pub score: PlacementScoreV1,
    pub accumulator_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementAllocationV1 {
    pub cell_id: CellId,
    pub partition: PlacementPartitionV1,
    pub reservation: ReservationRefV1,
    pub eligibility_proof_digest: Digest32,
    pub failure_independence_proof_digest: Digest32,
    pub post_placement_headroom_digest: Digest32,
    pub commercial_attribution: CommercialAllocationAttributionV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementDecisionPayloadV1 {
    pub schema_version: u32,
    pub operation: PlacementOperationKey,
    pub tenant_id: TenantId,
    pub policy_generation: PlacementPolicyGeneration,
    pub assurance_generation: crate::AssuranceGeneration,
    pub assurance_compiler_version: crate::AssuranceCompilerVersion,
    pub assurance_compilation_digest: Digest32,
    pub assurance_requirements_digest: Digest32,
    pub assurance_evidence_digest: Digest32,
    pub recovery_evidence_digest: Digest32,
    pub assurance_evidence: SignedAssuranceEvidenceV1,
    pub recovery_evidence: SignedRecoveryEvidenceV1,
    pub intent_proof_digest: Digest32,
    pub search_plan_digest: Digest32,
    pub home: PlacementAllocationV1,
    pub warm_recovery: Option<PlacementAllocationV1>,
    pub score: PlacementScoreV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedPlacementDecisionV1 {
    pub payload: PlacementDecisionPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementContinuationPayloadV1 {
    pub schema_version: u32,
    pub operation: PlacementOperationKey,
    pub tenant_id: TenantId,
    pub policy_generation: PlacementPolicyGeneration,
    pub search_plan_digest: Digest32,
    pub next_cursor: PlacementSearchCursorV1,
    pub evaluated_candidate_count: u64,
    pub evaluated_windows_total: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedPlacementContinuationV1 {
    pub payload: PlacementContinuationPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementExhaustionPayloadV1 {
    pub schema_version: u32,
    pub operation: PlacementOperationKey,
    pub tenant_id: TenantId,
    pub policy_generation: PlacementPolicyGeneration,
    pub search_plan_digest: Digest32,
    pub ordered_partition_root_digest: Digest32,
    pub partition_count: u64,
    pub ordered_candidate_root_digest: Digest32,
    pub candidate_count: u64,
    pub evaluated_partition_count: u64,
    pub evaluated_candidate_count: u64,
    pub evaluated_windows_total: u64,
    pub terminal_cursor: PlacementSearchCursorV1,
    pub exhaustion_accumulator_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedPlacementExhaustionV1 {
    pub payload: PlacementExhaustionPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementIntentExpectation {
    pub operation: PlacementOperationKey,
    pub tenant_id: TenantId,
    pub assurance_generation: crate::AssuranceGeneration,
    pub assurance_compiler_version: crate::AssuranceCompilerVersion,
    pub assurance_compilation_digest: Digest32,
    pub incumbent: Option<PlacementIncumbentV1>,
    pub purpose: PlacementIntentPurposeV1,
    pub movement_authority_digest: Option<Digest32>,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementDecisionExpectation {
    pub operation: PlacementOperationKey,
    pub tenant_id: TenantId,
    pub policy_generation: PlacementPolicyGeneration,
    pub assurance_generation: crate::AssuranceGeneration,
    pub assurance_compiler_version: crate::AssuranceCompilerVersion,
    pub assurance_compilation_digest: Digest32,
    pub assurance_requirements_digest: Digest32,
    pub intent_proof_digest: Digest32,
    pub assurance_evidence_ref: ImmutableEvidenceRefV1,
    pub recovery_evidence_ref: ImmutableEvidenceRefV1,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedPlacementIntent(SignedPlacementIntentV1);

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCellPlacementDecision(SignedPlacementDecisionV1);

impl VerifiedPlacementIntent {
    #[must_use]
    pub fn signed(&self) -> &SignedPlacementIntentV1 {
        &self.0
    }
}

impl VerifiedCellPlacementDecision {
    #[must_use]
    pub fn signed(&self) -> &SignedPlacementDecisionV1 {
        &self.0
    }
}

pub fn verify_placement_intent(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedPlacementIntentV1,
    _expectation: &PlacementIntentExpectation,
) -> Result<VerifiedPlacementIntent, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

pub fn verify_placement_decision(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedPlacementDecisionV1,
    _expectation: &PlacementDecisionExpectation,
) -> Result<VerifiedCellPlacementDecision, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}
