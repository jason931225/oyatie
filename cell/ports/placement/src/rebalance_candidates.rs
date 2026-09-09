use crate::{
    BoxCellFuture, CapacityPurchasePreferenceV1, CapacityVectorV1,
    CellControlReconciliationReadAuthorityV1, CellId, Digest32, EvidenceSetCommitmentV1,
    ImmutableEvidenceRefV1, MovementBudgetV1, PlacementContractError, PlacementIncumbentV1,
    PlacementPartitionV1, PlacementPolicyGeneration, PlacementReadAuthorityV1,
    ResilienceObjectiveV1, SignedAssuranceCompilationV1, TenantId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RebalanceCandidateScopeV1 {
    Cell(CellId),
    Partition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceCandidateSnapshotRequestV1 {
    pub partition: PlacementPartitionV1,
    pub scope: RebalanceCandidateScopeV1,
    pub trigger_digest: Digest32,
    pub placement_policy_generation: PlacementPolicyGeneration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceCandidateSnapshotV1 {
    pub request: RebalanceCandidateSnapshotRequestV1,
    pub candidates: EvidenceSetCommitmentV1,
    pub snapshot_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceCandidatePageTokenV1(Vec<u8>);

impl RebalanceCandidatePageTokenV1 {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn parse(_value: Vec<u8>) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceCandidateCursorV1 {
    pub snapshot: RebalanceCandidateSnapshotV1,
    pub next_candidate_ordinal: u64,
    pub next_page_token: Option<RebalanceCandidatePageTokenV1>,
    pub evaluated_candidate_count: u64,
    pub selection_accumulator_root_digest: Digest32,
    pub selected_candidate_count: u64,
    pub cursor_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceCandidatePageRequestV1 {
    pub snapshot: RebalanceCandidateSnapshotV1,
    pub start_ordinal: u64,
    pub page_size: u32,
    pub page_token: Option<RebalanceCandidatePageTokenV1>,
    pub maximum_encoded_bytes: u64,
    pub maximum_inclusion_path_depth: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceCandidateV1 {
    pub ordinal: u64,
    pub tenant_id: TenantId,
    pub source_cell_id: CellId,
    pub binding_generation: u64,
    pub binding_revision: u64,
    pub binding_record_digest: Digest32,
    pub movement_requirements: ImmutableEvidenceRefV1,
    pub candidate_digest: Digest32,
    pub inclusion_path: Vec<Digest32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceCandidatePageV1 {
    pub snapshot: RebalanceCandidateSnapshotV1,
    pub start_ordinal: u64,
    pub candidates: Vec<RebalanceCandidateV1>,
    pub next_ordinal: u64,
    pub next_page_token: Option<RebalanceCandidatePageTokenV1>,
    pub page_digest: Digest32,
}

pub trait RebalanceCandidatePageReader: Send + Sync {
    fn acquire_snapshot<'a>(
        &'a self,
        authority: &'a CellControlReconciliationReadAuthorityV1,
        request: &'a RebalanceCandidateSnapshotRequestV1,
    ) -> BoxCellFuture<'a, Result<RebalanceCandidateSnapshotV1, PlacementContractError>>;

    fn read_page<'a>(
        &'a self,
        authority: &'a CellControlReconciliationReadAuthorityV1,
        request: &'a RebalanceCandidatePageRequestV1,
    ) -> BoxCellFuture<'a, Result<RebalanceCandidatePageV1, PlacementContractError>>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceCandidateRequirementsV1 {
    pub tenant_id: TenantId,
    pub incumbent: PlacementIncumbentV1,
    pub assurance_compilation: SignedAssuranceCompilationV1,
    pub home_capacity: CapacityVectorV1,
    pub capacity_purchase_preference: CapacityPurchasePreferenceV1,
    pub resilience: ResilienceObjectiveV1,
    pub ordinary_movement_ceiling: MovementBudgetV1,
    pub forward_completion_ceiling: MovementBudgetV1,
    pub requirements_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceCandidateRequirementsRequestV1 {
    pub action: crate::PlacementBusinessActionKeyV1,
    pub snapshot: RebalanceCandidateSnapshotV1,
    pub candidate: RebalanceCandidateV1,
    pub maximum_encoded_bytes: u64,
    pub maximum_inclusion_path_depth: u32,
}

pub trait RebalanceCandidateRequirementsReader: Send + Sync {
    /// Reads the requirements for one rebalance candidate.
    ///
    /// `None` MEANS THE READER LOOKED AND FOUND NO REQUIREMENTS for that
    /// candidate in that snapshot. It is an outcome, not a failure, and it is
    /// deliberately not [`PlacementContractError::NotFoundOrNotAuthorized`]:
    /// that variant conflates absence with an authorization refusal, which is
    /// right at an unauthenticated edge and wrong for a method that already
    /// runs under an explicit [`PlacementReadAuthorityV1`].
    fn read_requirements<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        request: &'a RebalanceCandidateRequirementsRequestV1,
    ) -> BoxCellFuture<'a, Result<Option<RebalanceCandidateRequirementsV1>, PlacementContractError>>;
}
