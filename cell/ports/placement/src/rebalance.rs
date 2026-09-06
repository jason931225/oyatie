use crate::{
    BoxCellFuture, CellControlAuditRecordV1, CellControlIdempotencyRecordV1,
    CellControlOperationPreconditionV1, CellControlOperationV1, CellControlPersistenceAuthorityV1,
    CellControlReadAuthorityV1, CellId, Digest32, ImmutableEvidenceRefV1, PlacementContractError,
    PlacementPartitionV1, PlacementPolicyGeneration, ProofConstructionError,
    RebalanceBudgetAccountingV1, RebalanceBudgetLimitsV1,
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RebalanceJobId(String);

impl RebalanceJobId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(_value: impl Into<String>) -> Result<Self, ProofConstructionError> {
        Err(ProofConstructionError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RebalanceTriggerV1 {
    Drain {
        source_cell_id: CellId,
        drain_term: crate::DrainTermV1,
    },
    Recovery {
        source_cell_id: CellId,
        failure_evidence: ImmutableEvidenceRefV1,
    },
    DeploymentPolicy {
        release_evidence: ImmutableEvidenceRefV1,
    },
    SustainedCapacityBandCrossing {
        source_cell_id: CellId,
        observation_window_start_unix_seconds: u64,
        observation_window_end_unix_seconds: u64,
        telemetry_evidence: ImmutableEvidenceRefV1,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalancePolicyV1 {
    pub placement_policy_generation: PlacementPolicyGeneration,
    pub budget_limits: RebalanceBudgetLimitsV1,
    pub maximum_concurrent_movements: u32,
    pub maximum_candidates_per_page: u32,
    pub hysteresis_basis_points: u32,
    pub minimum_stable_observation_seconds: u64,
    pub cooldown_seconds: u64,
    pub policy_digest: Digest32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RebalanceJobStateV1 {
    Pending,
    Selecting,
    Moving,
    Succeeded,
    Failed,
    Cancelled,
    CancellationRequested,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebalanceJobV1 {
    pub source_partition: crate::RebalanceJobPartitionV1,
    pub job_id: RebalanceJobId,
    pub partition: PlacementPartitionV1,
    pub trigger: RebalanceTriggerV1,
    pub policy: RebalancePolicyV1,
    pub snapshot: crate::RebalanceCandidateSnapshotV1,
    pub candidate_cursor: crate::RebalanceCandidateCursorV1,
    pub budget_accounting: RebalanceBudgetAccountingV1,
    pub selected_movement_root_digest: Digest32,
    pub selected_movement_count: u64,
    pub completed_movement_root_digest: Digest32,
    pub completed_movement_count: u64,
    pub state: RebalanceJobStateV1,
    pub revision: u64,
    pub durable_checkpoint_digest: Digest32,
    pub created_at_unix_seconds: u64,
    pub updated_at_unix_seconds: u64,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RebalanceJobPreconditionV1 {
    Absent,
    Matches {
        revision: u64,
        record_digest: Digest32,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct RebalanceJobWriteSetV1 {
    parts: RebalanceJobWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RebalanceJobWriteSetPartsV1 {
    pub source_partition: crate::RebalanceJobPartitionV1,
    pub authority: CellControlPersistenceAuthorityV1,
    pub precondition: RebalanceJobPreconditionV1,
    pub job: RebalanceJobV1,
    pub drain_mutations: crate::DrainContributorMutationSetV1,
    pub operation_precondition: CellControlOperationPreconditionV1,
    pub operation: CellControlOperationV1,
    pub idempotency: CellControlIdempotencyRecordV1,
    pub proof_consumptions: Vec<crate::CellProofConsumptionV1>,
    pub audit_outbox: CellControlAuditRecordV1,
}

impl RebalanceJobWriteSetV1 {
    pub fn assemble(_parts: RebalanceJobWriteSetPartsV1) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &RebalanceJobWriteSetPartsV1 {
        &self.parts
    }
}

pub trait RebalanceJobStore: Send + Sync {
    fn apply<'a>(
        &'a self,
        write_set: &'a RebalanceJobWriteSetV1,
    ) -> BoxCellFuture<'a, Result<CellControlOperationV1, PlacementContractError>>;

    fn get<'a>(
        &'a self,
        authority: &'a CellControlReadAuthorityV1,
        partition: &'a crate::RebalanceJobPartitionV1,
        job_id: &'a RebalanceJobId,
    ) -> BoxCellFuture<'a, Result<Option<RebalanceJobV1>, PlacementContractError>>;
}
