use crate::{BindingDigest32, TenantControlPartitionRefV1};

#[derive(Debug, Eq, PartialEq)]
pub struct BindingControlPartitionMutationV1 {
    parts: BindingControlPartitionMutationPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingControlPartitionMutationPartsV1 {
    pub binding_partition: TenantControlPartitionRefV1,
    pub cell_index_partition: TenantControlPartitionRefV1,
    pub drain_contributor_partition: TenantControlPartitionRefV1,
    pub cell_index_mutations: crate::CellBindingIndexMutationSetV1,
    pub drain_mutations: cell_placement::DrainContributorMutationSetV1,
    pub contribution_outbox: crate::BindingControlContributionOutboxV1,
    pub contribution_limits: crate::BindingControlContributionLimitsV1,
}

impl BindingControlPartitionMutationV1 {
    pub fn assemble(
        _parts: BindingControlPartitionMutationPartsV1,
    ) -> Result<Self, crate::BindingStoreError> {
        Err(crate::BindingStoreError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &BindingControlPartitionMutationPartsV1 {
        &self.parts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionProjectionV1 {
    pub payload: crate::BindingControlContributionPayloadV1,
    pub source_partition: TenantControlPartitionRefV1,
    pub binding_generation: crate::BindingGeneration,
    pub binding_revision: crate::BindingRevision,
    pub binding_record_digest: BindingDigest32,
    pub cell_index_mutation_set_digest: BindingDigest32,
    pub drain_mutation_set_digest: BindingDigest32,
    pub projection_digest: BindingDigest32,
}
