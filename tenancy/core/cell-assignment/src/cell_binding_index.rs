use cell_placement::{CellId, ImmutableEvidenceRefV1};

use crate::{
    BindingDigest32, BindingGeneration, BindingReconciliationReadAuthorityV1, BindingRevision,
    BindingStoreError, BoxTenancyFuture, TenantId,
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CellBindingIndexPartitionKey(String);

impl CellBindingIndexPartitionKey {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(_value: impl Into<String>) -> Result<Self, crate::BindingProofConstructionError> {
        Err(crate::BindingProofConstructionError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellBindingIndexPageTokenV1(Vec<u8>);

impl CellBindingIndexPageTokenV1 {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn parse(_value: Vec<u8>) -> Result<Self, crate::BindingContractError> {
        Err(crate::BindingContractError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellBindingIndexSnapshotRequestV1 {
    pub partition: CellBindingIndexPartitionKey,
    pub cell_id: CellId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellBindingIndexSnapshotV1 {
    pub control_contribution_coverage: crate::BindingControlContributionCoverageV1,
    pub projection_revision: u64,
    pub projection_record_digest: BindingDigest32,
    pub partition: CellBindingIndexPartitionKey,
    pub cell_id: CellId,
    pub snapshot: ImmutableEvidenceRefV1,
    pub ordered_binding_root_digest: BindingDigest32,
    pub binding_count: u64,
    pub snapshot_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellBindingIndexPageRequestV1 {
    pub snapshot: CellBindingIndexSnapshotV1,
    pub start_ordinal: u64,
    pub page_size: u32,
    pub page_token: Option<CellBindingIndexPageTokenV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellBindingIndexEntryV1 {
    pub ordinal: u64,
    pub tenant_id: TenantId,
    pub generation: BindingGeneration,
    pub revision: BindingRevision,
    pub binding_record_digest: BindingDigest32,
    pub entry_digest: BindingDigest32,
    pub inclusion_path: Vec<BindingDigest32>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CellBindingIndexMutationKindV1 {
    AddHome,
    AddWarmRecovery,
    RemoveHome,
    RemoveWarmRecovery,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CellBindingIndexEntryPreconditionV1 {
    Absent,
    Matches {
        generation: BindingGeneration,
        revision: BindingRevision,
        binding_record_digest: BindingDigest32,
        index_record_digest: BindingDigest32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellBindingIndexEntryMutationV1 {
    pub control_partition: crate::TenantControlPartitionRefV1,
    pub partition: CellBindingIndexPartitionKey,
    pub cell_id: CellId,
    pub tenant_id: TenantId,
    pub precondition: CellBindingIndexEntryPreconditionV1,
    pub generation: BindingGeneration,
    pub revision: BindingRevision,
    pub binding_record_digest: BindingDigest32,
    pub mutation_kind: CellBindingIndexMutationKindV1,
    pub mutation_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CellBindingIndexMutationSetV1 {
    parts: CellBindingIndexMutationSetPartsV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellBindingIndexMutationSetPartsV1 {
    pub mutations: Vec<CellBindingIndexEntryMutationV1>,
    pub ordered_mutation_root_digest: BindingDigest32,
    pub mutation_count: u64,
    pub set_digest: BindingDigest32,
}

impl CellBindingIndexMutationSetV1 {
    pub fn assemble(
        _parts: CellBindingIndexMutationSetPartsV1,
        _maximum_mutation_count: u32,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &CellBindingIndexMutationSetPartsV1 {
        &self.parts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellBindingIndexPageV1 {
    pub snapshot: CellBindingIndexSnapshotV1,
    pub entries: Vec<CellBindingIndexEntryV1>,
    pub next_ordinal: u64,
    pub next_page_token: Option<CellBindingIndexPageTokenV1>,
    pub page_digest: BindingDigest32,
}

pub trait TenantBindingCellIndexStore: Send + Sync {
    fn acquire_snapshot<'a>(
        &'a self,
        authority: &'a BindingReconciliationReadAuthorityV1,
        request: &'a CellBindingIndexSnapshotRequestV1,
    ) -> BoxTenancyFuture<'a, Result<CellBindingIndexSnapshotV1, BindingStoreError>>;

    fn read_page<'a>(
        &'a self,
        authority: &'a BindingReconciliationReadAuthorityV1,
        request: &'a CellBindingIndexPageRequestV1,
    ) -> BoxTenancyFuture<'a, Result<CellBindingIndexPageV1, BindingStoreError>>;
}
