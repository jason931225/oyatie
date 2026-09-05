use crate::{BindingDigest32, BindingStoreError, BoxTenancyFuture};

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
    pub target_partition: crate::CellBindingIndexPartitionKey,
    pub expected_projection_revision: u64,
    pub expected_projection_digest: BindingDigest32,
    pub source_contributions: Vec<crate::BindingControlContributionProjectionV1>,
    pub maximum_contribution_count: u32,
    pub next_snapshot: crate::CellBindingIndexSnapshotV1,
    pub audit_outbox: crate::BindingAuditRecordV1,
}

impl BindingCellIndexProjectionWriteSetV1 {
    pub fn assemble(
        _parts: BindingCellIndexProjectionWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &BindingCellIndexProjectionWriteSetPartsV1 {
        &self.parts
    }
}

pub trait TenantBindingCellIndexProjectionStore: Send + Sync {
    fn apply_contributions<'a>(
        &'a self,
        write_set: &'a BindingCellIndexProjectionWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<crate::CellBindingIndexSnapshotV1, BindingStoreError>>;
}
