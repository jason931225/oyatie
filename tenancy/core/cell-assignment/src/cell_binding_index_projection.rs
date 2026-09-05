use crate::{BindingControlContributionError, BindingDigest32, BoxTenancyFuture};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionCoverageV1 {
    pub source_partition_root_digest: BindingDigest32,
    pub source_partition_count: u64,
    pub contribution_checkpoint_root_digest: BindingDigest32,
    pub source_manifest: cell_placement::ImmutableEvidenceRefV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingCellIndexProjectionWriteSetV1 {
    parts: BindingCellIndexProjectionWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingCellIndexProjectionWriteSetPartsV1 {
    pub authority: crate::BindingReconciliationPersistenceAuthorityV1,
    pub target: crate::BindingControlContributionTargetV1,
    pub expected_projection_revision: u64,
    pub expected_projection_digest: BindingDigest32,
    pub source_contributions: Vec<crate::VerifiedBindingControlContributionHandoff>,
    pub expected_source_checkpoints: Vec<crate::BindingControlContributionCheckpointV1>,
    pub next_source_checkpoints: Vec<crate::BindingControlContributionCheckpointV1>,
    pub application_intents: Vec<crate::BindingControlContributionApplicationIntentV1>,
    pub limits: crate::BindingControlContributionLimitsV1,
    pub proof_consumptions: Vec<crate::BindingProofConsumptionV1>,
    pub next_snapshot: crate::CellBindingIndexSnapshotV1,
    pub audit_outbox: crate::BindingAuditRecordV1,
}

impl BindingCellIndexProjectionWriteSetV1 {
    pub fn assemble(
        _parts: BindingCellIndexProjectionWriteSetPartsV1,
    ) -> Result<Self, BindingControlContributionError> {
        Err(BindingControlContributionError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &BindingCellIndexProjectionWriteSetPartsV1 {
        &self.parts
    }
}

pub trait TenantBindingCellIndexProjectionStore: Send + Sync {
    fn get_acknowledgment<'a>(
        &'a self,
        authority: &'a crate::BindingReconciliationReadAuthorityV1,
        query: &'a crate::BindingControlContributionAcknowledgmentQueryV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<
            crate::SignedBindingControlContributionAcknowledgmentV1,
            BindingControlContributionError,
        >,
    >;

    fn apply_contributions<'a>(
        &'a self,
        write_set: &'a BindingCellIndexProjectionWriteSetV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::BindingCellIndexProjectionResultV1, BindingControlContributionError>,
    >;
}
